use bevy::{
    prelude::{Bundle, ComputedVisibility, Handle, Mesh, Visibility},
    transform::TransformBundle,
};

use crate::prelude::SpecializedInstancedMaterial;

/// Components to create a mesh instance
#[derive(Default, Bundle)]
pub struct MeshInstanceBundle<M: SpecializedInstancedMaterial> {
    pub material: Handle<M>,
    pub mesh: Handle<Mesh>,
    #[bundle]
    pub transform: TransformBundle,
    pub visibility: Visibility,
    pub computed_visibility: ComputedVisibility,
}
