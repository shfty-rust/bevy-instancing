pub mod plugin;

use bevy::{
    ecs::system::{lifetimeless::SRes, SystemParamItem},
    reflect::TypeUuid,
    render::{
        render_asset::{PrepareAssetError, RenderAsset},
        render_resource::{
            AsBindGroup, BindGroup, BindGroupDescriptor, BindGroupLayout,
            BindGroupLayoutDescriptor, PreparedBindGroup,
        },
        renderer::RenderDevice,
    },
};

use crate::{
    instancing::material::material_instanced::{AsBatch, MaterialInstanced},
    prelude::{InstancedMaterialPipeline, MeshInstance},
};

#[derive(Debug, Default, Clone, TypeUuid)]
#[uuid = "40d95476-3236-4c43-a1c9-1f0645ca762a"]
pub struct BasicMaterial;

impl AsBindGroup for BasicMaterial {
    type Data = ();

    fn as_bind_group(
        &self,
        layout: &BindGroupLayout,
        render_device: &RenderDevice,
        _images: &bevy::render::render_asset::RenderAssets<bevy::prelude::Image>,
        _fallback_image: &bevy::render::texture::FallbackImage,
    ) -> Result<
        bevy::render::render_resource::PreparedBindGroup<Self>,
        bevy::render::render_resource::AsBindGroupError,
    > {
        Ok(PreparedBindGroup {
            bindings: vec![],
            bind_group: render_device.create_bind_group(&BindGroupDescriptor {
                label: Some("BasicMaterial Bind Group"),
                layout,
                entries: &[],
            }),
            data: (),
        })
    }

    fn bind_group_layout(render_device: &RenderDevice) -> BindGroupLayout {
        render_device.create_bind_group_layout(&BindGroupLayoutDescriptor {
            label: Some("BasicMaterial Bind Group Layout"),
            entries: &[],
        })
    }
}

#[derive(Clone)]
pub struct GpuBasicMaterial {
    pub bind_group: BindGroup,
}

impl RenderAsset for BasicMaterial {
    type ExtractedAsset = BasicMaterial;
    type PreparedAsset = GpuBasicMaterial;
    type Param = (SRes<RenderDevice>, SRes<InstancedMaterialPipeline<Self>>);
    fn extract_asset(&self) -> Self::ExtractedAsset {
        self.clone()
    }

    fn prepare_asset(
        _: Self::ExtractedAsset,
        (render_device, material_pipeline): &mut SystemParamItem<Self::Param>,
    ) -> Result<Self::PreparedAsset, PrepareAssetError<Self::ExtractedAsset>> {
        let bind_group = render_device.create_bind_group(&BindGroupDescriptor {
            entries: &[],
            label: None,
            layout: &material_pipeline.material_layout,
        });

        Ok(GpuBasicMaterial { bind_group })
    }
}

impl From<&BasicMaterial> for () {
    fn from(_: &BasicMaterial) -> Self {}
}

impl AsBatch for BasicMaterial {
    type BatchKey = ();
}

impl MaterialInstanced for BasicMaterial {
    type Instance = MeshInstance;
}
