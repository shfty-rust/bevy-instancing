pub use crate::{
    colored_mesh_instance::{color_instance_bundle::*, mesh_instance_color::*, plugin::*, *},
    instancing::{
        indirect::*,
        instance_slice::{instance_slice_bundle::*, *},
        instance_compute::*,
        material::{
            instanced_material_pipeline::*, plugin::*,
            set_instanced_material_bind_group::*, material_instanced::*, systems::*, *,
        },
        mesh_instance::{mesh_instance_bundle::*, *},
        plugin::*,
        render::{instance::*, instanced_mesh_pipeline::*, *},
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
