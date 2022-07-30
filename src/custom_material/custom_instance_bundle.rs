use bevy::prelude::Bundle;

use crate::prelude::{InstanceBundle, CustomMaterial, MeshInstanceColor};

#[derive(Default, Bundle)]
pub struct CustomInstanceBundle {
    #[bundle]
    pub instance_bundle: InstanceBundle<CustomMaterial>,
    pub mesh_instance_color: MeshInstanceColor,
}

