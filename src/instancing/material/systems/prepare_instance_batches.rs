use std::collections::{BTreeMap, BTreeSet};

use bevy::{
    prelude::{debug, default, Entity, Handle, Mesh, Query, Res, With, info},
    render::{
        renderer::RenderDevice,
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
    render::instance::{Instance, InstanceUniformLength},
};

use super::prepare_mesh_batches::MeshBatches;

pub fn system<M: MaterialInstanced>(
    render_device: Res<RenderDevice>,
    render_meshes: Res<RenderMeshes>,
    render_materials: Res<RenderMaterials<M>>,
    mesh_batches: Res<MeshBatches>,
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
                BTreeMap<
                    (&Handle<Mesh>, FloatOrd),
                    (
                        Entity,
                        &Handle<M>,
                        &<M::Instance as Instance>::ExtractedInstance,
                    ),
                >,
            >::new();

            for (entity, material_handle, mesh_handle, instance) in instance_meta
                .instances
                .iter()
                .flat_map(|entity| query_instance.get(*entity))
            {
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

                keyed_instances.entry(key).or_default().insert(
                    (mesh_handle, FloatOrd(dist)),
                    (entity, material_handle, instance),
                );
            }

            keyed_instances
        });

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
                let mesh = render_meshes.get(mesh_handle).unwrap();
                let mesh_key = mesh.key.clone();

                let material = render_materials.get(material_handle).unwrap();
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

        // Create an instance buffer vec for each key
        let mut keyed_instance_buffer_data =
            BTreeMap::<InstanceBatchKey<M>, GpuInstances<M>>::new();

        let gpu_instances =
            || GpuInstances::new(render_device.get_supported_read_only_binding_type(1));

        let span = bevy::prelude::info_span!("Populate instances");
        span.in_scope(|| {
            debug!("Populating instances");
            // Populate instances
            for (key, instances) in keyed_instances.iter() {
                debug!("{key:#?}");
                // Collect instance data
                let mut instance_buffer_data = instances
                    .iter()
                    .map(|((mesh_handle, _), (_, _, instance))| {
                        let MeshBatch { meshes, .. } = mesh_batches.get(&key.mesh_key).unwrap();

                        <M::Instance as Instance>::prepare_instance(
                            instance,
                            meshes.iter().position(|mesh| mesh == *mesh_handle).unwrap() as u32,
                        )
                    })
                    .collect::<Vec<_>>();

                let min_length = <M::Instance as InstanceUniformLength>::UNIFORM_BUFFER_LENGTH.get() as usize;
                if instance_buffer_data.len() < min_length {
                    instance_buffer_data.resize(min_length, default());
                }

                keyed_instance_buffer_data
                    .entry(key.clone())
                    .or_insert_with(gpu_instances)
                    .set(instance_buffer_data);
            }
        });

        let span = bevy::prelude::info_span!("Create instance slice ranges");
        let mut keyed_instance_slice_ranges = span.in_scope(|| {
            debug!("Creating instance slice ranges");
            // Create instance slice ranges
            keyed_instance_slices
                .iter()
                .map(|(key, instance_slices)| {
                    let instance_buffer_data_len = keyed_instance_buffer_data
                        .get(&key)
                        .map(GpuInstances::len)
                        .unwrap_or_default();

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

                let entry = keyed_instance_buffer_data
                    .entry(key.clone())
                    .or_insert_with(gpu_instances);

                entry.set((0..instance_count).map(|_| default()).collect::<Vec<_>>());
            }
        });

        let span = bevy::prelude::info_span!("Write instance batches");
        span.in_scope(|| {
            // Write instance batches to meta
            instance_meta
                .instance_batches
                .extend(keyed_instance_buffer_data.into_iter().map(
                    |(key, instance_buffer_data)| {
                        let instances = keyed_instances
                            .remove(&key)
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
                                instance_buffer_data,
                            },
                        )
                    },
                ));
        });
    }
}
