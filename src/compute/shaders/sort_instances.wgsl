#import indirect_instancing::instance_struct
#import indirect_instancing::indirect_struct

@group(0), binding(0)
var<storage, read> in_instances: Instances;

@group(0), binding(1)
var<storage, read_write> out_indirects: IndirectDrawCommands;

@group(0), binding(2)
var<storage, read_write> out_instances: Instances;


@compute
@workgroup_size(64)
fn sort_instances([[builtin(global_invocation_id)]] invocation_id: vec3<u32>) {
    // Destructure instance index
    let instance_idx = invocation_id.x;

    // Calculate maximum instance index
    let max_instance = arrayLength(&in_instances.instances);

    // Early out if outside instance buffer bounds
    if (instance_idx >= max_instance) {
        return;
    }

    // Fetch indirect and instance for this invocation
    let instance = in_instances.instances[instance_idx];
    let mesh_idx = instance.mesh;

    // Fetch the first index for this mesh
    let first_instance = atomicLoad(&out_indirects.indirects[mesh_idx].first_instance);

    // Increment the total instance count for this mesh
    let instance_count = atomicAdd(&out_indirects.indirects[mesh_idx].instance_count, 1u);

    // Write to the instance buffer
    out_instances.instances[first_instance + instance_count] = instance;
}
