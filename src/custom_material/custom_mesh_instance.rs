use bevy::{
    ecs::system::lifetimeless::Read,
    math::{Mat4, Vec4},
    prelude::{default, Commands, Component, Entity, Query},
};
use bytemuck::{Pod, Zeroable};

use crate::prelude::{
    GpuMeshInstance, Instance, MeshInstance, MeshInstanceColor, ReadOnlyQueryItem,
    SpecializedInstancedMaterial,
};

#[derive(Debug, Default, Clone, PartialEq, Component)]
pub struct CustomMeshInstance {
    pub base: MeshInstance,
    pub color: Vec4,
}

/// GPU-friendly data for a since mesh instance
#[derive(Debug, Copy, Clone, PartialEq, Pod, Zeroable, Component)]
#[repr(C)]
pub struct GpuCustomMeshInstance {
    pub base: GpuMeshInstance,
    pub color: Vec4,
}

impl Default for GpuCustomMeshInstance {
    fn default() -> Self {
        Self {
            base: default(),
            color: Vec4::ZERO,
        }
    }
}

impl Instance for CustomMeshInstance {
    type ExtractedInstance = Self;
    type PreparedInstance = GpuCustomMeshInstance;

    type Query = (<MeshInstance as Instance>::Query, Read<MeshInstanceColor>);

    fn extract_instance<'w>(
        (base, color): ReadOnlyQueryItem<Self::Query>,
    ) -> Self::ExtractedInstance {
        CustomMeshInstance {
            base: MeshInstance::extract_instance(base),
            color: Vec4::new(color.r(), color.g(), color.b(), color.a()),
        }
    }

    fn prepare_instance(instance: &Self::ExtractedInstance, mesh: u32) -> Self::PreparedInstance {
        GpuCustomMeshInstance {
            base: MeshInstance::prepare_instance(&instance.base, mesh),
            color: instance.color,
        }
    }

    fn transform(instance: &Self::ExtractedInstance) -> Mat4 {
        instance.base.transform
    }
}

pub fn extract_mesh_instances<M: SpecializedInstancedMaterial>(
    query_mesh_instance: Query<(Entity, <M::Instance as Instance>::Query)>,
    mut commands: Commands,
) {
    for (entity, item) in query_mesh_instance.iter() {
        commands
            .insert_or_spawn_batch([(entity, (<M::Instance as Instance>::extract_instance(item),))])
    }
}
