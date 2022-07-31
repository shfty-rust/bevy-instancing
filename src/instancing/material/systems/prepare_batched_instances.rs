use std::collections::BTreeMap;

use bevy::{
    prelude::{debug, info, Commands, Entity, Handle, Mesh, Query, Res, ResMut, With},
    render::{
        mesh::Indices,
        render_resource::BufferVec,
        renderer::{RenderDevice, RenderQueue},
        view::{ExtractedView, VisibleEntities},
    },
};
use wgpu::{util::BufferInitDescriptor, BindGroupDescriptor, BindGroupEntry, BufferUsages};

use crate::instancing::{
    instance_block::{InstanceBlock, InstanceBlockBuffer},
    material::{
        instanced_material_pipeline::InstancedMaterialPipeline,
        plugin::{
            BatchedInstances, GpuIndexBufferData, GpuIndirectBufferData, GpuIndirectData,
            GpuInstancedMeshes, InstanceBatchKey, InstanceViewMeta, MeshBatch,
        },
        specialized_instanced_material::SpecializedInstancedMaterial,
    },
    render::instance::Instance,
};
use crate::prelude::{DrawIndexedIndirect, DrawIndirect};

#[allow(clippy::too_many_arguments)]
pub fn system<M: SpecializedInstancedMaterial>(
    instanced_material_pipeline: Res<InstancedMaterialPipeline<M>>,
    render_meshes: Res<GpuInstancedMeshes<M>>,
    mut instance_view_meta: ResMut<InstanceViewMeta<M>>,
    render_device: Res<RenderDevice>,
    render_queue: Res<RenderQueue>,
    query_instance: Query<(
        Entity,
        &Handle<M>,
        &Handle<Mesh>,
        &<M::Instance as Instance>::ExtractedInstance,
    )>,
    query_instance_block: Query<(Entity, &Handle<M>, &Handle<Mesh>, &InstanceBlock)>,
    mut query_views: Query<Entity, (With<ExtractedView>, With<VisibleEntities>)>,
    mut commands: Commands,
) {
    debug!("{}", std::any::type_name::<M>());

    let render_meshes = &render_meshes.instanced_meshes;

    for view_entity in query_views.iter_mut() {
        debug!("\tView {view_entity:?}");
        let instance_meta = instance_view_meta.get_mut(&view_entity).unwrap();

        // Process batches
        let mut batched_instances = BTreeMap::<InstanceBatchKey<M>, BatchedInstances>::new();
        for (key, instance_batch) in &mut instance_meta.instance_batches {
            // Fetch data
            let MeshBatch {
                meshes,
                vertex_data,
                index_data,
                indirect_data,
            } = instance_meta.mesh_batches.get(&key.mesh_key).unwrap();

            // Calculate mesh instance counts for this batch
            let mut mesh_instance_counts = BTreeMap::<&Handle<Mesh>, usize>::new();

            for mesh in meshes {
                mesh_instance_counts.insert(mesh, 0);
            }

            let instance_meshes = instance_batch
                .instances
                .iter()
                .flat_map(|entity| query_instance.get(*entity))
                .map(|(_, _, mesh, _)| mesh);

            for mesh in instance_meshes {
                *mesh_instance_counts.get_mut(mesh).unwrap() += 1;
            }

            for (mesh, instance_block) in instance_batch
                .instance_block_ranges
                .iter()
                .flat_map(|(entity, _)| query_instance_block.get(*entity))
                .map(|(_, _, mesh, instance_block)| (mesh, instance_block))
            {
                *mesh_instance_counts.get_mut(mesh).unwrap() += instance_block.instance_count;
            }

            debug!("Mesh instance counts: {mesh_instance_counts:?}");

            // Calculate instance offsets for this batch
            let (mesh_instance_offsets, _) = mesh_instance_counts.iter().fold(
                (BTreeMap::<&Handle<Mesh>, usize>::new(), 0),
                |(mut offsets, mut offset), (mesh, count)| {
                    offsets.insert(mesh, offset);
                    offset += count;
                    (offsets, offset)
                },
            );

            // Calculate vertex offsets for this batch's mesh
            let (mesh_vertex_offsets, _) = mesh_instance_counts.iter().fold(
                (BTreeMap::<&Handle<Mesh>, usize>::new(), 0),
                |(mut offsets, mut offset), (mesh, _)| {
                    offsets.insert(mesh, offset);

                    let gpu_mesh = render_meshes.get(mesh).unwrap();

                    offset += match gpu_mesh.index_buffer_data {
                        GpuIndexBufferData::Indexed { index_count, .. } => index_count,
                        GpuIndexBufferData::NonIndexed { vertex_count } => vertex_count,
                    } as usize;

                    (offsets, offset)
                },
            );

            // Upload GPU data and create bind groups
            let vertex_buffer = render_device.create_buffer_with_data(&BufferInitDescriptor {
                label: Some("instanced vertex buffer"),
                contents: vertex_data,
                usage: BufferUsages::VERTEX,
            });

            let index_buffer = match index_data {
                Some(GpuIndexBufferData::Indexed { indices, .. }) => Some({
                    render_device.create_buffer_with_data(&BufferInitDescriptor {
                        label: Some("instanced index buffer"),
                        contents: match indices {
                            Indices::U16(indices) => bytemuck::cast_slice(indices),
                            Indices::U32(indices) => bytemuck::cast_slice(indices),
                        },
                        usage: BufferUsages::INDEX,
                    })
                }),
                _ => None,
            };

            let indirect_buffer = match indirect_data {
                GpuIndirectData::Indexed { buffer } => {
                    let indirect_data = buffer
                        .into_iter()
                        .copied()
                        .zip(
                            mesh_instance_counts.values().zip(
                                mesh_vertex_offsets
                                    .values()
                                    .zip(mesh_instance_offsets.values()),
                            ),
                        )
                        .map(
                            |(indirect, (instance_count, (index_offset, instance_offset)))| {
                                DrawIndexedIndirect {
                                    instance_count: *instance_count as u32,
                                    base_index: *index_offset as u32,
                                    base_instance: *instance_offset as u32,
                                    ..indirect
                                }
                            },
                        )
                        .collect::<Vec<_>>();

                    let mut indirect_buffer = BufferVec::new(BufferUsages::INDIRECT);
                    for indirect in &indirect_data {
                        indirect_buffer.push(*indirect);
                    }
                    indirect_buffer.write_buffer(&render_device, &render_queue);

                    GpuIndirectBufferData::Indexed {
                        indirects: indirect_data,
                        buffer: indirect_buffer,
                    }
                }
                GpuIndirectData::NonIndexed { buffer } => {
                    let indirect_data = buffer
                        .into_iter()
                        .copied()
                        .zip(
                            mesh_instance_counts.values().zip(
                                mesh_vertex_offsets
                                    .values()
                                    .zip(mesh_instance_offsets.values()),
                            ),
                        )
                        .map(
                            |(indirect, (instance_count, (vertex_offset, instance_offset)))| {
                                DrawIndirect {
                                    instance_count: *instance_count as u32,
                                    base_vertex: *vertex_offset as u32,
                                    base_instance: *instance_offset as u32,
                                    ..indirect
                                }
                            },
                        )
                        .collect::<Vec<_>>();

                    let mut indirect_buffer = BufferVec::new(BufferUsages::INDIRECT);
                    for indirect in &indirect_data {
                        indirect_buffer.push(*indirect);
                    }
                    indirect_buffer.write_buffer(&render_device, &render_queue);

                    GpuIndirectBufferData::NonIndexed {
                        indirects: indirect_data,
                        buffer: indirect_buffer,
                    }
                }
            };

            let indirect_indices = mesh_instance_counts
                .iter()
                .enumerate()
                .flat_map(|(i, (_, count))| if *count > 0 { Some(i) } else { None })
                .collect::<Vec<_>>();

            debug!("Indirect indices: {indirect_indices:#?}");

            instance_batch
                .instance_buffer_data
                .write_buffer(&render_device, &render_queue);

            let instance_bind_group = render_device.create_bind_group(&BindGroupDescriptor {
                label: Some("instance bind group"),
                layout: &instanced_material_pipeline
                    .instanced_mesh_pipeline
                    .bind_group_layout,
                entries: &[BindGroupEntry {
                    binding: 0,
                    resource: instance_batch.instance_buffer_data.binding().unwrap(),
                }],
            });

            // Insert instance block data
            for (entity, block_range) in instance_batch.instance_block_ranges.iter() {
                commands
                    .entity(*entity)
                    .insert(*block_range)
                    .insert(InstanceBlockBuffer {
                        buffer: instance_batch
                            .instance_buffer_data
                            .buffer()
                            .cloned()
                            .unwrap(),
                    });
            }

            // Spawn entity
            let material_batch = instance_meta
                .material_batches
                .get(&key.material_key)
                .unwrap();

            let batch_entity = commands
                .spawn()
                .insert(material_batch.material.clone_weak())
                .insert(key.clone())
                .id();

            // Insert meta
            let indirect_count = indirect_data.len();
            batched_instances.insert(
                key.clone(),
                BatchedInstances {
                    view_entity,
                    batch_entity,
                    vertex_buffer,
                    index_data: index_buffer
                        .map(|index_buffer| (index_buffer, key.mesh_key.index_format.unwrap())),
                    instance_bind_group,
                    indirect_buffer,
                    indirect_count,
                    indirect_indices,
                },
            );
        }

        instance_meta.batched_instances.extend(batched_instances);
    }
}
