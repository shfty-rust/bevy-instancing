use bevy::{
    prelude::{info, Entity, Handle, Query, ResMut, With, debug},
    render::view::{ExtractedView, VisibleEntities},
};

use crate::instancing::{
    material::{
        plugin::InstanceViewMeta, specialized_instanced_material::MaterialInstanced,
    },
    render::instance::Instance,
};

pub fn system<M: MaterialInstanced>(
    query_views: Query<(Entity, &VisibleEntities), With<ExtractedView>>,
    query_instance: Query<
        (Entity,),
        (
            With<Handle<M>>,
            With<<M::Instance as Instance>::ExtractedInstance>,
        ),
    >,
    mut instance_view_meta: ResMut<InstanceViewMeta<M>>,
) {
    debug!("{}", std::any::type_name::<M>());

    for (view_entity, visible_entities) in query_views.iter() {
        debug!("{view_entity:?}");

        instance_view_meta.get_mut(&view_entity).unwrap().instances = visible_entities
            .entities
            .iter()
            .copied()
            .filter(|entity| query_instance.get(*entity).is_ok())
            .collect::<Vec<_>>();
    }
}
