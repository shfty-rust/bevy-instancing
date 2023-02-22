use bevy::{
    asset::load_internal_asset,
    prelude::{App, HandleUntyped, IntoSystemDescriptor, Plugin, Shader},
    reflect::TypeUuid,
    render::{
        extract_component::ExtractComponentPlugin, render_asset::PrepareAssetLabel, RenderApp,
        RenderStage,
    },
};

use crate::{
    instancing::material::systems::prepare_mesh_batches::{self, MeshBatches},
    prelude::{InstanceSlice, InstancedMeshPipeline},
};

pub const INSTANCED_MESH_SHADER_HANDLE: HandleUntyped =
    HandleUntyped::weak_from_u64(Shader::TYPE_UUID, 7051817732463169032);

pub const INSTANCE_STRUCT_HANDLE: HandleUntyped =
    HandleUntyped::weak_from_u64(Shader::TYPE_UUID, 14563515845427599203);

pub const INDIRECT_STRUCT_HANDLE: HandleUntyped =
    HandleUntyped::weak_from_u64(Shader::TYPE_UUID, 7281773422344927676);

/// Plugin encapsulating instanced mesh rendering
#[derive(Debug, Default, Copy, Clone)]
pub struct IndirectRenderingPlugin;

impl Plugin for IndirectRenderingPlugin {
    fn build(&self, app: &mut App) {
        load_internal_asset!(
            app,
            INSTANCED_MESH_SHADER_HANDLE,
            "render/shaders/instanced_mesh.wgsl",
            Shader::from_wgsl
        );

        load_internal_asset!(
            app,
            INSTANCE_STRUCT_HANDLE,
            "render/shaders/instance_struct.wgsl",
            Shader::from_wgsl
        );

        load_internal_asset!(
            app,
            INDIRECT_STRUCT_HANDLE,
            "render/shaders/indirect_struct.wgsl",
            Shader::from_wgsl
        );

        app.register_type::<InstanceSlice>();

        app.add_plugin(ExtractComponentPlugin::<InstanceSlice>::default());

        app.sub_app_mut(RenderApp)
            .init_resource::<InstancedMeshPipeline>()
            .init_resource::<MeshBatches>()
            .add_system_to_stage(
                RenderStage::Prepare,
                prepare_mesh_batches::system.after(PrepareAssetLabel::AssetPrepare),
            );
    }
}
