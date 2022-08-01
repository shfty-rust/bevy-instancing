//! Demonstration of InstanceSlice compute functionality
//!
//! Also highlights alpha ordering behaviour for transparent instance blocks;
//! batch order is visible when instances from different blocks draw on top
//! of one another.
//!

use std::borrow::Cow;
use std::marker::PhantomData;
use std::num::NonZeroU64;

use bevy::ecs::system::lifetimeless::Read;
use bevy::prelude::{
    debug, Camera3dBundle, Component, Entity, FromWorld, Handle, Query, Res, With, World,
};
use bevy::reflect::TypeUuid;
use bevy::render::extract_component::{ExtractComponent, ExtractComponentPlugin};
use bevy::render::render_graph::{Node, NodeLabel, RenderGraph};
use bevy::render::render_resource::{
    BindGroup, BindGroupDescriptor, BindGroupEntry, BindGroupLayout, BindGroupLayoutDescriptor,
    BindGroupLayoutEntry, BindingResource, BindingType, BufferBinding, BufferBindingType,
    BufferInitDescriptor, BufferUsages, CachedComputePipelineId, ComputePassDescriptor,
    ComputePipelineDescriptor, Face, PipelineCache, ShaderStages,
};
use bevy::render::renderer::RenderDevice;
use bevy::render::{RenderApp, RenderStage};
use bevy::time::Time;
use bevy::{
    asset::load_internal_asset,
    core::Name,
    math::{Quat, Vec3},
    pbr::{AlphaMode, DirectionalLight, DirectionalLightBundle},
    prelude::{
        default,
        shape::{Cube, Icosphere},
        App, Assets, Commands, HandleUntyped, Mesh, Plugin, ResMut, Shader, Transform,
    },
    DefaultPlugins,
};

use bevy_instancing::prelude::{
    CustomMaterial, CustomMaterialPlugin, GpuColorMeshInstance, IndirectRenderingPlugin,
    InstanceSlice, InstanceSliceTarget, InstanceSliceBundle, InstanceSliceRange,
    MaterialInstanced,
};
use bytemuck::{Pod, Zeroable};

// Test indirect rendering
fn main() {
    let mut app = App::default();

    app.add_plugins(DefaultPlugins)
        .add_plugin(IndirectRenderingPlugin)
        .add_plugin(CustomMaterialPlugin)
        .add_plugin(InstanceComputePlugin::<CustomMaterial>::default());

    app.add_startup_system(setup_instancing);

    app.add_system(instance_compute_time);

    app.run()
}

pub const INSTANCE_SHADER_HANDLE: HandleUntyped =
    HandleUntyped::weak_from_u64(Shader::TYPE_UUID, 9845106354689849797);

struct InstanceCompute;

impl Into<Cow<'static, str>> for InstanceCompute {
    fn into(self) -> Cow<'static, str> {
        Cow::Borrowed("instance_compute")
    }
}
impl Into<NodeLabel> for InstanceCompute {
    fn into(self) -> NodeLabel {
        NodeLabel::Name(self.into())
    }
}

struct InstanceComputePlugin<M: MaterialInstanced>(PhantomData<M>);

impl<M: MaterialInstanced> Default for InstanceComputePlugin<M> {
    fn default() -> Self {
        Self(default())
    }
}

impl<M: MaterialInstanced> Plugin for InstanceComputePlugin<M> {
    fn build(&self, app: &mut App) {
        load_internal_asset!(
            app,
            INSTANCE_SHADER_HANDLE,
            "instances.wgsl",
            Shader::from_wgsl
        );

        app.add_plugin(ExtractComponentPlugin::<InstanceComputeUniform>::default());

        let render_app = app.sub_app_mut(RenderApp);
        render_app
            .init_resource::<InstanceComputePipeline>()
            .add_system_to_stage(RenderStage::Queue, queue_compute_instances::<M>);

        let mut render_graph = render_app.world.resource_mut::<RenderGraph>();
        render_graph.add_node(InstanceCompute, InstanceComputeNode);
        render_graph
            .add_node_edge(
                InstanceCompute,
                bevy::render::main_graph::node::CAMERA_DRIVER,
            )
            .unwrap();
    }
}

#[derive(Debug, Clone)]
pub struct InstanceComputePipeline {
    pipeline: CachedComputePipelineId,
    bind_group_layout: BindGroupLayout,
}

