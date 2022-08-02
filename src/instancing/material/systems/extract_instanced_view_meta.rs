use bevy::{
    prelude::{debug, Commands, Entity, Query, With},
    render::{view::VisibleEntities, Extract},
};

use crate::instancing::material::{material_instanced::MaterialInstanced, plugin::InstanceMeta};

pub fn system<M: MaterialInstanced>(
    query_views: Extract<Query<Entity, With<VisibleEntities>>>,
    mut commands: Commands,
) {
    debug!("{}", std::any::type_name::<M>());
    for view_entity in query_views.iter() {
        commands.insert_or_spawn_batch([(view_entity, (InstanceMeta::<M>::default(),))])
    }
}
