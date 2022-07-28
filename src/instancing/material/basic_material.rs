use bevy::{
    ecs::system::{lifetimeless::SRes, SystemParamItem},
    reflect::TypeUuid,
    render::{
        render_asset::{PrepareAssetError, RenderAsset},
        render_resource::{
            BindGroup, BindGroupDescriptor, BindGroupLayout, BindGroupLayoutDescriptor,
        },
        renderer::RenderDevice,
    },
};

use crate::prelude::{InstancedMaterial, InstancedMaterialPipeline, MeshInstance};

#[derive(Debug, Default, Clone, TypeUuid)]
#[uuid = "40d95476-3236-4c43-a1c9-1f0645ca762a"]
pub struct BasicMaterial;

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

impl InstancedMaterial for BasicMaterial {
    type Instance = MeshInstance;

    fn bind_group(render_asset: &<Self as RenderAsset>::PreparedAsset) -> &BindGroup {
        &render_asset.bind_group
    }

    fn bind_group_layout(render_device: &RenderDevice) -> BindGroupLayout {
        render_device.create_bind_group_layout(&BindGroupLayoutDescriptor {
            entries: &[],
            label: Some("material layout"),
        })
    }
}