impl FromWorld for InstanceComputePipeline {
    fn from_world(world: &mut World) -> Self {
        let render_device = world.get_resource::<RenderDevice>().unwrap();
        let bind_group_layout =
            render_device.create_bind_group_layout(&BindGroupLayoutDescriptor {
                label: Some("instance buffer bind group"),
                entries: &[
                    BindGroupLayoutEntry {
                        binding: 0,
                        visibility: ShaderStages::COMPUTE,
                        ty: BindingType::Buffer {
                            ty: BufferBindingType::Uniform,
                            has_dynamic_offset: false,
                            min_binding_size: None,
                        },
                        count: None,
                    },
                    BindGroupLayoutEntry {
                        binding: 1,
                        visibility: ShaderStages::COMPUTE,
                        ty: BindingType::Buffer {
                            ty: BufferBindingType::Storage { read_only: false },
                            has_dynamic_offset: false,
                            min_binding_size: None,
                        },
                        count: None,
                    },
                ],
            });

        let mut pipeline_cache = world.resource_mut::<PipelineCache>();
        let pipeline = pipeline_cache.queue_compute_pipeline(ComputePipelineDescriptor {
            label: Some("instance compute".into()),
            layout: Some(vec![bind_group_layout.clone()]),
            shader: INSTANCE_SHADER_HANDLE.typed::<Shader>(),
            shader_defs: vec![],
            entry_point: Cow::from("instances"),
        });

        InstanceComputePipeline {
            pipeline,
            bind_group_layout,
        }
    }
}

struct InstanceComputeNode;

struct InstanceComputeQueue(Vec<InstanceComputeJob>);

struct InstanceComputeJob {
    bind_group: BindGroup,
    instance_count: u64,
}

const WORKGROUP_SIZE: u64 = 64;

impl Node for InstanceComputeNode {
    fn run(
        &self,
        _graph: &mut bevy::render::render_graph::RenderGraphContext,
        render_context: &mut bevy::render::renderer::RenderContext,
        world: &bevy::prelude::World,
    ) -> Result<(), bevy::render::render_graph::NodeRunError> {
        debug!("InstanceComputeNode::run");
        let pipeline_cache = world.resource::<PipelineCache>();
        let pipeline = world.resource::<InstanceComputePipeline>();

        if let Some(instance_pipeline) = pipeline_cache.get_compute_pipeline(pipeline.pipeline) {
            debug!("Instance pipeline valid");
            let compute_jobs = &world.resource::<InstanceComputeQueue>().0;
            for compute_job in compute_jobs {
                debug!(
                    "Running compute job with {} instances",
                    compute_job.instance_count
                );

                let mut pass = render_context
                    .command_encoder
                    .begin_compute_pass(&ComputePassDescriptor::default());

                pass.set_bind_group(0, &compute_job.bind_group, &[]);

                let instance_workgroups =
                    (compute_job.instance_count / WORKGROUP_SIZE).max(1) as u32;

                pass.set_pipeline(instance_pipeline);
                pass.dispatch_workgroups(instance_workgroups, 1, 1);
            }
        }

        Ok(())
    }
}

pub fn queue_compute_instances<M: MaterialInstanced>(
    pipeline: Res<InstanceComputePipeline>,
    render_device: Res<RenderDevice>,
    query_instance_block: Query<
        (
            Entity,
            &InstanceComputeUniform,
            &InstanceSliceRange,
            &InstanceSliceTarget,
        ),
        With<Handle<M>>,
    >,
    mut commands: Commands,
) {
    debug!("queue_compute_instances::<{}>", std::any::type_name::<M>());
    let mut instance_compute_queue = vec![];

    for (
        instance_block_entity,
        instance_compute_uniform,
        instance_block_range,
        instance_block_buffer,
    ) in query_instance_block.iter()
    {
        let uniform_buffer = render_device.create_buffer_with_data(&BufferInitDescriptor {
            label: Some("uniform buffer"),
            contents: bytemuck::bytes_of(instance_compute_uniform),
            usage: BufferUsages::UNIFORM,
        });

        debug!("Instance block {instance_block_entity:?}");
        let bind_group = render_device.create_bind_group(&BindGroupDescriptor {
            label: None,
            layout: &pipeline.bind_group_layout,
            entries: &[
                BindGroupEntry {
                    binding: 0,
                    resource: uniform_buffer.as_entire_binding(),
                },
                BindGroupEntry {
                    binding: 1,
                    resource: BindingResource::Buffer(BufferBinding {
                        buffer: &instance_block_buffer.buffer,
                        offset: std::mem::size_of::<GpuColorMeshInstance>() as u64
                            * instance_block_range.offset,
                        size: NonZeroU64::new(
                            std::mem::size_of::<GpuColorMeshInstance>() as u64
                                * instance_block_range.instance_count,
                        ),
                    }),
                },
            ],
        });

        debug!(
            "Queueing board compute job for {} cells",
            instance_block_range.instance_count
        );
        instance_compute_queue.push(InstanceComputeJob {
            bind_group,
            instance_count: instance_block_range.instance_count,
        });
    }

    commands.insert_resource(InstanceComputeQueue(instance_compute_queue));
}

