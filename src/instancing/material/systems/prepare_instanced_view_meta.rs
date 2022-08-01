use bevy::{
    prelude::{debug, default, Entity, Query, ResMut, With},
    render::view::{ExtractedView, VisibleEntities},
};

use crate::instancing::material::{
    plugin::InstanceViewMeta, material_instanced::MaterialInstanced,
};

pub fn system<M: MaterialInstanced>(
    query_views: Query<Entity, (With<ExtractedView>, With<VisibleEntities>)>,
    mut instance_view_meta: ResMut<InstanceViewMeta<M>>,
) {
    debug!("{}", std::any::type_name::<M>());
    instance_view_meta.clear();
    for view_entity in query_views.iter() {
        debug!("\tView {view_entity:?}");
        instance_view_meta.insert(view_entity, default());
    }
}
