#define_import_path indirect_instancing::instance_struct

struct InstanceData {
    @size(4) @align(16)
    mesh: u32,
    @size(64) @align(16)
    transform: mat4x4<f32>,
    @size(64) @align(16)
    inverse_transpose_model: mat4x4<f32>,
};


#ifdef NO_STORAGE_BUFFERS_SUPPORT
struct Instances {
    instances: array<InstanceData, 112>,
};
#else
struct Instances {
    instances: array<InstanceData>,
};
#endif

