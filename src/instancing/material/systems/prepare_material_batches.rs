use std::{collections::BTreeSet, fmt::Debug};

use bevy::{
    prelude::{debug, info, Entity, Handle, Query, Res, ResMut, With},
    render::view::{ExtractedView, VisibleEntities},
};

use crate::instancing::{
    instance_block::InstanceBlock,
    material::{
        plugin::{
            GpuAlphaMode, InstanceViewMeta, InstancedMaterialBatchKey, MaterialBatch,
            RenderMaterials,
        },
        specialized_instanced_material::MaterialInstanced,
    },
    render::instance::Instance,
};

pub fn system<M: MaterialInstanced>(
    mut instance_view_meta: ResMut<InstanceViewMeta<M>>,
    render_materials: Res<RenderMaterials<M>>,
    mut query_views: Query<Entity, (With<ExtractedView>, With<VisibleEntities>)>,
    query_instance: Query<(
        Entity,
        &Handle<M>,
        &<M::Instance as Instance>::ExtractedInstance,
    )>,
    query_instance_block: Query<(Entity, &Handle<M>, &InstanceBlock)>,
) where
    M::Data: Debug + Clone,
{
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
