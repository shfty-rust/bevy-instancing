#define_import_path indirect_instancing::instance_struct

struct InstanceData {
    mesh: u32,
    _: u32,
    _: u32,
    _: u32,
    transform: mat4x4<f32>,
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

