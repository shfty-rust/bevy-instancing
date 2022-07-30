use bevy::{
    asset::load_internal_asset,
    core_pipeline::node::MAIN_PASS_DEPENDENCIES,
    prelude::{App, HandleUntyped, Plugin, Shader},
    reflect::TypeUuid,
    render::{render_graph::RenderGraph, RenderApp, RenderStage},
};

use bevy::asset as bevy_asset;

use crate::prelude::{queue_compute_jobs, IndirectComputeNode, IndirectComputePipelines};

pub struct IndirectComputePlugin;

pub const INDIRECT_OFFSETS_HANDLE: HandleUntyped =
    HandleUntyped::weak_from_u64(Shader::TYPE_UUID, 9845106354689849797);

pub const SORT_INSTANCES_HANDLE: HandleUntyped =
    HandleUntyped::weak_from_u64(Shader::TYPE_UUID, 5719622651740655916);

impl Plugin for IndirectComputePlugin {
    fn build(&self, app: &mut App) {
        load_internal_asset!(
            app,
            INDIRECT_OFFSETS_HANDLE,
            "shaders/indirect_offsets.wgsl",
            Shader::from_wgsl
        );

        load_internal_asset!(
            app,
            SORT_INSTANCES_HANDLE,
            "shaders/sort_instances.wgsl",
            Shader::from_wgsl
        );

        let render_app = app.sub_app_mut(RenderApp);
        render_app
            .init_resource::<IndirectComputePipelines>()
            .add_system_to_stage(RenderStage::Queue, queue_compute_jobs);

        let mut render_graph = render_app.world.resource_mut::<RenderGraph>();
        render_graph.add_node("indirect_compute", IndirectComputeNode::default());
        render_graph
            .add_node_edge("indirect_compute", MAIN_PASS_DEPENDENCIES)
            .unwrap();
    }
}
