use bevy::{
    asset::load_internal_asset,
    prelude::{AddAsset, Assets, Handle, HandleUntyped, Plugin, Shader},
    reflect::TypeUuid,
};

use bevy::asset as bevy_asset;

use crate::prelude::{InstancedMaterialPlugin, TextureMaterial, ColorInstancePlugin};

pub const TEXTURE_SHADER_HANDLE: HandleUntyped =
    HandleUntyped::weak_from_u64(Shader::TYPE_UUID, 5970006216441508455);

pub struct TextureMaterialPlugin;

impl Plugin for TextureMaterialPlugin {
    fn build(&self, app: &mut bevy::prelude::App) {
        load_internal_asset!(
            app,
            TEXTURE_SHADER_HANDLE,
            "texture.wgsl",
            Shader::from_wgsl
        );

        app.add_asset::<TextureMaterial>()
            .add_plugin(ColorInstancePlugin)
            .add_plugin(InstancedMaterialPlugin::<TextureMaterial>::default());

        app.world
            .resource_mut::<Assets<TextureMaterial>>()
            .set_untracked(
                Handle::<TextureMaterial>::default(),
                TextureMaterial::default(),
            );
    }
}

