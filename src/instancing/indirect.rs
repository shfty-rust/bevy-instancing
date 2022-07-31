use bytemuck::{Pod, Zeroable};

/// The structure expected in `indirect_buffer` for [`RenderEncoder::draw_indirect`](crate::util::RenderEncoder::draw_indirect).
#[derive(Debug, Default, Copy, Clone, Pod, Zeroable)]
#[repr(C)]
pub struct DrawIndirect {
    /// The number of vertices to draw.
    pub vertex_count: u32,
    /// The number of instances to draw.
    pub instance_count: u32,
    /// The Index of the first vertex to draw.
    pub base_vertex: u32,
    /// The instance ID of the first instance to draw.
    /// Has to be 0, unless [`Features::INDIRECT_FIRST_INSTANCE`](crate::Features::INDIRECT_FIRST_INSTANCE) is enabled.
    pub base_instance: u32,
}

/// The structure expected in `indirect_buffer` for [`RenderEncoder::draw_indexed_indirect`](crate::util::RenderEncoder::draw_indexed_indirect).
#[repr(C)]
#[derive(Debug, Default, Copy, Clone, Pod, Zeroable)]
pub struct DrawIndexedIndirect {
    /// The number of vertices to draw.
    pub vertex_count: u32,
    /// The number of instances to draw.
    pub instance_count: u32,
    /// The base index within the index buffer.
    pub base_index: u32,
    /// The value added to the vertex index before indexing into the vertex buffer.
    pub vertex_offset: i32,
    /// The instance ID of the first instance to draw.
    /// Has to be 0, unless [`Features::INDIRECT_FIRST_INSTANCE`](crate::Features::INDIRECT_FIRST_INSTANCE) is enabled.
    pub base_instance: u32,
}
