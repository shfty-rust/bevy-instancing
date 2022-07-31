use std::marker::PhantomData;

use bevy::{
    ecs::system::{
        lifetimeless::{Read, SQuery, SRes},
        SystemParamItem,
    },
    prelude::Entity,
    render::{render_phase::{EntityRenderCommand, RenderCommandResult, TrackedRenderPass}, render_resource::std140::{AsStd140, Std140}},
};
use bytemuck::Pod;

use crate::prelude::{InstanceBatchKey, InstanceViewMeta, SpecializedInstancedMaterial};

use super::instance::Instance;

/// Render command for drawing instanced meshes
pub struct SetInstancedMeshBindGroup<M: SpecializedInstancedMaterial, const I: usize>(
    PhantomData<M>,
);

impl<M: SpecializedInstancedMaterial, const I: usize> EntityRenderCommand
    for SetInstancedMeshBindGroup<M, I>
where
    <M::Instance as Instance>::PreparedInstance: AsStd140,
    <<<M::Instance as Instance>::PreparedInstance as AsStd140>::Output as Std140>::Padded: Pod,
{
    type Param = (SRes<InstanceViewMeta<M>>, SQuery<Read<InstanceBatchKey<M>>>);
    #[inline]
    fn render<'w>(
        view: Entity,
        item: Entity,
        (instance_view_meta, query_instance_batch_key): SystemParamItem<'w, '_, Self::Param>,
        pass: &mut TrackedRenderPass<'w>,
    ) -> RenderCommandResult {
        let batched_instances = instance_view_meta
            .into_inner()
            .get(&view)
            .unwrap()
            .batched_instances
            .get(query_instance_batch_key.get(item).unwrap())
            .unwrap();

        pass.set_bind_group(I, &batched_instances.instance_bind_group, &[]);

        RenderCommandResult::Success
    }
}
