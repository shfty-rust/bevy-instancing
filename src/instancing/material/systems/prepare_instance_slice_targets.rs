use bevy::{
    prelude::{debug, Commands, Entity, Query, Res, With},
    render::view::{ExtractedView, VisibleEntities},
};

use crate::instancing::{
    instance_slice::InstanceSliceTarget,
    material::{
        material_instanced::MaterialInstanced,
        plugin::{GpuInstances, InstanceMeta},
    },
};

use super::prepare_instance_batches::ViewInstanceData;

pub fn system<M: MaterialInstanced>(
    view_instance_data: Res<ViewInstanceData<M>>,
    query_views: Query<(Entity, &InstanceMeta<M>), (With<ExtractedView>, With<VisibleEntities>)>,
    mut commands: Commands,
) {
    for (view_entity, instance_meta) in query_views.iter() {
        debug!("\tView {view_entity:?}");
        let view_instance_data =
            if let Some(view_instance_data) = view_instance_data.get(&view_entity) {
                view_instance_data
            } else {
                continue;
            };

        for key in instance_meta.instance_batches.keys() {
            let instance_buffer_data = view_instance_data.get(key).unwrap();

            for (entity, slice_range) in instance_meta
                .instance_batches
                .get(&key)
                .unwrap()
                .instance_slice_ranges
                .iter()
            {
                commands.entity(*entity).insert_bundle((
                    *slice_range,
                    InstanceSliceTarget {
                        buffer: if let GpuInstances::Storage { buffer } = &instance_buffer_data {
                            buffer.buffer().unwrap().clone()
                        } else {
                            panic!("InstanceSlice cannot be used with non-storage buffers")
                        },
                    },
                ));
            }
        }
    }
}
