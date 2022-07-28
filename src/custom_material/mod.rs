pub mod board_instance_bundle;
pub mod board_material;
pub mod board_mesh_instance;
pub mod mesh_instance_color;

use bevy::{
    asset::load_internal_asset,
    prelude::{AddAsset, Assets, Handle, HandleUntyped, Plugin, Shader},
    reflect::TypeUuid,
};

use bevy::asset as bevy_asset;
use board_material::BoardMaterial;
use mesh_instance_color::MeshInstanceColor;

use crate::prelude::InstancedMaterialPlugin;

pub const BOARD_SHADER_HANDLE: HandleUntyped =
    HandleUntyped::weak_from_u64(Shader::TYPE_UUID, 2832496304849745969);

pub struct CustomMaterialPlugin;

impl Plugin for CustomMaterialPlugin {
    fn build(&self, app: &mut bevy::prelude::App) {
        load_internal_asset!(app, BOARD_SHADER_HANDLE, "board.wgsl", Shader::from_wgsl);

        app.register_type::<MeshInstanceColor>();

        app.add_asset::<BoardMaterial>()
            .add_plugin(InstancedMaterialPlugin::<BoardMaterial>::default());

        app.world
            .resource_mut::<Assets<BoardMaterial>>()
            .set_untracked(Handle::<BoardMaterial>::default(), BoardMaterial::default());
    }
}
