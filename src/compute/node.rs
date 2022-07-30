use bevy::{
    prelude::{World, debug},
    render::{
        render_graph,
        render_resource::{ComputePassDescriptor, PipelineCache},
        renderer::RenderContext,
    },
};

use crate::prelude::{IndirectComputePipelines, IndirectComputeQueue};

const WORKGROUP_SIZE: u32 = 64;

#[derive(Default)]
pub struct IndirectComputeNode;

impl render_graph::Node for IndirectComputeNode {
    fn run(
        &self,
        _graph: &mut render_graph::RenderGraphContext,
        render_context: &mut RenderContext,
        world: &World,
    ) -> Result<(), render_graph::NodeRunError> {
        let pipeline_cache = world.resource::<PipelineCache>();
        let pipelines = world.resource::<IndirectComputePipelines>();

        if let (Some(pipeline_indirect_offsets), Some(pipeline_sort_instances)) = (
            pipeline_cache.get_compute_pipeline(pipelines.indirect_offsets.pipeline),
            pipeline_cache.get_compute_pipeline(pipelines.sort_instances.pipeline),
        ) {
            let bind_groups = &world.resource::<IndirectComputeQueue>().0;
            for bind_group in bind_groups {
                debug!("Running compute job with {} instances", bind_group.instance_count);

                let mut pass = render_context
                    .command_encoder
                    .begin_compute_pass(&ComputePassDescriptor::default());

                let instance_workgroups = (bind_group.instance_count / WORKGROUP_SIZE).max(1);

                pass.set_bind_group(0, &bind_group.indirect_offsets, &[]);
                pass.set_pipeline(pipeline_indirect_offsets);
                pass.dispatch(instance_workgroups, 1, 1);

                pass.set_bind_group(0, &bind_group.sort_instances, &[]);
                pass.set_pipeline(pipeline_sort_instances);
                pass.dispatch(instance_workgroups, 1, 1);
            }
        }

        Ok(())
    }
}
