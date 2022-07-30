use bevy::asset::{AssetServer, Handle};
use bevy::pbr::AlphaMode;
use bevy::render::{
    mesh::MeshVertexBufferLayout,
    render_asset::RenderAsset,
    render_resource::{
        BindGroup, BindGroupLayout, RenderPipelineDescriptor, Shader, SpecializedMeshPipelineError,
    },
    renderer::RenderDevice,
};

use crate::prelude::{Instance, InstancedMaterial, InstancedMaterialPipeline};

use std::hash::Hash;

/// Materials are used alongside [`MaterialPlugin`] and [`MaterialMeshBundle`](crate::MaterialMeshBundle)
/// to spawn entities that are rendered with a specific [`SpecializedMaterial`] type. They serve as an easy to use high level
/// way to render [`Mesh`] entities with custom shader logic. [`SpecializedMaterials`](SpecializedMaterial) use their [`SpecializedMaterial::Key`]
/// to customize their [`RenderPipelineDescriptor`] based on specific material values. The slightly simpler [`Material`] trait
/// should be used for materials that do not need specialization. [`Material`] types automatically implement [`SpecializedMaterial`].
pub trait SpecializedInstancedMaterial: RenderAsset + Sized {
    /// The key used to specialize this material's [`RenderPipelineDescriptor`].
    type PipelineKey: std::fmt::Debug + PartialEq + Eq + Hash + Clone + Send + Sync;

    /// The key used to batch instances of this material together
    type BatchKey: std::fmt::Debug + PartialOrd + Ord + Clone + Send + Sync;

    /// Type used to store per-instance data
    type Instance: Instance;

    /// Extract the [`SpecializedInstancedMaterial::PipelineKey`] for the "prepared" version of this material. This key will be
    /// passed in to the [`SpecializedInstancedMaterial::specialize`] function when compiling the [`RenderPipeline`](bevy_render::render_resource::RenderPipeline)
    /// for a given entity's material.
    fn pipeline_key(material: &<Self as RenderAsset>::PreparedAsset) -> Self::PipelineKey;

    /// Extract the [`SpecializedInstancedMaterial::BatchKey`] for the "prepared" version of this material.
    fn batch_key(material: &<Self as RenderAsset>::PreparedAsset) -> Self::BatchKey;

    /// Specializes the given `descriptor` according to the given `key`.
    fn specialize(
        pipeline: &InstancedMaterialPipeline<Self>,
        descriptor: &mut RenderPipelineDescriptor,
        key: Self::PipelineKey,
        layout: &MeshVertexBufferLayout,
    ) -> Result<(), SpecializedMeshPipelineError>;

    /// Returns this material's [`BindGroup`]. This should match the layout returned by [`SpecializedInstancedMaterial::bind_group_layout`].
    fn bind_group(material: &<Self as RenderAsset>::PreparedAsset) -> &BindGroup;

    /// Returns this material's [`BindGroupLayout`]. This should match the [`BindGroup`] returned by [`SpecializedInstancedMaterial::bind_group`].
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
}

impl<M: InstancedMaterial> SpecializedInstancedMaterial for M {
    type PipelineKey = ();
    type BatchKey = ();

    type Instance = M::Instance;

    #[inline]
    fn pipeline_key(_material: &<Self as RenderAsset>::PreparedAsset) -> Self::PipelineKey {}

    #[inline]
    fn batch_key(_material: &<Self as RenderAsset>::PreparedAsset) -> Self::BatchKey {}

    #[inline]
    fn specialize(
        pipeline: &InstancedMaterialPipeline<Self>,
        descriptor: &mut RenderPipelineDescriptor,
        _key: Self::BatchKey,
        layout: &MeshVertexBufferLayout,
    ) -> Result<(), SpecializedMeshPipelineError> {
        <M as InstancedMaterial>::specialize(pipeline, descriptor, layout)
    }

    #[inline]
    fn bind_group(material: &<Self as RenderAsset>::PreparedAsset) -> &BindGroup {
        <M as InstancedMaterial>::bind_group(material)
    }

    #[inline]
    fn bind_group_layout(render_device: &RenderDevice) -> BindGroupLayout {
        <M as InstancedMaterial>::bind_group_layout(render_device)
    }

    #[inline]
    fn alpha_mode(material: &<Self as RenderAsset>::PreparedAsset) -> AlphaMode {
        <M as InstancedMaterial>::alpha_mode(material)
    }

    #[inline]
    fn vertex_shader(asset_server: &AssetServer) -> Option<Handle<Shader>> {
        <M as InstancedMaterial>::vertex_shader(asset_server)
    }

    #[inline]
    fn fragment_shader(asset_server: &AssetServer) -> Option<Handle<Shader>> {
        <M as InstancedMaterial>::fragment_shader(asset_server)
    }

    #[allow(unused_variables)]
    #[inline]
    fn dynamic_uniform_indices(material: &<Self as RenderAsset>::PreparedAsset) -> &[u32] {
        <M as InstancedMaterial>::dynamic_uniform_indices(material)
    }
}
