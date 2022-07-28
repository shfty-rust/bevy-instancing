use bevy::reflect::{FromReflect, Reflect};
use bytemuck::{Pod, Zeroable};

#[derive(Debug, Default, Copy, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Reflect, FromReflect, Pod, Zeroable)]
#[repr(C)]
pub struct DrawIndexedIndirect {
    /// The number of indices to draw.
    pub index_count: u32,
    /// The number of instances to draw.
    pub instance_count: u32,
    /// The base index within the index buffer.
    pub first_index: u32,
    /// The value added to the vertex index before indexing into the vertex buffer.
    pub vertex_offset: i32,
    /// The instance ID of the first instance to draw.
    /// has to be 0, unless [`Features::INDIRECT_FIRST_INSTANCE`] is enabled.
    pub first_instance: u32,
}

