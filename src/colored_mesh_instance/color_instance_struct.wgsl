#import indirect_instancing::instance_struct
#define_import_path indirect_instancing::color_instance_struct

struct ColorInstanceData {
    @size(144)
    base: InstanceData,
    @size(16)
    color: vec4<f32>,
};

#ifdef NO_STORAGE_BUFFERS_SUPPORT
struct ColorInstances {
    instances: array<ColorInstanceData, 102>,
};
#else
struct ColorInstances {
    instances: array<ColorInstanceData>,
};
#endif

