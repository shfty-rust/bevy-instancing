use bevy::{
    ecs::query::{Fetch, WorldQuery},
    math::Mat4,
    prelude::{Component, Handle, Mesh},
};
use bytemuck::Pod;

pub type ReadOnlyQueryItem<'w, 's, Q> = <<Q as WorldQuery>::ReadOnlyFetch as Fetch<'w, 's>>::Item;

pub trait Instance {
    type ExtractedInstance: Component;
    type PreparedInstance: Default + Clone + Send + Sync + Pod;
    type Query: WorldQuery;

    fn extract_instance(instance: ReadOnlyQueryItem<Self::Query>) -> Self::ExtractedInstance;
    fn prepare_instance(instance: &Self::ExtractedInstance, mesh: u32) -> Self::PreparedInstance;

    fn mesh(instance: &Self::ExtractedInstance) -> &Handle<Mesh>;
    fn transform(instance: &Self::ExtractedInstance) -> Mat4;
}

