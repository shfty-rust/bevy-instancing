use bevy::prelude::{Bundle, ComputedVisibility, Handle, Mesh, Visibility};

use crate::prelude::{InstanceBlock, SpecializedInstancedMaterial};

/// Components to create a mesh instance
#[derive(Default, Bundle)]
pub struct InstanceBlockBundle<M: SpecializedInstancedMaterial> {
    pub material: Handle<M>,
    pub mesh: Handle<Mesh>,
    pub mesh_instance_block: InstanceBlock,
    pub visibility: Visibility,
    pub computed_visibility: ComputedVisibility,
}
