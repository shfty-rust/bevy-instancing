use bevy::{
    asset::Handle,
    ecs::{
        entity::Entity,
        system::{
            lifetimeless::{Read, SQuery, SRes},
            SystemParamItem,
        },
    },
    prelude::debug,
    render::render_phase::{EntityRenderCommand, RenderCommandResult, TrackedRenderPass},
};

use crate::prelude::MaterialInstanced;

use std::marker::PhantomData;

use super::plugin::RenderMaterials;

pub struct SetInstancedMaterialBindGroup<M: MaterialInstanced, const I: usize>(PhantomData<M>);

impl<M: MaterialInstanced, const I: usize> EntityRenderCommand
    for SetInstancedMaterialBindGroup<M, I>
{
    type Param = (SRes<RenderMaterials<M>>, SQuery<Read<Handle<M>>>);
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
        pass.set_bind_group(I, &material.bind_group, &[]);
        RenderCommandResult::Success
    }
}
