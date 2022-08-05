use std::{collections::BTreeMap, num::NonZeroU64};

use bevy::{
    prelude::{
        debug, default, info, info_span, Deref, DerefMut, Entity, Handle, Mesh, Query, Res, ResMut,
        With,
    },
    render::{
        render_resource::{BufferVec, ShaderSize},
        renderer::{RenderDevice, RenderQueue},
        view::{ExtractedView, VisibleEntities},
    },
};
use wgpu::{BindGroupDescriptor, BindGroupEntry, BufferBinding, BufferUsages};

use crate::instancing::{
    instance_slice::InstanceSlice,
    material::{
        instanced_material_pipeline::InstancedMaterialPipeline,
        material_instanced::MaterialInstanced,
        plugin::{
            BatchedInstances, GpuIndexBufferData, GpuIndirectBufferData, GpuIndirectData,
            GpuInstances, InstanceBatchKey, InstanceMeta, RenderMeshes,
        },
    },
    render::instance::{Instance, InstanceUniformLength},
};
use crate::prelude::{DrawIndexedIndirect, DrawIndirect};

use super::{prepare_instance_batches::ViewInstanceData, prepare_mesh_batches::MeshBatches};

#[derive(Deref, DerefMut)]
pub struct ViewIndirectData<M: MaterialInstanced> {
    pub indirect_data: BTreeMap<Entity, BTreeMap<InstanceBatchKey<M>, Vec<BufferVec<u8>>>>,
}

impl<M: MaterialInstanced> Default for ViewIndirectData<M> {
    fn default() -> Self {
        Self {
            indirect_data: default(),
        }
    }
}

