use bevy::{
    asset::load_internal_asset,
    prelude::{AddAsset, Assets, Handle, HandleUntyped, Plugin, Shader},
    reflect::TypeUuid,
};

use crate::prelude::{ColorInstancePlugin, CustomMaterial, InstanceColor, InstancedMaterialPlugin};

pub const CUSTOM_SHADER_HANDLE: HandleUntyped =
    HandleUntyped::weak_from_u64(Shader::TYPE_UUID, 2832496304849745969);

pub struct CustomMaterialPlugin;

impl Plugin for CustomMaterialPlugin {
    fn build(&self, app: &mut bevy::prelude::App) {
        load_internal_asset!(app, CUSTOM_SHADER_HANDLE, "custom.wgsl", Shader::from_wgsl);

        app.register_type::<InstanceColor>();

        app.add_asset::<CustomMaterial>()
            .add_plugin(InstancedMaterialPlugin::<CustomMaterial>::default());

        if !app.is_plugin_added::<ColorInstancePlugin>() {
            app.add_plugin(ColorInstancePlugin);
        }

        app.world
            .resource_mut::<Assets<CustomMaterial>>()
            .set_untracked(
                Handle::<CustomMaterial>::default(),
                CustomMaterial::default(),
            );
    }
}
