use bevy::{
    prelude::{debug, Entity, Handle, Query, ResMut, With},
    render::view::{ExtractedView, VisibleEntities},
};

use crate::instancing::{
    instance_block::InstanceBlock,
    material::{
        plugin::InstanceViewMeta, specialized_instanced_material::SpecializedInstancedMaterial,
    },
};

pub fn system<M: SpecializedInstancedMaterial>(
    query_views: Query<(Entity, &VisibleEntities), With<ExtractedView>>,
    query_instance_block: Query<Entity, (With<Handle<M>>, With<InstanceBlock>)>,
    mut instance_view_meta: ResMut<InstanceViewMeta<M>>,
) {
    debug!("{}", std::any::type_name::<M>());

    for (view_entity, visible_entities) in query_views.iter() {
        let instance_blocks = visible_entities
            .entities
            .iter()
            .copied()
            .filter(|entity| query_instance_block.get(*entity).is_ok())
            .collect::<Vec<_>>();

        debug!("View {view_entity:?} instance blocks: {instance_blocks:#?}");

        instance_view_meta
            .get_mut(&view_entity)
            .unwrap()
            .instance_blocks = instance_blocks;
    }
}
