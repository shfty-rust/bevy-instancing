use std::{collections::BTreeSet, fmt::Debug};

use bevy::{
    prelude::{debug, info, Entity, Handle, Query, Res, ResMut, With},
    render::view::{ExtractedView, VisibleEntities},
};

use crate::instancing::{
    instance_slice::InstanceSlice,
    material::{
        plugin::{
            GpuAlphaMode, InstancedMaterialBatchKey, MaterialBatch,
            RenderMaterials, InstanceMeta,
        },
        material_instanced::MaterialInstanced,
    },
    render::instance::Instance,
};

pub fn system<M: MaterialInstanced>(
    render_materials: Res<RenderMaterials<M>>,
    mut query_views: Query<(Entity, &mut InstanceMeta<M>), (With<ExtractedView>, With<VisibleEntities>)>,
    query_instance: Query<(
        Entity,
        &Handle<M>,
        &<M::Instance as Instance>::ExtractedInstance,
    )>,
    query_instance_slice: Query<(Entity, &Handle<M>, &InstanceSlice)>,
) where
    M::Data: Debug + Clone,
{
    debug!("{}", std::any::type_name::<M>());

    for (view_entity, mut instance_meta) in query_views.iter_mut() {
        debug!("View {view_entity:?}");

        // Collect set of visible materials
        let materials = instance_meta
            .instances
            .iter()
            .flat_map(|entity| query_instance.get(*entity))
            .map(|(_, material, _)| material.clone_weak())
            .chain(
                instance_meta
                    .instance_slices
                    .iter()
                    .flat_map(|entity| query_instance_slice.get(*entity))
                    .map(|(_, material, _)| material.clone_weak()),
            )
            .collect::<BTreeSet<_>>();

        // Batch materials by key
        instance_meta.material_batches = materials
            .into_iter()
            .flat_map(|material_handle| {
                let material = render_materials.get(&material_handle)?;
                Some((
                    InstancedMaterialBatchKey {
                        alpha_mode: GpuAlphaMode::from(material.properties.alpha_mode),
                        key: material.batch_key.clone(),
                    },
                    MaterialBatch {
                        material: material_handle,
                        pipeline_key: material.pipeline_key.clone(),
                    },
                ))
            })
            .collect();

        debug!("Material batches: {:#?}", instance_meta.material_batches);
    }
}
