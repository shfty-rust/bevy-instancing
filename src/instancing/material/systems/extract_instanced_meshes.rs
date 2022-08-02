use bevy::{
    prelude::{
        AssetEvent, Assets, EventReader, Mesh, Res, ResMut,
    },
    render::Extract,
    utils::HashSet,
};

use crate::instancing::material::plugin::{
    GpuIndexBufferData, GpuInstancedMesh, RenderMeshes, InstancedMeshKey,
};

pub fn system(
    mut events: Extract<EventReader<AssetEvent<Mesh>>>,
    mut render_meshes: ResMut<RenderMeshes>,
    assets: Extract<Res<Assets<Mesh>>>,
) {
    let mut changed_assets = HashSet::default();
    let mut removed = Vec::new();
    for event in events.iter() {
        match event {
            AssetEvent::Created { handle } | AssetEvent::Modified { handle } => {
                changed_assets.insert(handle.clone_weak());
            }
            AssetEvent::Removed { handle } => {
                changed_assets.remove(handle);
                removed.push(handle.clone_weak());
            }
        }
    }

    let mut extracted_assets = Vec::new();
    for handle in changed_assets.drain() {
        if let Some(mesh) = assets.get(&handle) {
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

            extracted_assets.push((
                handle,
                GpuInstancedMesh {
                    key,
                    vertex_buffer_data,
                    vertex_count,
                    index_buffer_data,
                    primitive_topology: mesh.primitive_topology(),
                    layout: mesh_vertex_buffer_layout,
                },
            ))
        }
    }

    for removed in removed {
        render_meshes.remove(&removed);
    }

    for (handle, mesh) in extracted_assets {
        render_meshes.insert(handle, mesh);
    }
}

