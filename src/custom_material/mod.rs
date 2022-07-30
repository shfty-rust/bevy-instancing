pub mod custom_instance_bundle;
pub mod custom_material;
pub mod custom_mesh_instance;
pub mod mesh_instance_color;

use bevy::{
    asset::load_internal_asset,
    prelude::{AddAsset, Assets, Handle, HandleUntyped, Plugin, Shader},
    reflect::TypeUuid,
};

use bevy::asset as bevy_asset;
use custom_material::CustomMaterial;
use mesh_instance_color::MeshInstanceColor;

use crate::prelude::InstancedMaterialPlugin;

pub const CUSTOM_SHADER_HANDLE: HandleUntyped =
    HandleUntyped::weak_from_u64(Shader::TYPE_UUID, 2832496304849745969);

pub struct CustomMaterialPlugin;

impl Plugin for CustomMaterialPlugin {
    fn build(&self, app: &mut bevy::prelude::App) {
        load_internal_asset!(app, CUSTOM_SHADER_HANDLE, "custom.wgsl", Shader::from_wgsl);

        app.register_type::<MeshInstanceColor>();

        app.add_asset::<CustomMaterial>()
            .add_plugin(InstancedMaterialPlugin::<CustomMaterial>::default());

        app.world
            .resource_mut::<Assets<CustomMaterial>>()
            .set_untracked(Handle::<CustomMaterial>::default(), CustomMaterial::default());
    }
}
