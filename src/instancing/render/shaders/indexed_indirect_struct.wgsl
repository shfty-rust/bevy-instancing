#define_import_path indirect_instancing::indexed_indirect_struct

struct DrawIndexedIndirect {
    vertex_count: u32;
    instance_count:  atomic<u32>;
    first_index: u32;
    vertex_offset: i32;
    first_instance: atomic<u32>;
};

struct IndexedIndirectDrawCommands {
    indexed_indirects: array<DrawIndexedIndirect>;
};

