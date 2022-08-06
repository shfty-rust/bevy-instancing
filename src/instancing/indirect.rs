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

#[derive(Debug, Copy, Clone)]
pub enum IndirectDraw {
    Indexed(DrawIndexedIndirect),
    NonIndexed(DrawIndirect),
}

#[derive(Debug, Copy, Clone)]
pub enum DrawOffsets {
    Indexed { base_index: u32, vertex_offset: i32 },
    NonIndexed { base_vertex: u32 },
}

/// A type representing a wgpu draw call
pub trait DrawCall {
    fn vertex_count(&self) -> u32;
    fn instance_count(&self) -> u32;
    fn base_instance(&self) -> u32;
    fn offsets(&self) -> DrawOffsets;

    fn set_vertex_count(&mut self, vertex_count: u32);
    fn set_instance_count(&mut self, instance_count: u32);
    fn set_base_instance(&mut self, base_instance: u32);
    fn set_offsets(&mut self, draw_offsets: DrawOffsets);
}

impl DrawCall for DrawIndirect {
    fn vertex_count(&self) -> u32 {
        self.vertex_count
    }

    fn instance_count(&self) -> u32 {
        self.instance_count
    }

    fn base_instance(&self) -> u32 {
        self.base_instance
    }

    fn offsets(&self) -> DrawOffsets {
        DrawOffsets::NonIndexed {
            base_vertex: self.base_vertex,
        }
    }

    fn set_vertex_count(&mut self, vertex_count: u32) {
        self.vertex_count = vertex_count
    }

    fn set_instance_count(&mut self, instance_count: u32) {
        self.instance_count = instance_count
    }

    fn set_base_instance(&mut self, base_instance: u32) {
        self.base_instance = base_instance
    }

    fn set_offsets(&mut self, draw_offsets: DrawOffsets) {
        match draw_offsets {
            DrawOffsets::NonIndexed { base_vertex } => self.base_vertex = base_vertex,
            _ => panic!("Mismatched DrawOffsets variant"),
        }
    }
}

impl DrawCall for DrawIndexedIndirect {
    fn vertex_count(&self) -> u32 {
        self.vertex_count
    }

    fn instance_count(&self) -> u32 {
        self.instance_count
    }

    fn base_instance(&self) -> u32 {
        self.base_instance
    }

    fn offsets(&self) -> DrawOffsets {
        DrawOffsets::Indexed {
            base_index: self.base_index,
            vertex_offset: self.vertex_offset,
        }
    }

    fn set_vertex_count(&mut self, vertex_count: u32) {
        self.vertex_count = vertex_count
    }

    fn set_instance_count(&mut self, instance_count: u32) {
        self.instance_count = instance_count
    }

    fn set_base_instance(&mut self, base_instance: u32) {
        self.base_instance = base_instance
    }

    fn set_offsets(&mut self, draw_offsets: DrawOffsets) {
        match draw_offsets {
            DrawOffsets::Indexed {
                base_index,
                vertex_offset,
            } => {
                self.base_index = base_index;
                self.vertex_offset = vertex_offset;
            }
            _ => panic!("Mismatched DrawOffsets variant"),
        }
    }
}

impl DrawCall for IndirectDraw {
    fn vertex_count(&self) -> u32 {
        match self {
            IndirectDraw::Indexed(draw) => draw.vertex_count(),
            IndirectDraw::NonIndexed(draw) => draw.vertex_count(),
        }
    }

    fn instance_count(&self) -> u32 {
        match self {
            IndirectDraw::Indexed(draw) => draw.instance_count(),
            IndirectDraw::NonIndexed(draw) => draw.instance_count(),
        }
    }

    fn base_instance(&self) -> u32 {
        match self {
            IndirectDraw::Indexed(draw) => draw.base_instance(),
            IndirectDraw::NonIndexed(draw) => draw.base_instance(),
        }
    }

    fn offsets(&self) -> DrawOffsets {
        match self {
            IndirectDraw::Indexed(draw) => draw.offsets(),
            IndirectDraw::NonIndexed(draw) => draw.offsets(),
        }
    }

    fn set_vertex_count(&mut self, vertex_count: u32) {
        match self {
            IndirectDraw::Indexed(draw) => draw.set_vertex_count(vertex_count),
            IndirectDraw::NonIndexed(draw) => draw.set_vertex_count(vertex_count),
        }
    }

    fn set_instance_count(&mut self, instance_count: u32) {
        match self {
            IndirectDraw::Indexed(draw) => draw.set_instance_count(instance_count),
            IndirectDraw::NonIndexed(draw) => draw.set_instance_count(instance_count),
        }
    }

    fn set_base_instance(&mut self, base_instance: u32) {
        match self {
            IndirectDraw::Indexed(draw) => draw.set_base_instance(base_instance),
            IndirectDraw::NonIndexed(draw) => draw.set_base_instance(base_instance),
        }
    }

    fn set_offsets(&mut self, draw_offsets: DrawOffsets) {
        match self {
            IndirectDraw::Indexed(draw) => draw.set_offsets(draw_offsets),
            IndirectDraw::NonIndexed(draw) => draw.set_offsets(draw_offsets),
        }
    }
}
