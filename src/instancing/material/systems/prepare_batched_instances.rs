use std::collections::BTreeMap;

use bevy::{
    prelude::{debug, info_span, Entity, Handle, Mesh, Query, Res, With},
    render::{
        mesh::Indices,
        render_resource::BufferVec,
        renderer::{RenderDevice, RenderQueue},
        view::{ExtractedView, VisibleEntities},
    },
};
use wgpu::{util::BufferInitDescriptor, BindGroupDescriptor, BindGroupEntry, BufferUsages};

use crate::instancing::{
    instance_slice::InstanceSlice,
    material::{
        instanced_material_pipeline::InstancedMaterialPipeline,
        material_instanced::MaterialInstanced,
        plugin::{
            BatchedInstances, GpuIndexBufferData, GpuIndirectBufferData, GpuIndirectData,
            InstanceMeta, RenderMeshes,
        },
    },
    render::instance::Instance,
};
use crate::prelude::{DrawIndexedIndirect, DrawIndirect};

use super::prepare_mesh_batches::MeshBatches;

#[allow(clippy::too_many_arguments)]
pub fn system<M: MaterialInstanced>(
    instanced_material_pipeline: Res<InstancedMaterialPipeline<M>>,
    render_meshes: Res<RenderMeshes>,
    render_device: Res<RenderDevice>,
    render_queue: Res<RenderQueue>,
    mesh_batches: Res<MeshBatches>,
    query_instance: Query<(
        Entity,
        &Handle<M>,
        &Handle<Mesh>,
        &<M::Instance as Instance>::ExtractedInstance,
    )>,
    query_instance_slice: Query<(Entity, &Handle<M>, &Handle<Mesh>, &InstanceSlice)>,
    mut query_views: Query<
        (Entity, &mut InstanceMeta<M>),
        (With<ExtractedView>, With<VisibleEntities>),
    >,
) {
    debug!("{}", std::any::type_name::<M>());

    let render_meshes = &render_meshes.instanced_meshes;

    for (view_entity, mut instance_meta) in query_views.iter_mut() {
        debug!("\tView {view_entity:?}");

        // Process batches
        for key in instance_meta
            .instance_batches
            .keys()
            .cloned()
            .collect::<Vec<_>>()
        {
            debug!("{key:#?}");

            // Fetch data
            let mesh_batch = mesh_batches.get(&key.mesh_key).unwrap();
            let indirect_count = mesh_batch.indirect_data.len();

            // Calculate mesh instance counts for this batch
            let mesh_instance_counts = info_span!("Mesh instance counts").in_scope(|| {
                let mut mesh_instance_counts = mesh_batch
                    .meshes
                    .iter()
                    .map(|mesh| (mesh, 0))
                    .collect::<BTreeMap<_, _>>();

                let instance_batch = instance_meta.instance_batches.get(&key).unwrap();

                for mesh in query_instance.iter().filter_map(|(entity, _, mesh, _)| {
                    if instance_batch.instances.contains(&entity) {
                        Some(mesh)
                    } else {
                        None
                    }
                }) {
                    *mesh_instance_counts.get_mut(mesh).unwrap() += 1;
                }

                for (mesh, instance_slice) in
                    query_instance_slice
                        .iter()
                        .filter_map(|(entity, _, mesh, instance_slice)| {
                            if instance_batch.instance_slice_ranges.contains_key(&entity) {
                                Some((mesh, instance_slice))
                            } else {
                                None
                            }
                        })
                {
                    *mesh_instance_counts.get_mut(mesh).unwrap() += instance_slice.instance_count;
                }

                debug!("Mesh instance counts: {mesh_instance_counts:?}");
                mesh_instance_counts
            });

            // Calculate instance offsets for this batch
            let (mesh_instance_offsets, _) = info_span!("Mesh instance offsets").in_scope(|| {
                mesh_instance_counts.iter().fold(
                    (BTreeMap::<&Handle<Mesh>, usize>::new(), 0),
                    |(mut offsets, mut offset), (mesh, count)| {
                        offsets.insert(mesh, offset);
                        offset += count;
                        (offsets, offset)
                    },
                )
            });

            // Calculate vertex offsets for this batch's mesh
            let (mesh_vertex_offsets, _) = info_span!("Mesh vertex offsets").in_scope(|| {
                mesh_instance_counts.iter().fold(
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
                )
            });

            // Create buffers
            let vertex_buffer = info_span!("Create vertex buffer").in_scope(|| {
                render_device.create_buffer_with_data(&BufferInitDescriptor {
                    label: Some("instanced vertex buffer"),
                    contents: &mesh_batch.vertex_data,
                    usage: BufferUsages::VERTEX,
                })
            });

            let index_buffer =
                info_span!("Create index buffer").in_scope(|| match &mesh_batch.index_data {
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
                });

            let mut indirect_buffer =
                info_span!("Create indirect buffer").in_scope(|| match &mesh_batch.indirect_data {
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

                        GpuIndirectBufferData::NonIndexed {
                            indirects: indirect_data,
                            buffer: indirect_buffer,
                        }
                    }
                });

            // Calculate indirect indices
            let indirect_indices = info_span!("Indirect indices").in_scope(|| {
                mesh_instance_counts
                    .iter()
                    .enumerate()
                    .flat_map(|(i, (_, count))| if *count > 0 { Some(i) } else { None })
                    .collect::<Vec<_>>()
            });

            debug!("Indirect indices: {indirect_indices:#?}");

            // Write buffers
            info_span!("Write buffers").in_scope(|| {
                instance_meta
                    .instance_batches
                    .get_mut(&key)
                    .unwrap()
                    .instance_buffer_data
                    .write_buffer(&render_device, &render_queue);

                indirect_buffer.write_buffer(&render_device, &render_queue)
            });

            // Create bind group
            let instance_bind_group = info_span!("Create bind group").in_scope(|| {
                render_device.create_bind_group(&BindGroupDescriptor {
                    label: Some("instance bind group"),
                    layout: &instanced_material_pipeline
                        .instanced_mesh_pipeline
                        .bind_group_layout,
                    entries: &[BindGroupEntry {
                        binding: 0,
                        resource: instance_meta
                            .instance_batches
                            .get(&key)
                            .unwrap()
                            .instance_buffer_data
                            .binding()
                            .unwrap(),
                    }],
                })
            });

            // Insert meta
            info_span!("Insert meta").in_scope(|| {
                instance_meta.batched_instances.insert(
                    key.clone(),
                    BatchedInstances {
                        vertex_buffer,
                        index_data: index_buffer
                            .map(|index_buffer| (index_buffer, key.mesh_key.index_format.unwrap())),
                        instance_bind_group,
                        indirect_buffer,
                        indirect_count,
                        indirect_indices,
                    },
                )
            });
        }
    }
}
