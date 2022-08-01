use std::collections::{BTreeMap, BTreeSet};

use bevy::{
    prelude::{debug, default, info, Entity, Handle, Mesh, Query, Res, ResMut, With},
    render::{
        renderer::RenderDevice,
        view::{ExtractedView, VisibleEntities},
    },
    utils::FloatOrd,
};

use crate::instancing::{
    instance_block::{InstanceBlock, InstanceBlockRange},
    material::{
        plugin::{
            GpuAlphaMode, GpuInstancedMeshes, GpuInstances, InstanceBatch, InstanceBatchKey,
            InstanceViewMeta, InstancedMaterialBatchKey, MeshBatch, RenderMaterials,
        },
        specialized_instanced_material::MaterialInstanced,
    },
    render::instance::Instance,
};

pub fn system<M: MaterialInstanced>(
    mut instance_view_meta: ResMut<InstanceViewMeta<M>>,
    render_device: Res<RenderDevice>,
    render_meshes: Res<GpuInstancedMeshes<M>>,
    render_materials: Res<RenderMaterials<M>>,
    mut query_views: Query<(Entity, &ExtractedView), With<VisibleEntities>>,
    query_instance: Query<(
        Entity,
        &Handle<M>,
        &Handle<Mesh>,
        &<M::Instance as Instance>::ExtractedInstance,
    )>,
    query_instance_block: Query<(Entity, &Handle<M>, &Handle<Mesh>, &InstanceBlock)>,
) {
    debug!("{}", std::any::type_name::<M>());

    let render_meshes = &render_meshes.instanced_meshes;

    for (view_entity, view) in query_views.iter_mut() {
        debug!("View {view_entity:?}");
        let instance_meta = instance_view_meta.get_mut(&view_entity).unwrap();

        // Fetch view matrix for sorting
        let inverse_view_matrix = view.transform.compute_matrix().inverse();
        let inverse_view_row_2 = inverse_view_matrix.row(2);

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
                let mesh = render_meshes.get(mesh_handle).unwrap();
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

                let mesh_z =
                    inverse_view_row_2.dot(<M::Instance as Instance>::transform(instance).col(3));

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

        debug!("Keyed instances:");
        for (key, instance) in keyed_instances.iter().enumerate() {
            debug!("{key:#?}: {instance:#?}");
        }

        let span = bevy::prelude::info_span!("Batch instance blocks by key");
        let keyed_instance_blocks = span.in_scope(|| {
            // Batch instance blocks by key
            let mut keyed_instance_blocks =
                BTreeMap::<InstanceBatchKey<M>, Vec<(Entity, &Handle<M>, &InstanceBlock)>>::new();

            for (entity, material_handle, mesh_handle, instance_block) in instance_meta
                .instance_blocks
                .iter()
                .flat_map(|entity| query_instance_block.get(*entity))
            {
                debug!("Instance block {entity:?}");
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

                keyed_instance_blocks.entry(key).or_default().push((
                    entity,
                    material_handle,
                    instance_block,
                ));
            }

            keyed_instance_blocks
        });

        debug!(
            "Keyed instance blocks: {:#?}",
            keyed_instance_blocks.values()
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
                let instance_buffer_data =
                    instances
                        .iter()
                        .map(|((mesh_handle, _), (_, _, instance))| {
                            let MeshBatch { meshes, .. } =
                                instance_meta.mesh_batches.get(&key.mesh_key).unwrap();

                            <M::Instance as Instance>::prepare_instance(
                                instance,
                                meshes.iter().position(|mesh| mesh == *mesh_handle).unwrap() as u32,
                            )
                        });

                keyed_instance_buffer_data
                    .entry(key.clone())
                    .or_insert_with(gpu_instances)
                    .push(instance_buffer_data.collect::<Vec<_>>());
            }
        });

        let span = bevy::prelude::info_span!("Create instance block ranges");
        let mut keyed_instance_block_ranges = span.in_scope(|| {
            debug!("Creating instance block ranges");
            // Create instance block ranges
            keyed_instance_blocks
                .iter()
                .map(|(key, instance_blocks)| {
                    let instance_buffer_data_len = keyed_instance_buffer_data
                        .get(&key)
                        .map(GpuInstances::len)
                        .unwrap_or_default();

                    // Collect CPU instance block data
                    let mut offset = instance_buffer_data_len;
                    let mut instance_block_ranges = BTreeMap::<Entity, InstanceBlockRange>::new();
                    for (entity, _, instance_block) in instance_blocks {
                        debug!("Generating InstanceBlockRange for {entity:?}");
                        // Generate instance block range
                        instance_block_ranges.insert(
                            *entity,
                            InstanceBlockRange {
                                offset: offset as u64,
                                instance_count: instance_block.instance_count as u64,
                            },
                        );

                        offset += instance_block.instance_count;
                    }

                    debug!("Instance block ranges: {instance_block_ranges:?}");

                    (key.clone(), instance_block_ranges)
                })
                .collect::<BTreeMap<_, _>>()
        });

        let span = bevy::prelude::info_span!("Populate instance blocks");
        span.in_scope(|| {
            // Populate instance blocks
            for (key, instance_blocks) in keyed_instance_blocks.iter() {
                // Collect instance data
                let instance_count: usize = instance_blocks
                    .iter()
                    .map(|(_, _, instance_block)| instance_block.instance_count)
                    .sum();

                let entry = keyed_instance_buffer_data
                    .entry(key.clone())
                    .or_insert_with(gpu_instances);
                entry.push((0..instance_count).map(|_| default()).collect::<Vec<_>>());
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

                        let instance_block_ranges =
                            keyed_instance_block_ranges.remove(&key).unwrap_or_default();

                        (
                            key.clone(),
                            InstanceBatch::<M> {
                                instances,
                                instance_block_ranges,
                                instance_buffer_data,
                            },
                        )
                    },
                ));
        });
    }
}
