pub use crate::{
    custom_material::{
        board_instance_bundle::*, board_material::*, board_mesh_instance::*,
        mesh_instance_color::*, *,
    },
    instancing::{
        compute::{
            compute_jobs::*,
            node::*,
            pipelines::{indirect_offsets_pipeline::*, sort_instances_pipeline::*, *},
            plugin::*,
            *,
        },
        instance::{instance_block::*, instance_block_bundle::*, instance_bundle::*, *},
        material::{
            basic_material::*, instanced_material::*, instanced_material_pipeline::*, plugin::*,
            set_instanced_material_bind_group::*, specialized_instanced_material::*, *,
        },
        plugin::*,
        render::{
            draw_indexed_indirect::*, draw_indirect::*, instance::*, instance_data::*,
            instanced_mesh_pipeline::*, mesh_instance::*, set_instanced_mesh_bind_group::*, *,
        },
        *,
    },
    *,
};
