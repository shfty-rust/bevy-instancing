use std::collections::{BTreeMap, BTreeSet};

use bevy::{
    prelude::{debug, default, info, Entity, Handle, Mesh, Query, Res, ResMut, With},
    render::{
        mesh::Indices,
        view::{ExtractedView, VisibleEntities},
    },
};

use crate::instancing::{
    instance_block::InstanceBlock,
    material::{
        plugin::{
            DrawIndirectVariant, GpuIndexBufferData, GpuInstancedMeshes, InstanceViewMeta,
            InstancedMeshKey, MeshBatch,
        },
        specialized_instanced_material::SpecializedInstancedMaterial,
    },
    render::{
        draw_indexed_indirect::DrawIndexedIndirect, draw_indirect::DrawIndirect, instance::Instance,
    },
};

pub fn system<M: SpecializedInstancedMaterial>(
    mut instance_view_meta: ResMut<InstanceViewMeta<M>>,
    render_meshes: Res<GpuInstancedMeshes<M>>,
    query_views: Query<Entity, (With<ExtractedView>, With<VisibleEntities>)>,
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

    for view_entity in query_views.iter() {
        debug!("View {view_entity:?}");
        let instance_meta = instance_view_meta.get_mut(&view_entity).unwrap();

        // Collect set of visible meshes
        let meshes = instance_meta
            .instances
            .iter()
            .flat_map(|entity| query_instance.get(*entity))
            .map(|(_, _, mesh, _)| mesh.clone_weak())
            .chain(
                instance_meta
                    .instance_blocks
                    .iter()
                    .flat_map(|entity| {
                        debug!("Instance block {entity:?}");
                        query_instance_block.get(*entity)
                    })
                    .map(|(_, _, mesh, _)| {
                        debug!("Mesh: {mesh:?}");
                        mesh.clone_weak()
                    }),
            )
            .collect::<BTreeSet<_>>();

        // Sort meshes into batches by their InstancedMeshKey
        let mut keyed_meshes = BTreeMap::<InstancedMeshKey, BTreeSet<Handle<Mesh>>>::new();
        for mesh_handle in meshes.into_iter() {
            let mesh = render_meshes.get(&mesh_handle).unwrap();
            keyed_meshes
                .entry(mesh.key.clone())
                .or_default()
                .insert(mesh_handle);
        }

        // Generate vertex, index, and indirect data for each batch
        instance_meta.mesh_batches = keyed_meshes
            .into_iter()
            .map(|(key, meshes)| {
                let vertex_data = meshes
                    .iter()
                    .flat_map(|mesh| {
                        let mesh = render_meshes.get(mesh).unwrap();
                        mesh.vertex_buffer_data.iter().copied()
                    })
                    .collect::<Vec<_>>();

                let mut base_index = 0;
                let index_data = meshes.iter().fold(None, |acc, mesh| {
                    let mesh = render_meshes.get(mesh).unwrap();

                    let out = match &mesh.index_buffer_data {
                        GpuIndexBufferData::Indexed {
                            indices,
                            index_count,
                            index_format,
                        } => Some(match acc {
                            Some(GpuIndexBufferData::Indexed {
                                indices: acc_indices,
                                index_count: acc_index_count,
                                ..
                            }) => GpuIndexBufferData::Indexed {
                                indices: match (acc_indices, indices) {
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

                                index_count: index_count + acc_index_count,
                                index_format: *index_format,
                            },
                            None => GpuIndexBufferData::Indexed {
                                indices: indices.clone(),
                                index_count: *index_count,
                                index_format: *index_format,
                            },
                            _ => panic!("Mismatched GpuIndexBufferData"),
                        }),
                        GpuIndexBufferData::NonIndexed { vertex_count } => Some(match acc {
                            Some(GpuIndexBufferData::NonIndexed {
                                vertex_count: acc_vertex_count,
                            }) => GpuIndexBufferData::NonIndexed {
                                vertex_count: vertex_count + acc_vertex_count,
                            },
                            None => GpuIndexBufferData::NonIndexed {
                                vertex_count: *vertex_count,
                            },
                            _ => panic!("Mismatched GpuIndexBufferData"),
                        }),
                    };

                    base_index += mesh.vertex_count;

                    out
                });

                let mut base_index = 0u32;
                let indirect_data = meshes
                    .iter()
                    .map(
                        |mesh| match &render_meshes.get(mesh).unwrap().index_buffer_data {
                            GpuIndexBufferData::Indexed { index_count, .. } => {
                                base_index += index_count;

                                DrawIndirectVariant::Indexed(DrawIndexedIndirect {
                                    index_count: *index_count,
                                    ..default()
                                })
                            }
                            GpuIndexBufferData::NonIndexed { vertex_count } => {
                                base_index += vertex_count;

                                DrawIndirectVariant::NonIndexed(DrawIndirect {
                                    vertex_count: *vertex_count,
                                    ..default()
                                })
                            }
                        },
                    )
                    .collect::<Vec<_>>();

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
            .collect();

        debug!("Mesh batches:");
        for (key, batch) in instance_meta.mesh_batches.iter() {
            debug!("Key: {key:#?}");
            debug!("Meshes: {:#?}", batch.meshes);
            debug!("Indirect Data: {:#?}", batch.indirect_data);
        }
    }
}
