use bevy::{
    prelude::{debug, info, Entity, Handle, Query, ResMut, With},
    render::view::{ExtractedView, VisibleEntities},
};

use crate::instancing::{
    instance_slice::InstanceSlice,
    material::{plugin::InstanceViewMeta, specialized_instanced_material::MaterialInstanced},
};

pub fn system<M: MaterialInstanced>(
    query_views: Query<(Entity, &VisibleEntities), With<ExtractedView>>,
    query_instance_slice: Query<Entity, (With<Handle<M>>, With<InstanceSlice>)>,
    mut instance_view_meta: ResMut<InstanceViewMeta<M>>,
) {
    debug!("{}", std::any::type_name::<M>());

    for (view_entity, visible_entities) in query_views.iter() {
        debug!("View {view_entity:?}");

        debug!("Visible entities: {visible_entities:#?}");

        let instance_slices = visible_entities
            .entities
            .iter()
            .copied()
            .filter(|entity| query_instance_slice.get(*entity).is_ok())
            .collect::<Vec<_>>();

        debug!("Instance slices: {instance_slices:#?}");

        instance_view_meta
            .get_mut(&view_entity)
            .unwrap()
            .instance_slices = instance_slices;
    }
}
