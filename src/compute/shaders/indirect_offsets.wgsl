#import indirect_instancing::instance_struct
#import indirect_instancing::indirect_struct

@group(0)
@binding(0)
var<storage, read> in_instances: Instances;

@group(0)
@binding(1)
var<storage, read_write> out_indirects: IndirectDrawCommands;

@compute
@workgroup_size(64)
fn indirect_offsets(@builtin(global_invocation_id) invocation_id: vec3<u32>) {
    // Calculate maximum indices
    let max_instance = arrayLength(&in_instances.instances);
    let max_mesh = arrayLength(&out_indirects.indirects);

    // Destructure invocation index
    let instance_idx = invocation_id.x;

    // Early-out if we're out of bounds
    if (instance_idx >= max_instance) {
        return;
    }

    // Fetch instance for this invocation
    let instance = in_instances.instances[instance_idx];
    let mesh_idx = instance.mesh;

    // Early-out if this is the last mesh
    if (mesh_idx == max_mesh - 1u) {
        return;
    }

    // Increment the instance offset for all subsequent meshes
    for (var i = mesh_idx + 1u; i < max_mesh; i = i + 1u) {
        atomicAdd(&out_indirects.indirects[i].first_instance, 1u);
    }
}
