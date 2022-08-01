@compute
@workgroup_size(64)
fn instances(@builtin(global_invocation_id) invocation_id: vec3<u32>) {}
