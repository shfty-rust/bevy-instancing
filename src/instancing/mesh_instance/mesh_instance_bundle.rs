use bevy::prelude::{Bundle, Handle, Mesh, SpatialBundle};

use crate::prelude::MaterialInstanced;

/// Components to create a mesh instance
#[derive(Default, Bundle)]
pub struct MeshInstanceBundle<M: MaterialInstanced> {
    pub material: Handle<M>,
    pub mesh: Handle<Mesh>,
    #[bundle]
    pub spatial_bundle: SpatialBundle,
}
