use bevy::{
    ecs::query::{Fetch, WorldQuery},
    math::Mat4,
    prelude::Component,
    render::render_resource::{std140::AsStd140, std430::AsStd430},
};
use bytemuck::Pod;

pub type ReadOnlyQueryItem<'w, 's, Q> = <<Q as WorldQuery>::ReadOnlyFetch as Fetch<'w, 's>>::Item;

pub trait Instance {
    type ExtractedInstance: std::fmt::Debug + Component;
    type PreparedInstance: std::fmt::Debug + Default + Clone + Send + Sync + Pod + AsStd140 + AsStd430;
    type Query: WorldQuery;

    fn extract_instance(instance: ReadOnlyQueryItem<Self::Query>) -> Self::ExtractedInstance;
    fn prepare_instance(instance: &Self::ExtractedInstance, mesh: u32) -> Self::PreparedInstance;

    fn transform(instance: &Self::ExtractedInstance) -> Mat4;
}
