use bevy::prelude::Bundle;

use crate::prelude::{InstanceBundle, BoardMaterial, MeshInstanceColor};

#[derive(Default, Bundle)]
pub struct BoardInstanceBundle {
    #[bundle]
    pub instance_bundle: InstanceBundle<BoardMaterial>,
    pub mesh_instance_color: MeshInstanceColor,
}

