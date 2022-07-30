use bevy::prelude::Bundle;

use crate::{prelude::{MeshInstanceBundle, InstanceColor}, instancing::material::specialized_instanced_material::SpecializedInstancedMaterial};

#[derive(Default, Bundle)]
pub struct ColorInstanceBundle<M: SpecializedInstancedMaterial> {
    #[bundle]
    pub instance_bundle: MeshInstanceBundle<M>,
    pub mesh_instance_color: InstanceColor,
}

