#import bevy_pbr::mesh_view_bind_group
#import bevy_pbr::mesh_struct
#import indirect_instancing::instance_struct
#import indirect_instancing::color_instance_struct

[[group(1), binding(0)]]
var in_texture: texture_2d<f32>;

[[group(1), binding(1)]]
var in_sampler: sampler;

[[group(2), binding(0)]]
var<storage> in_instances: ColorInstances;

struct VertexInput {
    [[builtin(instance_index)]] instance: u32;
    [[location(0)]] vertex: vec3<f32>;
    [[location(1)]] normal: vec3<f32>;
    [[location(2)]] uv: vec2<f32>;
};

struct VertexOutput {
    [[builtin(position)]] clip_position: vec4<f32>;
    [[location(0)]] world_position: vec4<f32>;
    [[location(1)]] vertex: vec3<f32>;
    [[location(2)]] normal: vec3<f32>;
    [[location(3)]] uv: vec2<f32>;
    [[location(4)]] color: vec4<f32>;
};

[[stage(vertex)]]
fn vertex(in: VertexInput) -> VertexOutput {
    let instance = in_instances.instances[in.instance];

    var out: VertexOutput;
    out.world_position = instance.base.transform * vec4<f32>(in.vertex, 1.0);
    out.clip_position = view.view_proj * out.world_position;
    out.vertex = in.vertex;
    out.normal = in.normal;
    out.uv = in.uv;
    out.color = instance.color;
    return out;
}

fn luminance(v: vec3<f32>) -> f32 {
    return dot(v, vec3<f32>(0.2126, 0.7152, 0.0722));
}

[[stage(fragment)]]
fn fragment(in: VertexOutput) -> [[location(0)]] vec4<f32> {
    let directional_light = lights.directional_lights[0];
    let directional_fac = dot(in.normal, directional_light.direction_to_light);
    let directional_color = directional_light.color * directional_fac;

    let ambient = 0.3;
    let maximum = 0.6;

    let tex = textureSample(in_texture, in_sampler, in.uv);

    let tint = in.color.xyz * clamp(
        directional_color.xyz,
        vec3<f32>(ambient),
        vec3<f32>(maximum),
    );

    let color = tex.rgb * tint * directional_color.xyz;

    return vec4<f32>(color, in.color.a);
}
