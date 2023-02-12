use std::collections::{BTreeMap, BTreeSet};

use crate::{
    instancing::material::plugin::GpuIndexBufferData,
    prelude::{DrawIndexedIndirect, DrawIndirect},
};
use bevy::{
    prelude::{debug, default, info_span, Deref, DerefMut, Handle, Mesh, Res, ResMut},
    render::{
        mesh::Indices,
        render_resource::BufferVec,
        renderer::{RenderDevice, RenderQueue},
    },
};
// use wgpu::BufferUsages;
use bevy::render::render_resource::BufferUsages;

use crate::instancing::material::plugin::{GpuIndirectData, InstancedMeshKey, RenderMeshes};

pub enum BufferIndices {
    U32(BufferVec<u32>),
    U16(BufferVec<u16>),
}

impl BufferIndices {
    pub fn len(&self) -> usize {
        match self {
            BufferIndices::U32(indices) => indices.len(),
            BufferIndices::U16(indices) => indices.len(),
        }
    }
}

pub struct MeshBatch {
    pub meshes: BTreeSet<Handle<Mesh>>,
    pub vertex_data: BufferVec<u8>,
    pub index_data: Option<BufferVec<u8>>,
    pub indirect_data: GpuIndirectData,
}

#[derive(Default, Deref, DerefMut)]
pub struct MeshBatches {
    pub mesh_batches: BTreeMap<InstancedMeshKey, MeshBatch>,
}

pub fn system(
    render_device: Res<RenderDevice>,
    render_queue: Res<RenderQueue>,
    render_meshes: Res<RenderMeshes>,
    mut mesh_batches: ResMut<MeshBatches>,
) {
    if !render_meshes.is_changed() {
        return;
    }

    let render_meshes = &render_meshes.instanced_meshes;

    // Sort meshes into batches by their InstancedMeshKey
    let keyed_meshes = info_span!("Key meshes").in_scope(|| {
        let mut keyed_meshes = BTreeMap::<InstancedMeshKey, BTreeSet<Handle<Mesh>>>::new();
        for (handle, mesh) in render_meshes.iter() {
            keyed_meshes
                .entry(mesh.key.clone())
                .or_default()
                .insert(handle.clone_weak());
        }
        keyed_meshes
    });

    // Generate vertex, index, and indirect data for each batch
    info_span!("Batch meshes").in_scope(|| {
        mesh_batches.extend({
            keyed_meshes.into_iter().map(|(key, meshes)| {
                let vertex_data = info_span!("Vertex data").in_scope(|| {
                    let mut vertex_data =
                        BufferVec::new(BufferUsages::VERTEX | BufferUsages::COPY_DST);

                    let bytes = meshes
                        .iter()
                        .flat_map(|mesh| render_meshes.get(mesh))
                        .flat_map(|mesh| mesh.vertex_buffer_data.iter())
                        .copied()
                        .collect::<Vec<_>>();

                    vertex_data.reserve(bytes.len(), &render_device);

                    for byte in bytes {
                        vertex_data.push(byte);
                    }

                    vertex_data.write_buffer(&render_device, &render_queue);

                    vertex_data
                });

                let index_data = info_span!("Index data").in_scope(|| {
                    let mut base_index = 0;
                    let indices = meshes.iter().fold(None, |acc, mesh| {
                        let mesh = render_meshes.get(mesh).unwrap();

                        let out = match &mesh.index_buffer_data {
                            GpuIndexBufferData::Indexed { indices, .. } => Some(match acc {
                                Some(acc_indices) => match (acc_indices, indices) {
                                    (Indices::U16(lhs), Indices::U16(rhs)) => Indices::U16(
                                        lhs.iter()
                                            .copied()
                                            .chain(rhs.iter().map(|idx| base_index as u16 + *idx))
                                            .collect(),
                                    ),
                                    (Indices::U32(lhs), Indices::U32(rhs)) => Indices::U32(
                                        lhs.iter()
                                            .copied()
                                            .chain(rhs.iter().map(|idx| base_index as u32 + *idx))
                                            .collect(),
                                    ),
                                    _ => panic!("Mismatched index format"),
                                },
                                None => indices.clone(),
                            }),
                            GpuIndexBufferData::NonIndexed { .. } => None,
                        };

                        base_index += mesh.vertex_count;

                        out
                    });

                    indices.map(|indices| {
                        let bytes: Vec<u8> = match indices {
                            Indices::U16(indices) => bytemuck::cast_slice(&indices).to_vec(),
                            Indices::U32(indices) => bytemuck::cast_slice(&indices).to_vec(),
                        };

                        let mut index_data =
                            BufferVec::new(BufferUsages::INDEX | BufferUsages::COPY_DST);

                        index_data.reserve(bytes.len(), &render_device);

                        for byte in bytes {
                            index_data.push(byte);
                        }

                        index_data.write_buffer(&render_device, &render_queue);

                        index_data
                    })
                });

                let mut base_index = 0u32;
                let indirect_data =
                    info_span!("Indirect data").in_scope(|| match key.index_format {
                        Some(_) => GpuIndirectData::Indexed {
                            buffer: meshes
                                .iter()
                                .map(|mesh| {
                                    match &render_meshes.get(mesh).unwrap().index_buffer_data {
                                        GpuIndexBufferData::Indexed { indices, .. } => {
                                            base_index += indices.len() as u32;

                                            DrawIndexedIndirect {
                                                vertex_count: indices.len() as u32,
                                                ..default()
                                            }
                                        }
                                        _ => panic!("Mismatched GpuIndexBufferData"),
                                    }
                                })
                                .collect::<Vec<_>>(),
                        },
                        None => GpuIndirectData::NonIndexed {
                            buffer: meshes
                                .iter()
                                .map(|mesh| {
                                    match &render_meshes.get(mesh).unwrap().index_buffer_data {
                                        GpuIndexBufferData::NonIndexed { vertex_count } => {
                                            base_index += vertex_count;

                                            DrawIndirect {
                                                vertex_count: *vertex_count,
                                                ..default()
                                            }
                                        }
                                        _ => panic!("Mismatched GpuIndexBufferData"),
                                    }
                                })
                                .collect::<Vec<_>>(),
                        },
                    });

                debug!("Mesh batch {key:#?}: {meshes:#?}");

                (
                    key.clone(),
                    MeshBatch {
                        meshes,
                        vertex_data,
                        index_data,
                        indirect_data,
                    },
                )
            })
        })
    });
}
