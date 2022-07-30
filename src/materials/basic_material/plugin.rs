use bevy::{
    prelude::{AddAsset, Assets, Handle, HandleUntyped, Plugin, Shader},
    reflect::TypeUuid,
};

use crate::prelude::InstancedMaterialPlugin;

use super::BasicMaterial;

pub const TEXTURE_SHADER_HANDLE: HandleUntyped =
    HandleUntyped::weak_from_u64(Shader::TYPE_UUID, 5970006216441508455);

pub struct BasicMaterialPlugin;

impl Plugin for BasicMaterialPlugin {
    fn build(&self, app: &mut bevy::prelude::App) {
        app.add_asset::<BasicMaterial>()
            .add_plugin(InstancedMaterialPlugin::<BasicMaterial>::default());

        app.world
            .resource_mut::<Assets<BasicMaterial>>()
            .set_untracked(Handle::<BasicMaterial>::default(), BasicMaterial::default());
    }
}

