pub mod mesh_instance_bundle;

use bevy::{
    ecs::{query::ROQueryItem, system::lifetimeless::Read},
    math::Mat4,
    prelude::{
        default, Commands, Component, ComputedVisibility, Entity, GlobalTransform, Handle, Mesh,
        Query,
    },
    render::{render_resource::ShaderType, Extract},
};
use crate::prelude::Instance;

use super::material::material_instanced::MaterialInstanced;

#[derive(Debug, Default, Clone, PartialEq, Component)]
pub struct MeshInstance {
    pub mesh: Handle<Mesh>,
    pub transform: Mat4,
}

#[derive(Debug, Copy, Clone, ShaderType, Component)]
#[repr(C)]
pub struct GpuMeshInstance {
    #[size(4)]
    #[align(16)]
    pub mesh: u32,
    #[size(64)]
    #[align(16)]
    pub transform: Mat4,
    #[size(64)]
    #[align(16)]
    pub inverse_transpose_model: Mat4,
}

impl PartialEq for GpuMeshInstance {
    fn eq(&self, other: &Self) -> bool {
        self.mesh == other.mesh
    }
}

impl Eq for GpuMeshInstance {}

impl PartialOrd for GpuMeshInstance {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        self.mesh.partial_cmp(&other.mesh)
    }
}

impl Ord for GpuMeshInstance {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        self.mesh.cmp(&other.mesh)
    }
}

impl Default for GpuMeshInstance {
    fn default() -> Self {
        Self {
            mesh: default(),
            transform: Mat4::ZERO,
            inverse_transpose_model: Mat4::ZERO,
        }
    }
}

impl Instance for MeshInstance {
    type ExtractedInstance = Self;
    type PreparedInstance = GpuMeshInstance;

    type Query = (
        Read<Handle<Mesh>>,
        Read<GlobalTransform>,
        Read<ComputedVisibility>,
    );

    fn extract_instance<'w>(
        (mesh, transform, visibility): ROQueryItem<Self::Query>,
    ) -> Self::ExtractedInstance {
        let transform = if visibility.is_visible() {
            transform.compute_matrix()
        } else {
            Mat4::ZERO
        };

        MeshInstance {
            mesh: mesh.clone_weak(),
            transform,
        }
    }

    fn prepare_instance(instance: &Self::ExtractedInstance, mesh: u32) -> Self::PreparedInstance {
        GpuMeshInstance {
            mesh,
            transform: instance.transform,
            inverse_transpose_model: instance.transform.inverse().transpose(),
            ..default()
        }
    }

    fn transform(instance: &Self::ExtractedInstance) -> Mat4 {
        instance.transform
    }
}

pub fn extract_mesh_instances<M: MaterialInstanced>(
    query_mesh_instance: Extract<Query<(Entity, <M::Instance as Instance>::Query)>>,
    mut commands: Commands,
) {
    for (entity, item) in query_mesh_instance.iter() {
        commands
            .insert_or_spawn_batch([(entity, (<M::Instance as Instance>::extract_instance(item),))])
    }
}
