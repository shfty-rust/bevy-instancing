use std::collections::{BTreeMap, BTreeSet};

use bevy::{
    prelude::{debug, default, Assets, Commands, Handle, Mesh, Query, Res, With, info},
    render::Extract,
};

use crate::instancing::material::{
    plugin::{GpuIndexBufferData, GpuInstancedMesh, GpuInstancedMeshes, InstancedMeshKey},
    specialized_instanced_material::SpecializedInstancedMaterial,
};

pub fn system<M: SpecializedInstancedMaterial>(
    meshes: Extract<Res<Assets<Mesh>>>,
    query_instance: Extract<Query<&Handle<Mesh>, With<Handle<M>>>>,
    mut commands: Commands,
) {
    debug!("{}", std::any::type_name::<M>());
    let mut instanced_meshes = BTreeMap::new();

    for mesh_handle in query_instance.iter().collect::<BTreeSet<_>>() {
        debug!("Mesh {mesh_handle:?}");
        let mesh = meshes.get(mesh_handle).unwrap();
        let vertex_buffer_data = mesh.get_vertex_buffer_data();
        let vertex_count = mesh.count_vertices();

        let index_buffer_data = mesh.indices().map_or(
            GpuIndexBufferData::NonIndexed {
                vertex_count: vertex_count as u32,
            },
            |indices| -> GpuIndexBufferData {
                GpuIndexBufferData::Indexed {
                    indices: indices.clone(),
                    index_count: mesh.indices().unwrap().len() as u32,
                    index_format: mesh.indices().unwrap().into(),
                }
            },
        );

        let mesh_vertex_buffer_layout = mesh.get_mesh_vertex_buffer_layout();

        let primitive_topology = mesh.primitive_topology();

        let key = InstancedMeshKey {
            primitive_topology,
            layout: mesh_vertex_buffer_layout.clone(),
            index_format: match index_buffer_data {
                GpuIndexBufferData::Indexed { index_format, .. } => Some(index_format),
                GpuIndexBufferData::NonIndexed { .. } => None,
            },
        };

        instanced_meshes.insert(
            mesh_handle.clone_weak(),
            GpuInstancedMesh {
                key,
                vertex_buffer_data,
                vertex_count,
                index_buffer_data,
                primitive_topology: mesh.primitive_topology(),
                layout: mesh_vertex_buffer_layout,
            },
        );
    }

    commands.insert_resource(GpuInstancedMeshes::<M> {
        instanced_meshes,
        ..default()
    })
}

