pub mod instance_slice_bundle;

use bevy::{
    ecs::{reflect::ReflectComponent, system::lifetimeless::Read},
    prelude::Component,
    reflect::Reflect,
    render::{extract_component::ExtractComponent, render_resource::Buffer},
};

/// Allocates a contiguous slice of the instance buffer corresponding to a given mesh and material
/// Used to reserve space for compute-driven instances
#[derive(Debug, Default, Copy, Clone, Component, Reflect)]
#[reflect(Component)]
pub struct InstanceSlice {
    pub instance_count: usize,
}

impl ExtractComponent for InstanceSlice {
    type Query = Read<Self>;

    type Filter = ();

    fn extract_component(item: bevy::ecs::query::QueryItem<Self::Query>) -> Self {
        *item
    }
}

#[derive(Debug, Copy, Clone, Component)]
pub struct InstanceSliceRange {
    pub offset: u64,
    pub instance_count: u64,
}

#[derive(Debug, Clone, Component)]
pub struct InstanceSliceTarget {
    pub buffer: Buffer,
}
