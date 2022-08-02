use std::marker::PhantomData;

use bevy::{
    ecs::system::{
        lifetimeless::{Read, SQuery},
        SystemParamItem,
    },
    prelude::{debug, Entity},
    render::{
        render_phase::{EntityRenderCommand, RenderCommandResult, TrackedRenderPass},
        render_resource::encase::private::ShaderType,
    },
};

use crate::{
    instancing::material::plugin::InstanceMeta,
    prelude::{InstanceBatchKey, MaterialInstanced},
};

use super::instance::Instance;

/// Render command for drawing instanced meshes
pub struct SetInstancedMeshBindGroup<M: MaterialInstanced, const I: usize>(PhantomData<M>);

impl<M: MaterialInstanced, const I: usize> EntityRenderCommand for SetInstancedMeshBindGroup<M, I>
where
    <M::Instance as Instance>::PreparedInstance: ShaderType,
{
    type Param = (
        SQuery<Read<InstanceMeta<M>>>,
        SQuery<Read<InstanceBatchKey<M>>>,
    );
    #[inline]
    fn render<'w>(
        view: Entity,
        item: Entity,
        (instance_view_meta, query_instance_batch_key): SystemParamItem<'w, '_, Self::Param>,
        pass: &mut TrackedRenderPass<'w>,
    ) -> RenderCommandResult {
        debug!(
            "SetInstancedMeshBindGroup<{}, {}>",
            std::any::type_name::<M>(),
            I
        );

        let batched_instances = instance_view_meta
            .get_inner(view)
            .unwrap()
            .batched_instances
            .get(query_instance_batch_key.get(item).unwrap())
            .unwrap();

        pass.set_bind_group(I, &batched_instances.instance_bind_group, &[]);

        RenderCommandResult::Success
    }
}
