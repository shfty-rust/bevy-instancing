pub mod color_instance_bundle;
pub mod mesh_instance_color;
pub mod plugin;

use bevy::{
    ecs::{system::lifetimeless::Read, query::ROQueryItem},
    math::{Mat4, Vec4},
    prelude::{default, Component}, render::render_resource::ShaderType, 
};
use crate::prelude::{GpuMeshInstance, Instance, InstanceColor, MeshInstance};

#[derive(Debug, Default, Clone, PartialEq, Component)]
pub struct ColorMeshInstance {
    pub base: MeshInstance,
    pub color: Vec4,
}

/// GPU-friendly data for a since mesh instance
#[derive(Debug, Copy, Clone, PartialEq, ShaderType, Component)]
pub struct GpuColorMeshInstance {
    #[size(144)]
    #[align(16)]
    pub base: GpuMeshInstance,
    #[size(16)]
    #[align(16)]
    pub color: Vec4,
}

impl Default for GpuColorMeshInstance {
    fn default() -> Self {
        Self {
            base: default(),
            color: Vec4::ZERO,
        }
    }
}

impl Instance for ColorMeshInstance {
    type ExtractedInstance = Self;
    type PreparedInstance = GpuColorMeshInstance;

    type Query = (<MeshInstance as Instance>::Query, Read<InstanceColor>);

    fn extract_instance<'w>(
        (base, color): ROQueryItem<Self::Query>,
    ) -> Self::ExtractedInstance {
        ColorMeshInstance {
            base: MeshInstance::extract_instance(base),
            color: Vec4::new(color.r(), color.g(), color.b(), color.a()),
        }
    }

    fn prepare_instance(instance: &Self::ExtractedInstance, mesh: u32) -> Self::PreparedInstance {
        GpuColorMeshInstance {
            base: MeshInstance::prepare_instance(&instance.base, mesh),
            color: instance.color,
        }
    }

    fn transform(instance: &Self::ExtractedInstance) -> Mat4 {
        instance.base.transform
    }
}
