use bevy::asset::{Asset, AssetServer, Handle};

use bevy::pbr::AlphaMode;
use bevy::render::{
    mesh::MeshVertexBufferLayout,
    render_asset::RenderAsset,
    render_resource::{
        BindGroup, BindGroupLayout, RenderPipelineDescriptor, Shader,
        SpecializedMeshPipelineError,
    },
    renderer::RenderDevice,
};

use crate::prelude::{InstancedMaterialPipeline, Instance};

/// Materials are used alongside [`InstancedMaterialPlugin`] and [`InstancedMaterialMeshBundle`](crate::InstancedMaterialMeshBundle)
/// to spawn entities that are rendered with a specific [`Material`] type. They serve as an easy to use high level
/// way to render [`Mesh`] entities with custom shader logic. For materials that can specialize their [`RenderPipelineDescriptor`]
/// based on specific material values, see [`SpecializedInstanceMaterial`]. [`InstanceMaterial`] automatically implements [`SpecializedInstanceMaterial`]
/// and can be used anywhere that type is used (such as [`InstancedMaterialPlugin`]).
pub trait InstancedMaterial: Asset + RenderAsset + Sized {
    type Instance: Instance;

    /// Returns this material's [`BindGroup`]. This should match the layout returned by [`InstancedMaterial::bind_group_layout`].
    fn bind_group(material: &<Self as RenderAsset>::PreparedAsset) -> &BindGroup;

    /// Returns this material's [`BindGroupLayout`]. This should match the [`BindGroup`] returned by [`InstancedMaterial::bind_group`].
    fn bind_group_layout(render_device: &RenderDevice) -> BindGroupLayout;

    /// Returns this material's vertex shader. If [`None`] is returned, the default mesh vertex shader will be used.
    /// Defaults to [`None`].
    #[allow(unused_variables)]
    fn vertex_shader(asset_server: &AssetServer) -> Option<Handle<Shader>> {
        None
    }

    /// Returns this material's fragment shader. If [`None`] is returned, the default mesh fragment shader will be used.
    /// Defaults to [`None`].
    #[allow(unused_variables)]
    fn fragment_shader(asset_server: &AssetServer) -> Option<Handle<Shader>> {
        None
    }

    /// Returns this material's [`AlphaMode`]. Defaults to [`AlphaMode::Opaque`].
    #[allow(unused_variables)]
    fn alpha_mode(material: &<Self as RenderAsset>::PreparedAsset) -> AlphaMode {
        AlphaMode::Opaque
    }

    /// The dynamic uniform indices to set for the given `material`'s [`BindGroup`].
    /// Defaults to an empty array / no dynamic uniform indices.
    #[allow(unused_variables)]
    #[inline]
    fn dynamic_uniform_indices(material: &<Self as RenderAsset>::PreparedAsset) -> &[u32] {
        &[]
    }

    /// Customizes the default [`RenderPipelineDescriptor`].
    #[allow(unused_variables)]
    #[inline]
    fn specialize(
        pipeline: &InstancedMaterialPipeline<Self>,
        descriptor: &mut RenderPipelineDescriptor,
        layout: &MeshVertexBufferLayout,
    ) -> Result<(), SpecializedMeshPipelineError> {
        Ok(())
    }
}

