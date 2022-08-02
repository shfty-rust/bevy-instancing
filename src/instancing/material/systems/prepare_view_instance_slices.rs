use bevy::{
    prelude::{debug, Entity, Handle, Query, With},
    render::view::{ExtractedView, VisibleEntities},
};

use crate::instancing::{
    instance_slice::InstanceSlice,
    material::{material_instanced::MaterialInstanced, plugin::InstanceMeta},
};

pub fn system<M: MaterialInstanced>(
    mut query_views: Query<(Entity, &VisibleEntities, &mut InstanceMeta<M>), With<ExtractedView>>,
    query_instance_slice: Query<Entity, (With<Handle<M>>, With<InstanceSlice>)>,
) {
    debug!("{}", std::any::type_name::<M>());

    for (view_entity, visible_entities, mut instance_meta) in query_views.iter_mut() {
        debug!("View {view_entity:?}");

        debug!("Visible entities: {visible_entities:#?}");

        let instance_slices = visible_entities
            .entities
            .iter()
            .copied()
            .filter(|entity| query_instance_slice.get(*entity).is_ok())
            .collect::<Vec<_>>();

        debug!("Instance slices: {instance_slices:#?}");

        instance_meta.instance_slices = instance_slices;
    }
}
