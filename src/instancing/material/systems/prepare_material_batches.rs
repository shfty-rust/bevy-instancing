use std::{
    collections::BTreeMap,
    fmt::Debug,
    marker::PhantomData,
    ops::{Deref, DerefMut},
};

use bevy::prelude::{debug, Res, ResMut};

use crate::instancing::material::{
    material_instanced::MaterialInstanced,
    plugin::{GpuAlphaMode, InstancedMaterialBatchKey, MaterialBatch, RenderMaterials},
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
) where
    M::Data: Debug + Clone,
{
    if !render_materials.is_changed() {
        return;
    }

    debug!("{}", std::any::type_name::<M>());

    // Batch materials by key
    **material_batches = render_materials
        .iter()
        .flat_map(|(material_handle, material)| {
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
