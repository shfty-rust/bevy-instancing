#import indirect_instancing::instance_struct
#import indirect_instancing::indirect_struct
#import indirect_instancing::color_instance_struct

struct UniformData {
    time: f32;
    _: f32;
    _: f32;
    _: f32;
    normal: vec3<f32>;
    _: f32;
    tangent: vec3<f32>;
    _: f32;
    tint: vec3<f32>;
    _: f32;
};

[[group(0), binding(0)]]
var<uniform> in_uniform: UniformData;

[[group(0), binding(1)]]
var<storage, read_write> out_instances: ColorInstances;

[[stage(compute), workgroup_size(64)]]
fn instances([[builtin(global_invocation_id)]] invocation_id: vec3<u32>) {
    // Calculate maximum indices
    let max_instance = arrayLength(&out_instances.instances);

    // Destructure invocation index
    let instance_idx = invocation_id.x;

    // Early-out if we're out of bounds
    if (instance_idx >= max_instance) {
        return;
    }

    let f = f32(instance_idx) / f32(max_instance);

    let frequency = f * 3.141 * 0.1;
    let amplitude = f * 3.141 * 0.5;
    let scale = f * 25.0;

    let fac = sin(in_uniform.time * frequency) * amplitude;

    let pos = ((in_uniform.normal * sin(fac)) + (in_uniform.tangent * cos(fac))) * scale;

    // Write instance transform
    out_instances.instances[instance_idx].base.transform = mat4x4<f32>(
        vec4<f32>(1.0, 0.0, 0.0, 0.0),
        vec4<f32>(0.0, 1.0, 0.0, 0.0),
        vec4<f32>(0.0, 0.0, 1.0, 0.0),
        vec4<f32>(pos, 1.0),
    );
    out_instances.instances[instance_idx].color = vec4<f32>(in_uniform.tint, abs(f));
}
