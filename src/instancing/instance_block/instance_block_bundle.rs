use bevy::{
    prelude::{Bundle, ComputedVisibility, Handle, Mesh, Visibility, default},
    render::view::NoFrustumCulling,
};

use crate::prelude::{InstanceBlock, MaterialInstanced};

/// Components to create a mesh instance
#[derive(Bundle)]
pub struct InstanceBlockBundle<M: MaterialInstanced> {
    pub material: Handle<M>,
    pub mesh: Handle<Mesh>,
    pub mesh_instance_block: InstanceBlock,
    pub visibility: Visibility,
    pub computed_visibility: ComputedVisibility,
    pub no_frustum_culling: NoFrustumCulling,
}

impl<M: MaterialInstanced> Default for InstanceBlockBundle<M> {
    fn default() -> Self {
        Self {
            material: default(),
            mesh: default(),
            mesh_instance_block: default(),
            visibility: default(),
            computed_visibility: default(),
            no_frustum_culling: NoFrustumCulling,
        }
    }
}
