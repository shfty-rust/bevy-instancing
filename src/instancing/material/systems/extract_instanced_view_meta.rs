use bevy::{
    prelude::{debug, Commands, Entity, Query},
    render::{view::VisibleEntities, Extract},
};

use crate::instancing::material::{material_instanced::MaterialInstanced, plugin::InstanceMeta};

pub fn system<M: MaterialInstanced>(
    query_views: Extract<Query<(Entity, &VisibleEntities)>>,
    mut commands: Commands,
) {
    debug!("{}", std::any::type_name::<M>());
    for (view_entity, visible_entities) in query_views.iter() {
        if visible_entities.is_empty() {
            continue;
        }

        commands.insert_or_spawn_batch([(view_entity, (InstanceMeta::<M>::default(),))])
    }
}
