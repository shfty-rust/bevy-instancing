use bevy::asset::AssetServer;
use bevy::pbr::AlphaMode;
use bevy::reflect::TypeUuid;
use bevy::render::render_resource::{AsBindGroup, ShaderRef};
use bevy::render::{
    mesh::MeshVertexBufferLayout,
    render_resource::{RenderPipelineDescriptor, SpecializedMeshPipelineError},
};

use crate::prelude::{Instance, InstancedMaterialPipeline};

pub trait AsBatch {
    type BatchKey: std::fmt::Debug + PartialOrd + Ord + Clone + Send + Sync + for<'a> From<&'a Self>;
}

/// Materials are used alongside [`MaterialPlugin`] and [`MaterialMeshBundle`](crate::MaterialMeshBundle)
/// to spawn entities that are rendered with a specific [`SpecializedMaterial`] type. They serve as an easy to use high level
/// way to render [`Mesh`] entities with custom shader logic. [`SpecializedMaterials`](SpecializedMaterial) use their [`SpecializedMaterial::Key`]
/// to customize their [`RenderPipelineDescriptor`] based on specific material values. The slightly simpler [`Material`] trait
/// should be used for materials that do not need specialization. [`Material`] types automatically implement [`SpecializedMaterial`].
pub trait MaterialInstanced:
    AsBindGroup + AsBatch + Send + Sync + Clone + TypeUuid + Sized + 'static
{
    /// Type used to store per-instance data
    type Instance: Instance;

    /// Returns this material's vertex shader. If [`None`] is returned, the default mesh vertex shader will be used.
    /// Defaults to [`None`].
    #[allow(unused_variables)]
    fn vertex_shader(asset_server: &AssetServer) -> ShaderRef {
        ShaderRef::Default
    }

    /// Returns this material's fragment shader. If [`None`] is returned, the default mesh fragment shader will be used.
    /// Defaults to [`None`].
    #[allow(unused_variables)]
    fn fragment_shader(asset_server: &AssetServer) -> ShaderRef {
        ShaderRef::Default
    }

    /// Returns this material's [`AlphaMode`]. Defaults to [`AlphaMode::Opaque`].
    #[allow(unused_variables)]
    fn alpha_mode(&self) -> AlphaMode {
        AlphaMode::Opaque
    }

    #[inline]
    /// Add a bias to the view depth of the mesh which can be used to force a specific render order
    /// for meshes with equal depth, to avoid z-fighting.
    fn depth_bias(&self) -> f32 {
        0.0
    }

    /// Specializes the given `descriptor` according to the given `key`.
    #[allow(unused_variables)]
    fn specialize(
        pipeline: &InstancedMaterialPipeline<Self>,
        descriptor: &mut RenderPipelineDescriptor,
        key: Self::Data,
        layout: &MeshVertexBufferLayout,
    ) -> Result<(), SpecializedMeshPipelineError> {
        Ok(())
    }
}
