use std::{
    collections::BTreeMap,
    fmt::Debug,
    marker::PhantomData,
    ops::{Deref, DerefMut},
};

use bevy::{
    prelude::{debug, info, Entity, Handle, Query, Res, ResMut, With},
    render::view::{ExtractedView, VisibleEntities},
};

use crate::instancing::{
    instance_slice::InstanceSlice,
    material::{
        material_instanced::MaterialInstanced,
        plugin::{
            GpuAlphaMode, InstanceMeta, InstancedMaterialBatchKey, MaterialBatch, RenderMaterials,
        },
    },
    render::instance::Instance,
};

pub struct MaterialBatches<M: MaterialInstanced> {
    pub material_batches: BTreeMap<InstancedMaterialBatchKey<M>, MaterialBatch<M>>,
    _phantom: PhantomData<M>,
}

impl<M: MaterialInstanced> Debug for MaterialBatches<M>
where
    M::Data: Debug,
{
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("MaterialBatches")
            .field("material_batches", &self.material_batches)
            .field("_phantom", &self._phantom)
            .finish()
    }
}

impl<M: MaterialInstanced> Default for MaterialBatches<M> {
    fn default() -> Self {
        Self {
            material_batches: Default::default(),
            _phantom: Default::default(),
        }
    }
}

impl<M: MaterialInstanced> Deref for MaterialBatches<M> {
    type Target = BTreeMap<InstancedMaterialBatchKey<M>, MaterialBatch<M>>;

    fn deref(&self) -> &Self::Target {
        &self.material_batches
    }
}

impl<M: MaterialInstanced> DerefMut for MaterialBatches<M> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.material_batches
    }
}

pub fn system<M: MaterialInstanced>(
    render_materials: Res<RenderMaterials<M>>,
    mut material_batches: ResMut<MaterialBatches<M>>,
    query_views: Query<
        (Entity, &mut InstanceMeta<M>),
        (With<ExtractedView>, With<VisibleEntities>),
    >,
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

    // Batch materials by key
    **material_batches = query_instance
        .iter()
        .map(|(_, material, _)| material)
        .chain(query_instance_slice.iter().map(|(_, material, _)| material))
        .flat_map(|material_handle| {
            let material = render_materials.get(material_handle)?;
            Some((
                InstancedMaterialBatchKey {
                    alpha_mode: GpuAlphaMode::from(material.properties.alpha_mode),
                    key: material.batch_key.clone(),
                },
                MaterialBatch {
                    material: material_handle.clone_weak(),
                    pipeline_key: material.pipeline_key.clone(),
                },
            ))
        })
        .collect();

    debug!("Material batches: {:#?}", material_batches);
}
