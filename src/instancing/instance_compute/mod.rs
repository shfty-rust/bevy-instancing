use std::marker::PhantomData;
use std::num::NonZeroU64;
use std::{borrow::Cow, hash::Hash};

use bevy::{
    asset::load_internal_asset,
    prelude::{
        debug, default, App, AssetServer, Commands, Entity, FromWorld, HandleUntyped, Image,
        Plugin, Query, Res, ResMut, Shader, World,
    },
    reflect::TypeUuid,
    render::{
        extract_component::{ExtractComponent, ExtractComponentPlugin},
        render_asset::RenderAssets,
        render_graph::{Node, NodeLabel, RenderGraph},
        render_resource::{
            AsBindGroup, BindGroup, BindGroupDescriptor, BindGroupEntry, BindGroupLayout,
            BindGroupLayoutDescriptor, BindGroupLayoutEntry, BindingResource, BindingType,
            BufferBinding, BufferBindingType, ComputePassDescriptor, ComputePipelineDescriptor,
            PipelineCache, PreparedBindGroup, ShaderRef, ShaderStages, SpecializedComputePipeline,
            SpecializedComputePipelines,
        },
        renderer::RenderDevice,
        texture::FallbackImage,
        RenderApp, RenderStage,
    },
};
use bevy::{prelude::Handle, render::render_resource::CachedComputePipelineId};

use crate::prelude::{InstanceSliceRange, InstanceSliceTarget};

use super::render::instance::Instance;

struct InstanceComputeLabel<T>(PhantomData<T>);

impl<T> Default for InstanceComputeLabel<T> {
    fn default() -> Self {
        Self(default())
    }
}

impl<T> Into<Cow<'static, str>> for InstanceComputeLabel<T> {
    fn into(self) -> Cow<'static, str> {
        Cow::Owned(format!(
            "instance_compute::<{}>",
            std::any::type_name::<T>()
        ))
    }
}
impl<T> Into<NodeLabel> for InstanceComputeLabel<T> {
    fn into(self) -> NodeLabel {
        NodeLabel::Name(self.into())
    }
}

pub const INSTANCE_COMPUTE_SHADER_HANDLE: HandleUntyped =
    HandleUntyped::weak_from_u64(Shader::TYPE_UUID, 3197649561934630342);

#[derive(Debug, Default, Copy, Clone)]
pub struct InstanceComputePlugin<T: InstanceCompute>(PhantomData<T>);

impl<T> Plugin for InstanceComputePlugin<T>
where
    T: 'static + Send + Sync + InstanceCompute,
    T::Data: Clone + PartialEq + Eq + Hash + for<'a> From<&'a T>,
{
    fn build(&self, app: &mut App) {
        load_internal_asset!(
            app,
            INSTANCE_COMPUTE_SHADER_HANDLE,
            "instance_compute.wgsl",
            Shader::from_wgsl
        );

        app.add_plugin(ExtractComponentPlugin::<T>::default());

        let render_app = app.sub_app_mut(RenderApp);
        render_app
            .init_resource::<InstanceComputePipeline<T>>()
            .init_resource::<SpecializedComputePipelines<InstanceComputePipeline<T>>>()
            .add_system_to_stage(RenderStage::Queue, queue_compute_instances::<T>);

        let mut render_graph = render_app.world.resource_mut::<RenderGraph>();
        render_graph.add_node(
            InstanceComputeLabel::<T>::default(),
            InstanceComputeNode::<T>::default(),
        );
        render_graph
            .add_node_edge(
                InstanceComputeLabel::<T>::default(),
                bevy::render::main_graph::node::CAMERA_DRIVER,
            )
            .unwrap();
    }
}

#[derive(Debug, Clone)]
pub struct InstanceComputePipeline<T: InstanceCompute> {
    pub uniform_bind_group_layout: BindGroupLayout,
    pub instance_bind_group_layout: BindGroupLayout,
    pub shader: Option<Handle<Shader>>,
    marker: PhantomData<T>,
}

impl<T> SpecializedComputePipeline for InstanceComputePipeline<T>
where
    T: InstanceCompute,
    T::Data: Clone + PartialEq + Eq + Hash,
{
    type Key = T::Data;

    fn specialize(&self, key: Self::Key) -> ComputePipelineDescriptor {
        debug!("InstanceComputePipeline::specialize");

        let mut descriptor = ComputePipelineDescriptor {
            label: Some("instance compute".into()),
            layout: Some(vec![
                self.uniform_bind_group_layout.clone(),
                self.instance_bind_group_layout.clone(),
            ]),
            shader: if let Some(shader) = &self.shader {
                shader.clone_weak()
            } else {
                INSTANCE_COMPUTE_SHADER_HANDLE.typed()
            },
            shader_defs: vec![],
            entry_point: Cow::from("instances"),
        };

        T::specialize(self, &mut descriptor, key);

        descriptor
    }
}

impl<T: InstanceCompute> FromWorld for InstanceComputePipeline<T> {
    fn from_world(world: &mut World) -> Self {
        let render_device = world.get_resource::<RenderDevice>().unwrap();

        let uniform_bind_group_layout = T::bind_group_layout(render_device);

        let instance_bind_group_layout =
            render_device.create_bind_group_layout(&BindGroupLayoutDescriptor {
                label: Some("instance buffer bind group"),
                entries: &[BindGroupLayoutEntry {
                    binding: 0,
                    visibility: ShaderStages::COMPUTE,
                    ty: BindingType::Buffer {
                        ty: BufferBindingType::Storage { read_only: false },
                        has_dynamic_offset: false,
                        min_binding_size: None,
                    },
                    count: None,
                }],
            });

        let asset_server = world.resource::<AssetServer>();
        let shader = match T::shader() {
            ShaderRef::Default => None,
            ShaderRef::Handle(handle) => Some(handle),
            ShaderRef::Path(path) => Some(asset_server.load(path)),
        };

        InstanceComputePipeline {
            uniform_bind_group_layout,
            instance_bind_group_layout,
            shader,
            marker: default(),
        }
    }
}

