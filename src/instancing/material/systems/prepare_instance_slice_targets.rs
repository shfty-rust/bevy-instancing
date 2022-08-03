use bevy::{
    prelude::{debug, Commands, Entity, Query, With},
    render::view::{ExtractedView, VisibleEntities},
};

use crate::instancing::{
    instance_slice::InstanceSliceTarget,
    material::{
        material_instanced::MaterialInstanced,
        plugin::{GpuInstances, InstanceMeta},
    },
};

pub fn system<M: MaterialInstanced>(
    query_views: Query<(Entity, &InstanceMeta<M>), (With<ExtractedView>, With<VisibleEntities>)>,
    mut commands: Commands,
) {
    for (view_entity, instance_meta) in query_views.iter() {
        debug!("\tView {view_entity:?}");

        for (key, instance_batch) in instance_meta.instance_batches.iter() {
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
                        buffer: if let GpuInstances::Storage { buffer } =
                            &instance_batch.instance_buffer_data
                        {
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
