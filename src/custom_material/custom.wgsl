#import bevy_pbr::mesh_view_bind_group
#import bevy_pbr::mesh_struct
#import indirect_instancing::instance_struct

struct CustomInstanceData {
    base: InstanceData;
    color: vec4<f32>;
};

struct CustomInstances {
    instances: array<CustomInstanceData>;
};


[[group(2), binding(0)]]
var<storage> instances: CustomInstances;

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
    [[location(3)]] color: vec4<f32>;
};

[[stage(vertex)]]
fn vertex(in: VertexInput) -> VertexOutput {
    let instance = instances.instances[in.instance];

    var out: VertexOutput;
    out.world_position = instance.base.transform * vec4<f32>(in.vertex, 1.0);
    out.clip_position = view.view_proj * out.world_position;
    out.vertex = in.vertex;
    out.normal = in.normal;
    out.color = instance.color;
    return out;
}

let margin_size = 0.066;
let grad_size = 0.066;

fn luminance(v: vec3<f32>) -> f32 {
    return dot(v, vec3<f32>(0.2126, 0.7152, 0.0722));
}

[[stage(fragment)]]
fn fragment(in: VertexOutput) -> [[location(0)]] vec4<f32> {
    let grad_size = fwidth(in.world_position.xyz);
    let margin_max = 0.5 - margin_size;
    let margin_min = -margin_max;
    let stripe_axis = 
            smoothStep(vec3<f32>(margin_min), vec3<f32>(margin_min) + grad_size, in.vertex) *
            smoothStep(vec3<f32>(margin_max), vec3<f32>(margin_max) - grad_size, in.vertex);
    let stripe_fac = max(stripe_axis.x + stripe_axis.y + stripe_axis.z, 1.0);
    let stripe = mix(0.0, 1.0, stripe_fac);

    let height_fac = in.vertex.y + 0.5;
    let grad = mix(0.33, 1.0, height_fac);

    let diagonal_fac = 1.0 - abs(dot(in.vertex.xz, vec2<f32>(1.0)));
    let diagonal_fac = max(diagonal_fac, 1.0 - in.normal.y);
    let diagonal_fac = mix(0.33, 1.0, diagonal_fac);

    let directional_light = lights.directional_lights[0];
    let directional_fac = dot(in.normal, directional_light.direction_to_light);
    let directional_color = directional_light.color * directional_fac;

    let ambient = 0.3;
    let maximum = 0.6;

    let color = in.color.xyz * grad * stripe * diagonal_fac * clamp(
        directional_color.xyz,
        vec3<f32>(ambient),
        vec3<f32>(maximum),
    );
    let color = color * luminance(color.xyz);

    return vec4<f32>(color, in.color.a);
}
