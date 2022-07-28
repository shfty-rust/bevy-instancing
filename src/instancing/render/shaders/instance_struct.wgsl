#define_import_path indirect_instancing::instance_struct

struct InstanceData {
    mesh: u32;
    _: u32;
    _: u32;
    _: u32;
    transform: mat4x4<f32>;
    inverse_transpose_model: mat4x4<f32>;
};


struct Instances {
    instances: array<InstanceData>;
};

