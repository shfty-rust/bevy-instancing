#define_import_path indirect_instancing::indirect_struct

struct DrawIndirect {
    vertex_count: u32;
    instance_count:  atomic<u32>;
    first_vertex: u32;
    first_instance: atomic<u32>;
};

struct IndirectDrawCommands {
    indirects: array<DrawIndirect>;
};

