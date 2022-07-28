use bevy::reflect::{FromReflect, Reflect};
use bytemuck::{Pod, Zeroable};

#[derive(Debug, Default, Copy, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Reflect, FromReflect, Pod, Zeroable)]
#[repr(C)]
pub struct DrawIndirect {
    // The number of vertices to draw.
    pub vertex_count: u32,
    // The number of instances to draw.
    pub instance_count: u32,
    // The Index of the first vertex to draw.
    pub first_vertex: u32,
    // The instance ID of the first instance to draw.
    // has to be 0, unless [`Features::INDIRECT_FIRST_INSTANCE`] is enabled.
    pub first_instance: u32,
}

