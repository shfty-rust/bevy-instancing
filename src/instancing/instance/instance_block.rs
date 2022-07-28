use bevy::{
    ecs::{reflect::ReflectComponent, system::lifetimeless::Read},
    prelude::{Component, Handle, Mesh},
    reflect::Reflect,
    render::{render_component::ExtractComponent, render_resource::Buffer},
};

/// Allocates a contiguous block of the instance buffer corresponding to a given material
/// Used to reserve space for compute-driven instances
#[derive(Debug, Default, Clone, Component, Reflect)]
#[reflect(Component)]
pub struct InstanceBlock {
    pub mesh: Handle<Mesh>,
    pub instance_count: usize,
}

impl ExtractComponent for InstanceBlock {
    type Query = Read<Self>;

    type Filter = ();

    fn extract_component(item: bevy::ecs::query::QueryItem<Self::Query>) -> Self {
        item.clone()
    }
}

#[derive(Debug, Copy, Clone, Component)]
pub struct InstanceBlockRange {
    pub offset: u64,
    pub instance_count: u64,
}

#[derive(Debug, Clone, Component)]
pub struct InstanceBlockBuffer {
    pub buffer: Buffer,
}
