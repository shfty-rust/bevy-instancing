use bevy::prelude::Bundle;

use crate::{prelude::{MeshInstanceBundle, InstanceColor}, instancing::material::specialized_instanced_material::MaterialInstanced};

#[derive(Default, Bundle)]
pub struct ColorInstanceBundle<M: MaterialInstanced> {
    #[bundle]
    pub instance_bundle: MeshInstanceBundle<M>,
    pub mesh_instance_color: InstanceColor,
}

