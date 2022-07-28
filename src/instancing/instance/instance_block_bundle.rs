use bevy::prelude::{Bundle, Handle, Visibility, ComputedVisibility};

use crate::prelude::SpecializedInstancedMaterial;

use super::instance_block::InstanceBlock;

/// Components to create a mesh instance
#[derive(Default, Bundle)]
pub struct InstanceBlockBundle<M: SpecializedInstancedMaterial> {
    pub material: Handle<M>,
    pub mesh_instance_block: InstanceBlock,
    pub visibility: Visibility,
    pub computed_visibility: ComputedVisibility,
}