#[derive(Debug, Default, Copy, Clone, Component, Pod, Zeroable)]
#[repr(C)]
pub struct InstanceComputeUniform {
    time: f32,
    _pad0: [f32; 3],
    normal: Vec3,
    _pad1: f32,
    tangent: Vec3,
    _pad2: f32,
    tint: Vec3,
    _pad3: f32,
}

impl ExtractComponent for InstanceComputeUniform {
    type Query = Read<Self>;

    type Filter = ();

    fn extract_component(item: bevy::ecs::query::QueryItem<Self::Query>) -> Self {
        *item
    }
}

fn setup_instancing(
    mut meshes: ResMut<Assets<Mesh>>,
    mut board_materials: ResMut<Assets<CustomMaterial>>,
    mut commands: Commands,
) {
    // Perspective camera
    commands.spawn_bundle(Camera3dBundle {
        transform: Transform::from_xyz(-50.0, 50.0, 50.0).looking_at(Vec3::ZERO, Vec3::Y),
        ..default()
    });

    // Directional Light
    commands.spawn().insert_bundle(DirectionalLightBundle {
        directional_light: DirectionalLight {
            illuminance: 4000.,
            ..default()
        },
        transform: Transform {
            // Workaround: Pointing straight up or down prevents directional shadow from rendering
            rotation: Quat::from_rotation_x(-std::f32::consts::FRAC_PI_2 * 0.6),
            ..default()
        },
        ..default()
    });

    // Populate scene
    let mesh_cube = meshes.add(Cube::default().into());
    let mesh_sphere = meshes.add(
        Icosphere {
            radius: 0.75,
            ..default()
        }
        .into(),
    );

    let material_front = board_materials.add(CustomMaterial {
        alpha_mode: AlphaMode::Blend,
        cull_mode: Some(Face::Back),
    });

    let material_back = board_materials.add(CustomMaterial {
        alpha_mode: AlphaMode::Blend,
        cull_mode: Some(Face::Front),
    });

    commands
        .spawn()
        .insert(Name::new("Back Face Cube Instance Block"))
        .insert_bundle(InstanceSliceBundle {
            material: material_back.clone(),
            mesh: mesh_cube.clone(),
            mesh_instance_slice: InstanceSlice {
                instance_count: 200,
            },
            ..default()
        })
        .insert(InstanceComputeUniform {
            tint: Vec3::new(1.0, 1.0, 1.0),
            normal: Vec3::X,
            tangent: -Vec3::Y,
            ..default()
        });

    commands
        .spawn()
        .insert(Name::new("Front Face Cube Instance Block"))
        .insert_bundle(InstanceSliceBundle {
            material: material_front.clone(),
            mesh: mesh_cube.clone(),
            mesh_instance_slice: InstanceSlice {
                instance_count: 200,
            },
            ..default()
        })
        .insert(InstanceComputeUniform {
            tint: Vec3::new(1.0, 0.0, 0.0),
            normal: -Vec3::X,
            tangent: Vec3::Y,
            ..default()
        });

    commands
        .spawn()
        .insert(Name::new("Back Face Sphere Instance Block"))
        .insert_bundle(InstanceSliceBundle {
            material: material_back.clone(),
            mesh: mesh_sphere.clone(),
            mesh_instance_slice: InstanceSlice {
                instance_count: 200,
            },
            ..default()
        })
        .insert(InstanceComputeUniform {
            tint: Vec3::new(0.0, 1.0, 0.0),
            normal: -Vec3::Z,
            tangent: -Vec3::Y,
            ..default()
        });

    commands
        .spawn()
        .insert(Name::new("Front Face Sphere Instance Block"))
        .insert_bundle(InstanceSliceBundle {
            material: material_front.clone(),
            mesh: mesh_sphere.clone(),
            mesh_instance_slice: InstanceSlice {
                instance_count: 200,
            },
            ..default()
        })
        .insert(InstanceComputeUniform {
            tint: Vec3::new(0.0, 0.0, 1.0),
            normal: Vec3::Z,
            tangent: Vec3::Y,
            ..default()
        });
}

fn instance_compute_time(time: Res<Time>, mut query_uniform: Query<&mut InstanceComputeUniform>) {
    for mut uniform in query_uniform.iter_mut() {
        uniform.time = time.seconds_since_startup() as f32;
    }
}
