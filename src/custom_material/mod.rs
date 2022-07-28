pub mod board_instance_bundle;
pub mod board_material;
pub mod board_mesh_instance;
pub mod mesh_instance_color;

use bevy::{
    prelude::{HandleUntyped, Shader},
    reflect::TypeUuid,
};

pub const BOARD_SHADER_HANDLE: HandleUntyped =
    HandleUntyped::weak_from_u64(Shader::TYPE_UUID, 2832496304849745969);
