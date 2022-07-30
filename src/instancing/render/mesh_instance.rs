use bevy::{
    ecs::system::lifetimeless::Read,
    math::{Mat4, Vec3},
    prelude::{default, Component, ComputedVisibility, GlobalTransform, Handle, Mesh},
};

use crate::prelude::{Instance, ReadOnlyQueryItem, GpuMeshInstance};

#[derive(Debug, Default, Clone, PartialEq, Component)]
pub struct MeshInstance {
    pub mesh: Handle<Mesh>,
    pub transform: Mat4,
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
        (mesh, transform, visibility): ReadOnlyQueryItem<Self::Query>,
    ) -> Self::ExtractedInstance {
        let transform = if visibility.is_visible {
            *transform
        } else {
            transform.with_scale(Vec3::ZERO)
        }
        .compute_matrix();

        MeshInstance {
            mesh: mesh.clone_weak(),
            transform,
        }
    }

    fn prepare_instance(
        instance: &Self::ExtractedInstance,
        mesh: u32,
    ) -> Self::PreparedInstance {
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

