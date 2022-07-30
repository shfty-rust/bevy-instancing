#import indirect_instancing::instance_struct
#define_import_path indirect_instancing::color_instance_struct

struct ColorInstanceData {
    base: InstanceData;
    color: vec4<f32>;
};


struct ColorInstances {
    instances: array<ColorInstanceData>;
};