#[allow(clippy::too_many_arguments)]
pub fn system<M: MaterialInstanced>(
    instanced_material_pipeline: Res<InstancedMaterialPipeline<M>>,
    render_meshes: Res<RenderMeshes>,
    render_device: Res<RenderDevice>,
    render_queue: Res<RenderQueue>,
    mesh_batches: Res<MeshBatches>,
    view_instance_data: Res<ViewInstanceData<M>>,
    mut view_indirect_data: ResMut<ViewIndirectData<M>>,
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

        let view_instance_data = view_instance_data.get(&view_entity).unwrap();
        let view_indirect_data = view_indirect_data.entry(view_entity).or_default();

        // Process batches
        for key in instance_meta
            .instance_batches
            .keys()
            .cloned()
            .collect::<Vec<_>>()
        {
            debug!("{key:#?}");

            // Fetch mesh batch data
            let mesh_batch = mesh_batches.get(&key.mesh_key).unwrap();

            // Fetch vertex and index buffers
            let vertex_buffer = mesh_batch.vertex_data.buffer().unwrap().clone();
            let index_buffer = mesh_batch
                .index_data
                .as_ref()
                .map(|index_data| index_data.buffer().unwrap().clone())
                .map(|index_buffer| (index_buffer, key.mesh_key.index_format.unwrap()));

            // Calculate mesh instance counts for indirect data
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

            // Calculate instance offsets for indirect data
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

            // Calculate vertex offsets for indirect data
            let (mesh_vertex_offsets, _) = info_span!("Mesh vertex offsets").in_scope(|| {
                mesh_instance_counts.iter().fold(
                    (BTreeMap::<&Handle<Mesh>, usize>::new(), 0),
                    |(mut offsets, mut offset), (mesh, _)| {
                        offsets.insert(mesh, offset);

                        let gpu_mesh = render_meshes.get(mesh).unwrap();

                        offset += match &gpu_mesh.index_buffer_data {
                            GpuIndexBufferData::Indexed { indices, .. } => indices.len(),
                            GpuIndexBufferData::NonIndexed { vertex_count } => {
                                *vertex_count as usize
                            }
                        };

                        (offsets, offset)
                    },
                )
            });

            // Create bind group
            let instance_buffer_data = view_instance_data.get(&key).unwrap();

            // Build indirect buffer
            let indirect_buffers = view_indirect_data.entry(key.clone()).or_default();

            let mut indirect_buffer_data =
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
                            .flat_map(
                                |(indirect, (instance_count, (index_offset, instance_offset)))| {
                                    if *instance_count > 0 {
                                        Some(DrawIndexedIndirect {
                                            instance_count: *instance_count as u32,
                                            base_index: *index_offset as u32,
                                            base_instance: *instance_offset as u32,
                                            ..indirect
                                        })
                                    } else {
                                        None
                                    }
                                },
                            )
                            .collect::<Vec<_>>();

                        debug!("Indirect data: {indirect_data:#?}");

                        let mut split_data = vec![];
                        if matches!(instance_buffer_data, GpuInstances::Uniform { .. }) {
                            split_data.push(vec![]);
                            let mut current_split = &mut split_data[0];

                            let mut count = 0usize;
                            let mut offset = 0;
                            for indirect in &indirect_data {
                                count += indirect.instance_count as usize;
                                debug!("Count: {count:}");
                                let total =
                                    <M::Instance as InstanceUniformLength>::UNIFORM_BUFFER_LENGTH
                                        .get();
                                let delta = count as isize - total as isize;
                                debug!("Delta: {delta:}");
                                if delta > 0 {
                                    debug!("Splitting batch");
                                    let mut indirect = *indirect;
                                    let lhs_count = indirect.instance_count - delta as u32;
                                    indirect.instance_count = lhs_count;
                                    indirect.base_instance -= offset;
                                    current_split.push(indirect);

                                    drop(current_split);

                                    split_data.push(vec![]);
                                    current_split = split_data.last_mut().unwrap();

                                    count = delta as usize;

                                    indirect.instance_count = count as u32;
                                    offset += indirect.base_instance + lhs_count;
                                    indirect.base_instance = 0;
                                    current_split.push(indirect);
                                } else {
                                    let mut indirect = *indirect;
                                    indirect.base_instance -= offset;
                                    current_split.push(indirect);
                                }
                            }
                        } else {
                            split_data.push(indirect_data);
                        }

                        debug!("Split data: {split_data:#?}");

                        split_data
                            .into_iter()
                            .enumerate()
                            .map(|(i, data)| {
                                if indirect_buffers.len() < i + 1 {
                                    indirect_buffers.push(BufferVec::new(
                                        BufferUsages::INDIRECT | BufferUsages::COPY_DST,
                                    ));
                                }

                                let indirect_buffer = &mut indirect_buffers[i];

                                let bytes: Vec<u8> = bytemuck::cast_slice(&data).to_vec();

                                indirect_buffer.clear();

                                for byte in bytes {
                                    indirect_buffer.push(byte);
                                }

                                indirect_buffer.write_buffer(&render_device, &render_queue);

                                GpuIndirectBufferData::Indexed {
                                    indirects: data,
                                    buffer: indirect_buffer.buffer().unwrap().clone(),
                                }
                            })
                            .collect::<Vec<_>>()
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

                        let mut split_data = vec![];
                        if matches!(instance_buffer_data, GpuInstances::Uniform { .. }) {
                            split_data.push(vec![]);
                            let mut current_split = &mut split_data[0];

                            let mut count = 0usize;
                            for indirect in &indirect_data {
                                count += indirect.instance_count as usize;
                                info!("Count: {count:}");
                                let total =
                                    <M::Instance as InstanceUniformLength>::UNIFORM_BUFFER_LENGTH
                                        .get();
                                let delta = count as isize - total as isize;
                                info!("Delta: {delta:}");
                                if delta > 0 {
                                    info!("Splitting batch");
                                    let mut indirect = *indirect;
                                    let lhs_count = indirect.instance_count - delta as u32;
                                    indirect.instance_count = lhs_count;
                                    current_split.push(indirect);

                                    drop(current_split);

                                    split_data.push(vec![]);
                                    current_split = split_data.last_mut().unwrap();

                                    indirect.instance_count = delta as u32;
                                    indirect.base_instance += lhs_count;
                                    current_split.push(indirect);

                                    count = 0;
                                } else {
                                    current_split.push(*indirect);
                                }
                            }
                        } else {
                            split_data.push(indirect_data);
                        }

                        split_data
                            .into_iter()
                            .enumerate()
                            .map(|(i, data)| {
                                if indirect_buffers.len() < i + 1 {
                                    indirect_buffers.push(BufferVec::new(
                                        BufferUsages::INDIRECT | BufferUsages::COPY_DST,
                                    ));
                                }

                                let indirect_buffer = &mut indirect_buffers[i];

                                let bytes: Vec<u8> = bytemuck::cast_slice(&data).to_vec();

                                indirect_buffer.clear();

                                for byte in bytes {
                                    indirect_buffer.push(byte);
                                }

                                indirect_buffer.write_buffer(&render_device, &render_queue);

                                GpuIndirectBufferData::NonIndexed {
                                    indirects: data,
                                    buffer: indirect_buffer.buffer().unwrap().clone(),
                                }
                            })
                            .collect::<Vec<_>>()
                    }
                });

            let mut batches = vec![];

            match instance_buffer_data {
                GpuInstances::Uniform { buffers } => {
                    info!("Buffers: {}", buffers.len());
                    for (i, (buffer, indirect)) in
                        buffers.into_iter().zip(indirect_buffer_data).enumerate()
                    {
                        info!("BatchedInstances {i:}");
                        let bind_group = render_device.create_bind_group(&BindGroupDescriptor {
                            label: Some("instance bind group"),
                            layout: &instanced_material_pipeline
                                .instanced_mesh_pipeline
                                .bind_group_layout,
                            entries: &[BindGroupEntry {
                                binding: 0,
                                resource: wgpu::BindingResource::Buffer(BufferBinding {
                                    buffer: buffer.buffer().unwrap(),
                                    offset: 0,
                                    size: Some(
                                        NonZeroU64::new(<M::Instance as InstanceUniformLength>::UNIFORM_BUFFER_LENGTH.get() * <M::Instance as Instance>::PreparedInstance::SHADER_SIZE.get()).unwrap(),
                                    ),
                                }),
                            }],
                        });

                        batches.push(BatchedInstances {
                            vertex_buffer: vertex_buffer.clone(),
                            index_buffer: index_buffer.clone(),
                            indirect_buffer: indirect,
                            bind_group,
                        });
                    }
                }
                GpuInstances::Storage { buffer } => {
                    let bind_group = render_device.create_bind_group(&BindGroupDescriptor {
                        label: Some("instance bind group"),
                        layout: &instanced_material_pipeline
                            .instanced_mesh_pipeline
                            .bind_group_layout,
                        entries: &[BindGroupEntry {
                            binding: 0,
                            resource: buffer.binding().unwrap(),
                        }],
                    });

                    batches.push(BatchedInstances {
                        vertex_buffer,
                        index_buffer,
                        indirect_buffer: indirect_buffer_data.remove(0),
                        bind_group,
                    });
                }
            }

            // Insert meta
            info_span!("Insert meta")
                .in_scope(|| instance_meta.batched_instances.insert(key.clone(), batches));
        }
    }
}
