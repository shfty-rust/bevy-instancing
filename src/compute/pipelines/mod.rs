use bevy::prelude::{FromWorld, World};

use crate::prelude::{IndirectOffsetsPipeline, SortInstancesPipeline};

pub mod indirect_offsets_pipeline;
pub mod sort_instances_pipeline;

pub struct IndirectComputePipelines {
    pub indirect_offsets: IndirectOffsetsPipeline,
    pub sort_instances: SortInstancesPipeline,
}

impl FromWorld for IndirectComputePipelines {
    fn from_world(world: &mut World) -> Self {
        let indirect_offsets = IndirectOffsetsPipeline::from_world(world);
        let sort_instances = SortInstancesPipeline::from_world(world);

        IndirectComputePipelines {
            indirect_offsets,
            sort_instances,
        }
    }
}
