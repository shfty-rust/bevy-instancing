use bevy::{
    asset::Handle,
    ecs::{
        entity::Entity,
        system::{
            lifetimeless::{Read, SQuery, SRes},
            SystemParamItem,
        },
    },
    render::{
        render_asset::RenderAssets,
        render_phase::{EntityRenderCommand, RenderCommandResult, TrackedRenderPass},
    }, prelude::debug,
};

use crate::prelude::SpecializedInstancedMaterial;

use std::marker::PhantomData;

pub struct SetInstancedMaterialBindGroup<M: SpecializedInstancedMaterial, const I: usize>(
    PhantomData<M>,
);

impl<M: SpecializedInstancedMaterial, const I: usize> EntityRenderCommand
    for SetInstancedMaterialBindGroup<M, I>
{
    type Param = (SRes<RenderAssets<M>>, SQuery<Read<Handle<M>>>);
    fn render<'w>(
        _view: Entity,
        item: Entity,
        (materials, query): SystemParamItem<'w, '_, Self::Param>,
        pass: &mut TrackedRenderPass<'w>,
    ) -> RenderCommandResult {
        debug!(
            "SetInstancedMaterialBindGroup<{}, {}>",
            std::any::type_name::<M>(),
            I
        );

        let material_handle = query.get(item).unwrap();
        let material = materials.into_inner().get(material_handle).unwrap();
        pass.set_bind_group(
            I,
            M::bind_group(material),
            M::dynamic_uniform_indices(material),
        );
        RenderCommandResult::Success
    }
}

