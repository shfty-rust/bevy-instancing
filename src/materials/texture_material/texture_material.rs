use bevy::{
    ecs::system::{lifetimeless::SRes, SystemParamItem},
    pbr::AlphaMode,
    prelude::{default, AssetServer, Handle, Image, Shader},
    reflect::TypeUuid,
    render::{
        mesh::MeshVertexBufferLayout,
        render_asset::{PrepareAssetError, RenderAsset, RenderAssets},
        render_resource::{
            AsBindGroup, BindGroup, BindGroupDescriptor, BindGroupEntry, BindingResource, Face,
            RenderPipelineDescriptor, SpecializedMeshPipelineError,
        },
        renderer::RenderDevice,
    },
};

use crate::{
    instancing::material::specialized_instanced_material::AsBatch,
    prelude::{ColorMeshInstance, InstancedMaterialPipeline, MaterialInstanced},
};

use super::plugin::TEXTURE_SHADER_HANDLE;

#[derive(Debug, Clone, AsBindGroup, TypeUuid)]
#[uuid = "335058d3-aa56-4b1b-b0aa-cf483b2c6ca4"]
#[bind_group_data(TextureMaterialPipelineKey)]
pub struct TextureMaterial {
    #[texture(0)]
    #[sampler(1)]
    pub texture: Handle<Image>,
    pub alpha_mode: AlphaMode,
    pub cull_mode: Option<Face>,
}

impl Default for TextureMaterial {
    fn default() -> Self {
        Self {
            texture: default(),
            alpha_mode: default(),
            cull_mode: Some(Face::Back),
        }
    }
}

#[derive(Clone)]
pub struct GpuTextureMaterial {
    pub texture: Handle<Image>,
    pub bind_group: BindGroup,
    pub alpha_mode: AlphaMode,
    pub cull_mode: Option<Face>,
}

impl RenderAsset for TextureMaterial {
    type ExtractedAsset = TextureMaterial;
    type PreparedAsset = GpuTextureMaterial;
    type Param = (
        SRes<RenderAssets<Image>>,
        SRes<RenderDevice>,
        SRes<InstancedMaterialPipeline<Self>>,
    );
    fn extract_asset(&self) -> Self::ExtractedAsset {
        self.clone()
    }

    fn prepare_asset(
        extracted_asset: Self::ExtractedAsset,
        (gpu_images, render_device, material_pipeline): &mut SystemParamItem<Self::Param>,
    ) -> Result<Self::PreparedAsset, PrepareAssetError<Self::ExtractedAsset>> {
        let gpu_image = if let Some(gpu_image) = gpu_images.get(&extracted_asset.texture) {
            gpu_image
        } else {
            return Err(PrepareAssetError::RetryNextUpdate(extracted_asset));
        };

        let bind_group = render_device.create_bind_group(&BindGroupDescriptor {
            entries: &[
                BindGroupEntry {
                    binding: 0,
                    resource: BindingResource::TextureView(&gpu_image.texture_view),
                },
                BindGroupEntry {
                    binding: 1,
                    resource: BindingResource::Sampler(&gpu_image.sampler),
                },
            ],
            label: None,
            layout: &material_pipeline.material_layout,
        });

        Ok(GpuTextureMaterial {
            texture: extracted_asset.texture,
            bind_group,
            alpha_mode: extracted_asset.alpha_mode,
            cull_mode: extracted_asset.cull_mode,
        })
    }
}

#[derive(Debug, Default, Clone, PartialEq, Eq, Hash)]
pub struct TextureMaterialPipelineKey {
    pub cull_mode: Option<Face>,
}

impl PartialOrd for TextureMaterialPipelineKey {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        self.cull_mode
            .map(|cull_mode| cull_mode as usize)
            .partial_cmp(&other.cull_mode.map(|cull_mode| cull_mode as usize))
    }
}

impl Ord for TextureMaterialPipelineKey {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        self.cull_mode
            .map(|cull_mode| cull_mode as usize)
            .cmp(&other.cull_mode.map(|cull_mode| cull_mode as usize))
    }
}

impl From<&TextureMaterial> for TextureMaterialPipelineKey {
    fn from(texture_material: &TextureMaterial) -> Self {
        TextureMaterialPipelineKey {
            cull_mode: texture_material.cull_mode,
        }
    }
}

#[derive(Debug, Default, Clone, PartialEq, Eq, Hash)]
pub struct TextureMaterialBatchKey {
    pub texture: Handle<Image>,
    pub cull_mode: Option<Face>,
}

impl PartialOrd for TextureMaterialBatchKey {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        match self.texture.partial_cmp(&other.texture) {
            Some(core::cmp::Ordering::Equal) => {}
            ord => return ord,
        }
        self.cull_mode
            .map(|cull_mode| cull_mode as usize)
            .partial_cmp(&other.cull_mode.map(|cull_mode| cull_mode as usize))
    }
}

impl Ord for TextureMaterialBatchKey {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        match self.texture.cmp(&other.texture) {
            core::cmp::Ordering::Equal => {}
            ord => return ord,
        }
        self.cull_mode
            .map(|cull_mode| cull_mode as usize)
            .cmp(&other.cull_mode.map(|cull_mode| cull_mode as usize))
    }
}

impl From<&TextureMaterial> for TextureMaterialBatchKey {
    fn from(texture_material: &TextureMaterial) -> Self {
        TextureMaterialBatchKey {
            texture: texture_material.texture.clone_weak(),
            cull_mode: texture_material.cull_mode,
        }
    }
}

impl AsBatch for TextureMaterial {
    type BatchKey = TextureMaterialBatchKey;
}

impl MaterialInstanced for TextureMaterial {
    type Instance = ColorMeshInstance;

    fn vertex_shader(_: &AssetServer) -> Option<Handle<Shader>> {
        Some(TEXTURE_SHADER_HANDLE.typed::<Shader>())
    }

    fn fragment_shader(_: &AssetServer) -> Option<Handle<Shader>> {
        Some(TEXTURE_SHADER_HANDLE.typed::<Shader>())
    }

    fn specialize(
        _pipeline: &InstancedMaterialPipeline<Self>,
        descriptor: &mut RenderPipelineDescriptor,
        key: Self::Data,
        _layout: &MeshVertexBufferLayout,
    ) -> Result<(), SpecializedMeshPipelineError> {
        descriptor.primitive.cull_mode = key.cull_mode;
        if let Some(label) = &mut descriptor.label {
            *label = format!("custom_{}", *label).into();
        }
        Ok(())
    }

    fn alpha_mode(&self) -> AlphaMode {
        self.alpha_mode
    }
}
