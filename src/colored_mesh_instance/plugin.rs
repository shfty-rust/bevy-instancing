use bevy::{
    asset::load_internal_asset,
    prelude::{HandleUntyped, Plugin, Shader},
    reflect::TypeUuid,
};

use bevy::asset as bevy_asset;

use crate::prelude::InstanceColor;

pub const COLOR_INSTANCE_STRUCT_HANDLE: HandleUntyped =
    HandleUntyped::weak_from_u64(Shader::TYPE_UUID, 12512679806184200914);

pub struct ColorInstancePlugin;

impl Plugin for ColorInstancePlugin {
    fn build(&self, app: &mut bevy::prelude::App) {
        load_internal_asset!(
            app,
            COLOR_INSTANCE_STRUCT_HANDLE,
            "color_instance_struct.wgsl",
            Shader::from_wgsl
        );

        app.register_type::<InstanceColor>();
    }
}
