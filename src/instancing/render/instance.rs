use bevy::{
    ecs::query::{ROQueryItem, ReadOnlyWorldQuery},
    math::Mat4,
    prelude::Component,
    render::render_resource::{
        encase::private::{ShaderType, WriteInto},
        ShaderSize,
    },
};

pub trait Instance {
    type ExtractedInstance: std::fmt::Debug + Component;
    type PreparedInstance: std::fmt::Debug
        + Default
        + Clone
        + Send
        + Sync
        + ShaderType
        + ShaderSize
        + WriteInto;
    type Query: ReadOnlyWorldQuery;

    fn extract_instance(instance: ROQueryItem<Self::Query>) -> Self::ExtractedInstance;
    fn prepare_instance(instance: &Self::ExtractedInstance, mesh: u32) -> Self::PreparedInstance;

    fn transform(instance: &Self::ExtractedInstance) -> Mat4;
}
