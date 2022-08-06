use std::collections::{BTreeMap, BTreeSet};

use bevy::{
    prelude::{
        debug, default, info, Deref, DerefMut, Entity, Handle, Mesh, Query, Res, ResMut, With,
    },
    render::{
        renderer::{RenderDevice, RenderQueue},
        view::{ExtractedView, VisibleEntities},
    },
    utils::FloatOrd,
};

use crate::instancing::{
    instance_slice::{InstanceSlice, InstanceSliceRange},
    material::{
        material_instanced::MaterialInstanced,
        plugin::{
            GpuAlphaMode, GpuInstances, InstanceBatch, InstanceBatchKey, InstanceMeta,
            InstancedMaterialBatchKey, RenderMaterials, RenderMeshes,
        },
        systems::prepare_mesh_batches::MeshBatch,
    },
    render::instance::Instance,
};

use super::prepare_mesh_batches::MeshBatches;

#[derive(Deref, DerefMut)]
pub struct ViewInstanceData<M: MaterialInstanced> {
    pub instance_data: BTreeMap<Entity, BTreeMap<InstanceBatchKey<M>, GpuInstances<M>>>,
}

impl<M: MaterialInstanced> Default for ViewInstanceData<M> {
    fn default() -> Self {
        Self {
            instance_data: default(),
        }
    }
}