struct InstanceComputeNode<T>(PhantomData<T>);

impl<T: InstanceCompute> Default for InstanceComputeNode<T> {
    fn default() -> Self {
        Self(default())
    }
}

struct InstanceComputeQueue<T: InstanceCompute>(Vec<InstanceComputeJob<T>>);

struct InstanceComputeJob<T: InstanceCompute> {
    pipeline: CachedComputePipelineId,
    uniform_bind_group: PreparedBindGroup<T>,
    instance_bind_group: BindGroup,
    instance_count: u64,
}

const WORKGROUP_SIZE: u64 = 64;

impl<T> Node for InstanceComputeNode<T>
where
    T: 'static + Send + Sync + InstanceCompute,
{
    fn run(
        &self,
        _graph: &mut bevy::render::render_graph::RenderGraphContext,
        render_context: &mut bevy::render::renderer::RenderContext,
        world: &bevy::prelude::World,
    ) -> Result<(), bevy::render::render_graph::NodeRunError> {
        debug!("InstanceComputeNode::run");
        let pipeline_cache = world.resource::<PipelineCache>();

        let compute_jobs = &world.resource::<InstanceComputeQueue<T>>().0;
        for compute_job in compute_jobs {
            if let Some(instance_pipeline) =
                pipeline_cache.get_compute_pipeline(compute_job.pipeline)
            {
                debug!(
                    "Running compute job with {} instances",
                    compute_job.instance_count
                );

                let mut pass = render_context
                    .command_encoder
                    .begin_compute_pass(&ComputePassDescriptor::default());

                pass.set_bind_group(0, &compute_job.uniform_bind_group.bind_group, &[]);
                pass.set_bind_group(1, &compute_job.instance_bind_group, &[]);

                let instance_workgroups =
                    (compute_job.instance_count / WORKGROUP_SIZE).max(1) as u32;

                pass.set_pipeline(instance_pipeline);
                pass.dispatch_workgroups(instance_workgroups, 1, 1);
            }
        }

        Ok(())
    }
}

pub fn queue_compute_instances<T>(
    pipeline: Res<InstanceComputePipeline<T>>,
    render_device: Res<RenderDevice>,
    mut pipeline_cache: ResMut<PipelineCache>,
    mut compute_pipelines: ResMut<SpecializedComputePipelines<InstanceComputePipeline<T>>>,
    render_images: Res<RenderAssets<Image>>,
    fallback_image: Res<FallbackImage>,
    query_instance_slice: Query<(Entity, &T, &InstanceSliceRange, &InstanceSliceTarget)>,
    mut commands: Commands,
) where
    T: InstanceCompute,
    T::Data: Clone + PartialEq + Eq + Hash + for<'a> From<&'a T>,
{
    debug!("queue_compute_instances");
    let mut instance_compute_queue = vec![];

    for (
        instance_slice_entity,
        instance_compute_uniform,
        instance_slice_range,
        instance_slice_buffer,
    ) in query_instance_slice.iter()
    {
        debug!("Instance slice {instance_slice_entity:?}");
        let uniform_bind_group = match instance_compute_uniform.as_bind_group(
            &pipeline.uniform_bind_group_layout,
            &render_device,
            &render_images,
            &fallback_image,
        ) {
            Ok(uniform_bind_group) => uniform_bind_group,
            Err(_) => panic!("Failed to create uniform bind group"),
        };

        let instance_bind_group = render_device.create_bind_group(&BindGroupDescriptor {
            label: None,
            layout: &pipeline.instance_bind_group_layout,
            entries: &[BindGroupEntry {
                binding: 0,
                resource: BindingResource::Buffer(BufferBinding {
                    buffer: &instance_slice_buffer.buffer,
                    offset: std::mem::size_of::<<T::Instance as Instance>::PreparedInstance>()
                        as u64
                        * instance_slice_range.offset,
                    size: NonZeroU64::new(
                        std::mem::size_of::<<T::Instance as Instance>::PreparedInstance>() as u64
                            * instance_slice_range.instance_count,
                    ),
                }),
            }],
        });

        let pipeline = compute_pipelines.specialize(
            &mut pipeline_cache,
            &pipeline,
            instance_compute_uniform.into(),
        );

        debug!(
            "Queueing InstanceComputeJob for {} cells",
            instance_slice_range.instance_count
        );

        instance_compute_queue.push(InstanceComputeJob {
            pipeline,
            uniform_bind_group,
            instance_bind_group,
            instance_count: instance_slice_range.instance_count,
        });
    }

    commands.insert_resource(InstanceComputeQueue(instance_compute_queue));
}

pub trait InstanceCompute: AsBindGroup + ExtractComponent {
    type Instance: Instance;

    fn shader() -> ShaderRef {
        ShaderRef::Default
    }

    #[allow(unused_variables)]
    fn specialize(
        pipeline: &InstanceComputePipeline<Self>,
        descriptor: &mut ComputePipelineDescriptor,
        key: Self::Data,
    ) {
    }
}
