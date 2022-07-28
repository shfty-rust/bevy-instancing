use bevy::{
    math::Mat4,
    prelude::{default, Component},
};
use bytemuck::{Pod, Zeroable};

#[derive(Debug, Copy, Clone, Pod, Zeroable, Component)]
#[repr(C)]
pub struct GpuMeshInstance {
    pub mesh: u32,
    pub _padding: [u32; 3],
    pub transform: Mat4,
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
            _padding: default(),
            transform: Mat4::ZERO,
            inverse_transpose_model: Mat4::ZERO,
        }
    }
}
