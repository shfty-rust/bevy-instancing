use bevy::{
    prelude::{default, Commands, Res},
    render::{render_resource::BindGroup, renderer::RenderDevice},
};

use crate::prelude::IndirectComputePipelines;

/// The collection of bind groups and other data necessary to compute one set of instance data
pub struct IndirectComputeJob {
    pub indirect_offsets: BindGroup,
    pub sort_instances: BindGroup,
    pub mesh_count: u32,
    pub instance_count: u32,
}

/// Resource containing pending [IndirectComputeJob]
pub struct IndirectComputeQueue(pub Vec<IndirectComputeJob>);

/// Creates [IndirectComputeJob]s from bind groups and pushes them into the [IndirectComputeQueue]
pub fn queue_compute_jobs(
    mut commands: Commands,
    pipeline: Res<IndirectComputePipelines>,
    render_device: Res<RenderDevice>,
    //query_instanced_material: Query<&GpuInstancedMaterial>,
) {
    /*
    let mut bind_groups_queue = vec![];

    for (i, instanced_material) in query_instanced_material.iter().enumerate() {
        if instanced_material.indirect_count == 0 || instanced_material.instance_count == 0 {
            continue;
        }

        let bind_group_counts_offsets = render_device.create_bind_group(&BindGroupDescriptor {
            label: None,
            layout: &pipeline.indirect_offsets.bind_group_layout,
            entries: &[
                BindGroupEntry {
                    binding: 0,
                    resource: instanced_material.instance_buffer_unsorted.as_entire_binding(),
                },
                BindGroupEntry {
                    binding: 1,
                    resource: instanced_material.indexed_indirect_buffer.as_entire_binding(),
                },
            ],
        });

        let bind_group_sort_instances = render_device.create_bind_group(&BindGroupDescriptor {
            label: None,
            layout: &pipeline.sort_instances.bind_group_layout,
            entries: &[
                BindGroupEntry {
                    binding: 0,
                    resource: instanced_material.instance_buffer_unsorted.as_entire_binding(),
                },
                BindGroupEntry {
                    binding: 1,
                    resource: instanced_material.indexed_indirect_buffer.as_entire_binding(),
                },
                BindGroupEntry {
                    binding: 2,
                    resource: instanced_material.instance_buffer_sorted.as_entire_binding(),
                },
            ],
        });

        bind_groups_queue.push(IndirectComputeJob {
            indirect_offsets: bind_group_counts_offsets,
            sort_instances: bind_group_sort_instances,
            mesh_count: instanced_material.indirect_count as u32,
            instance_count: instanced_material.instance_count as u32,
        });
    }

    debug!("Queueing {} compute jobs", bind_groups_queue.len());
    commands.insert_resource(IndirectComputeQueue(bind_groups_queue));
    */

    commands.insert_resource(IndirectComputeQueue(default()));
}
