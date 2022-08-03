use std::num::NonZeroU64;

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

pub trait InstanceUniformLength: Instance {
    const UNIFORM_BUFFER_LENGTH: NonZeroU64;
}

impl<T: Instance> InstanceUniformLength for T
where
    T: Instance,
{
    const UNIFORM_BUFFER_LENGTH: NonZeroU64 =
        unsafe { NonZeroU64::new_unchecked(16384 / T::PreparedInstance::SHADER_SIZE.get()) };
}
