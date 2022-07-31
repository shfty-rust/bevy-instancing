use std::collections::BTreeSet;

use bevy::{
    prelude::{debug, Entity, Handle, Query, Res, ResMut, With, info},
    render::{
        render_asset::RenderAssets,
        view::{ExtractedView, VisibleEntities},
    },
};

use crate::instancing::{
    instance_block::InstanceBlock,
    material::{
        plugin::{InstanceViewMeta, InstancedMaterialBatchKey, MaterialBatch},
        specialized_instanced_material::SpecializedInstancedMaterial,
    },
    render::instance::Instance,
};

pub fn system<M: SpecializedInstancedMaterial>(
    mut instance_view_meta: ResMut<InstanceViewMeta<M>>,
    render_materials: Res<RenderAssets<M>>,
    mut query_views: Query<Entity, (With<ExtractedView>, With<VisibleEntities>)>,
    query_instance: Query<(
        Entity,
        &Handle<M>,
        &<M::Instance as Instance>::ExtractedInstance,
    )>,
    query_instance_block: Query<(Entity, &Handle<M>, &InstanceBlock)>,
) {
    debug!("{}", std::any::type_name::<M>());

    for view_entity in query_views.iter_mut() {
        debug!("View {view_entity:?}");
        let instance_meta = instance_view_meta.get_mut(&view_entity).unwrap();

        // Collect set of visible materials
        let materials = instance_meta
            .instances
            .iter()
            .flat_map(|entity| query_instance.get(*entity))
            .map(|(_, material, _)| material.clone_weak())
            .chain(
                instance_meta
                    .instance_blocks
                    .iter()
                    .flat_map(|entity| query_instance_block.get(*entity))
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
                        alpha_mode: M::alpha_mode(material).into(),
                        key: M::batch_key(material),
                    },
                    MaterialBatch {
                        material: material_handle,
                        pipeline_key: M::pipeline_key(material),
                    },
                ))
            })
            .collect();

        debug!("Material batches: {:#?}", instance_meta.material_batches);
    }
}
