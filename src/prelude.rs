pub use crate::{
    colored_mesh_instance::{color_instance_bundle::*, mesh_instance_color::*, plugin::*, *},
    instancing::{
        instance_block::{instance_block_bundle::*, *},
        material::{
            instanced_material::*, instanced_material_pipeline::*, plugin::*,
            set_instanced_material_bind_group::*, specialized_instanced_material::*, systems::*, *,
        },
        mesh_instance::{mesh_instance_bundle::*, *},
        plugin::*,
        render::{
            instance::*, instanced_mesh_pipeline::*,
            set_instanced_mesh_bind_group::*, *,
        },
        *,
    },
    materials::{
        basic_material::{plugin::*, *},
        custom_material::{custom_material::*, plugin::*, *},
        texture_material::{plugin::*, texture_material::*, *},
        *,
    },
    *,
};