pub fn system<M: MaterialInstanced>(
    render_device: Res<RenderDevice>,
    render_queue: Res<RenderQueue>,
    render_meshes: Res<RenderMeshes>,
    render_materials: Res<RenderMaterials<M>>,
    mesh_batches: Res<MeshBatches>,
    mut view_instance_data: ResMut<ViewInstanceData<M>>,
    mut query_views: Query<(Entity, &ExtractedView, &mut InstanceMeta<M>), With<VisibleEntities>>,
    query_instance: Query<(
        Entity,
        &Handle<M>,
        &Handle<Mesh>,
        &<M::Instance as Instance>::ExtractedInstance,
    )>,
    query_instance_slice: Query<(Entity, &Handle<M>, &Handle<Mesh>, &InstanceSlice)>,
) {
    debug!("{}", std::any::type_name::<M>());

    let render_meshes = &render_meshes.instanced_meshes;

    for (view_entity, view, mut instance_meta) in query_views.iter_mut() {
        debug!("View {view_entity:?}");

        // Fetch view rangefinder for sorting
        let rangefinder = view.rangefinder3d();

        let span = bevy::prelude::info_span!("Batch instances by key");
        let mut keyed_instances = span.in_scope(|| {
            // Batch instances by key
            let mut keyed_instances = BTreeMap::<
                InstanceBatchKey<M>,
                Vec<(
                    (&Handle<Mesh>, FloatOrd),
                    (
                        Entity,
                        &Handle<M>,
                        &<M::Instance as Instance>::ExtractedInstance,
                    ),
                )>,
            >::new();

            for (entity, material_handle, mesh_handle, instance) in instance_meta
                .instances
                .iter()
                .flat_map(|entity| query_instance.get(*entity))
            {
                debug!("Instance {entity:?}");

                let mesh = if let Some(mesh) = render_meshes.get(mesh_handle) {
                    mesh
                } else {
                    continue;
                };

                debug!("Mesh valid");

                let mesh_key = mesh.key.clone();

                let material = if let Some(material) = render_materials.get(material_handle) {
                    material
                } else {
                    continue;
                };

                debug!("Material valid");

                let alpha_mode = GpuAlphaMode::from(material.properties.alpha_mode);
                let material_key = InstancedMaterialBatchKey {
                    alpha_mode,
                    key: material.batch_key.clone(),
                };

                let mesh_z = rangefinder.distance(&<M::Instance as Instance>::transform(instance))
                    + material.properties.depth_bias;

                let dist = mesh_z
                    * if alpha_mode == GpuAlphaMode::Blend {
                        // Back-to-front ordering
                        1.0
                    } else {
                        // Front-to-back ordering
                        -1.0
                    };

                let key = InstanceBatchKey {
                    mesh_key,
                    material_key,
                };

                keyed_instances.entry(key).or_default().push((
                    (mesh_handle, FloatOrd(dist)),
                    (entity, material_handle, instance),
                ));
            }

            keyed_instances
        });

        if keyed_instances.is_empty() {
            continue;
        }

        for instances in keyed_instances.values_mut() {
            instances.sort_unstable_by(|(lhs_key, _), (rhs_key, _)| lhs_key.cmp(rhs_key))
        }

        debug!("Keyed instances: {:#?}", keyed_instances.values());

        let span = bevy::prelude::info_span!("Batch instance slices by key");
        let keyed_instance_slices = span.in_scope(|| {
            // Batch instance slices by key
            let mut keyed_instance_slices =
                BTreeMap::<InstanceBatchKey<M>, Vec<(Entity, &Handle<M>, &InstanceSlice)>>::new();

            for (entity, material_handle, mesh_handle, instance_slice) in instance_meta
                .instance_slices
                .iter()
                .flat_map(|entity| query_instance_slice.get(*entity))
            {
                debug!("Instance slice {entity:?}");
                let mesh = if let Some(mesh) = render_meshes.get(mesh_handle) {
                    mesh
                } else {
                    continue;
                };

                let mesh_key = mesh.key.clone();

                let material = if let Some(material) = render_materials.get(material_handle) {
                    material
                } else {
                    continue;
                };

                let alpha_mode = GpuAlphaMode::from(material.properties.alpha_mode);
                let material_key = InstancedMaterialBatchKey {
                    alpha_mode,
                    key: material.batch_key.clone(),
                };

                let key = InstanceBatchKey {
                    mesh_key,
                    material_key,
                };

                keyed_instance_slices.entry(key).or_default().push((
                    entity,
                    material_handle,
                    instance_slice,
                ));
            }

            keyed_instance_slices
        });

        debug!(
            "Keyed instance slices: {:#?}",
            keyed_instance_slices.values()
        );

        // Create instance buffer data
        let gpu_instances =
            || GpuInstances::new(render_device.get_supported_read_only_binding_type(1));

        let mut instance_buffer_data =
            BTreeMap::<InstanceBatchKey<M>, Vec<<M::Instance as Instance>::PreparedInstance>>::new(
            );

        let span = bevy::prelude::info_span!("Populate instances");
        span.in_scope(|| {
            debug!("Populating instances");
            // Populate instances
            for (key, instances) in keyed_instances.iter() {
                debug!("{key:#?}");
                // Collect instance data
                let data = instances
                    .iter()
                    .map(|((mesh_handle, _), (_, _, instance))| {
                        let MeshBatch { meshes, .. } = mesh_batches.get(&key.mesh_key).unwrap();

                        <M::Instance as Instance>::prepare_instance(
                            instance,
                            meshes.iter().position(|mesh| mesh == *mesh_handle).unwrap() as u32,
                        )
                    })
                    .collect::<Vec<_>>();

                instance_buffer_data
                    .entry(key.clone())
                    .or_default()
                    .extend(data);
            }
        });

        let span = bevy::prelude::info_span!("Create instance slice ranges");
        let mut keyed_instance_slice_ranges = span.in_scope(|| {
            debug!("Creating instance slice ranges");
            // Create instance slice ranges
            keyed_instance_slices
                .iter()
                .map(|(key, instance_slices)| {
                    let instance_buffer_data_len =
                        instance_buffer_data.entry(key.clone()).or_default().len();

                    // Collect CPU instance slice data
                    let mut offset = instance_buffer_data_len;
                    let mut instance_slice_ranges = BTreeMap::<Entity, InstanceSliceRange>::new();
                    for (entity, _, instance_slice) in instance_slices {
                        debug!("Generating InstanceSliceRange for {entity:?}");
                        // Generate instance slice range
                        instance_slice_ranges.insert(
                            *entity,
                            InstanceSliceRange {
                                offset: offset as u64,
                                instance_count: instance_slice.instance_count as u64,
                            },
                        );

                        offset += instance_slice.instance_count;
                    }

                    debug!("Instance slice ranges: {instance_slice_ranges:?}");

                    (key.clone(), instance_slice_ranges)
                })
                .collect::<BTreeMap<_, _>>()
        });

        let span = bevy::prelude::info_span!("Populate instance slices");
        span.in_scope(|| {
            // Populate instance slices
            for (key, instance_slices) in keyed_instance_slices.iter() {
                // Collect instance data
                let instance_count: usize = instance_slices
                    .iter()
                    .map(|(_, _, instance_slice)| instance_slice.instance_count)
                    .sum();

                instance_buffer_data
                    .entry(key.clone())
                    .or_default()
                    .extend((0..instance_count).map(|_| default()));
            }
        });

        let view_instance_data = view_instance_data.entry(view_entity).or_default();
        for (key, instance_buffer_data) in instance_buffer_data {
            debug!(
                "Instance batch {key:#?} count: {}",
                instance_buffer_data.len()
            );

            let entry = view_instance_data.entry(key).or_insert_with(gpu_instances);

            entry.set(instance_buffer_data);
            entry.write_buffer(&render_device, &render_queue);
        }

        let span = bevy::prelude::info_span!("Write instance batches");
        span.in_scope(|| {
            // Write instance batches to meta
            instance_meta
                .instance_batches
                .extend(view_instance_data.keys().map(|key| {
                    let instances = keyed_instances
                        .remove(key)
                        .map(|instances| {
                            instances
                                .into_iter()
                                .map(|((_, _), (instance, _, _))| instance)
                                .collect::<BTreeSet<_>>()
                        })
                        .unwrap_or_default();

                    let instance_slice_ranges =
                        keyed_instance_slice_ranges.remove(&key).unwrap_or_default();

                    (
                        key.clone(),
                        InstanceBatch::<M> {
                            instances,
                            instance_slice_ranges,
                            _phantom: default(),
                        },
                    )
                }));
        });
    }
}

pub fn prune_instance_data<M: MaterialInstanced>(
    mut view_instance_data: ResMut<ViewInstanceData<M>>,
    query_instance_meta: Query<
        (Entity, &mut InstanceMeta<M>),
        (With<ExtractedView>, With<VisibleEntities>),
    >,
) {
    // Prune indirect data for views with no batches
    for entity in view_instance_data.keys().cloned().collect::<Vec<_>>() {
        if !query_instance_meta
            .iter()
            .any(|(view_entity, _)| view_entity == entity)
        {
            info!("View {entity:?} has no instance meta, pruning instance data");
            view_instance_data.remove(&entity);
        }
    }
}
