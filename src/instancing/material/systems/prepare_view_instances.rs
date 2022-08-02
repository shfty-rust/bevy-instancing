use bevy::{
    prelude::{debug, Entity, Handle, Query, With},
    render::view::{ExtractedView, VisibleEntities},
};

use crate::instancing::{
    material::{material_instanced::MaterialInstanced, plugin::InstanceMeta},
    render::instance::Instance,
};

pub fn system<M: MaterialInstanced>(
    mut query_views: Query<(Entity, &VisibleEntities, &mut InstanceMeta<M>), With<ExtractedView>>,
    query_instance: Query<
        Entity,
        (
            With<Handle<M>>,
            With<<M::Instance as Instance>::ExtractedInstance>,
        ),
    >,
) {
    debug!("{}", std::any::type_name::<M>());

    for (view_entity, visible_entities, mut instance_meta) in query_views.iter_mut() {
        debug!("{view_entity:?}");

        instance_meta.instances = visible_entities
            .entities
            .iter()
            .copied()
            .filter(|entity| query_instance.get(*entity).is_ok())
            .collect::<Vec<_>>();
    }
}
