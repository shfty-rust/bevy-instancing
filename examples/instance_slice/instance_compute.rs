//! Demonstration of InstanceSlice compute functionality
//!
//! Also highlights alpha ordering behaviour for transparent instance blocks;
//! batch order is visible when instances from different blocks draw on top
//! of one another.
//!

use bevy::ecs::system::lifetimeless::Read;
use bevy::prelude::{Camera3dBundle, Component, Query, Res};
use bevy::reflect::TypeUuid;
use bevy::render::extract_component::ExtractComponent;
use bevy::render::render_resource::{AsBindGroup, Face, ShaderRef};
use bevy::time::Time;
use bevy::{
    core::Name,
    math::{Quat, Vec3},
    pbr::{AlphaMode, DirectionalLight, DirectionalLightBundle},
    prelude::{
        default,
        shape::{Cube, Icosphere},
        App, Assets, Commands, Mesh, ResMut, Transform,
    },
    DefaultPlugins,
};

use bevy_instancing::prelude::{
    ColorMeshInstance, CustomMaterial, CustomMaterialPlugin, IndirectRenderingPlugin,
    InstanceCompute, InstanceComputePlugin, InstanceSlice, InstanceSliceBundle,
};

// Test indirect rendering
fn main() {
    let mut app = App::default();

    app.add_plugins(DefaultPlugins)
        .add_plugin(IndirectRenderingPlugin)
        .add_plugin(CustomMaterialPlugin);

    app.add_plugin(InstanceComputePlugin::<RadialSineInstances>::default());

    app.add_startup_system(setup_instancing);

    app.add_system(instance_compute_time);

    app.run()
}

#[derive(Debug, Default, Copy, Clone, Component, AsBindGroup)]
#[repr(C)]
pub struct RadialSineInstances {
    #[uniform(0)]
    time: f32,
    #[uniform(0)]
    normal: Vec3,
    #[uniform(0)]
    tangent: Vec3,
    #[uniform(0)]
    tint: Vec3,
}

impl From<&RadialSineInstances> for () {
    fn from(_: &RadialSineInstances) -> Self {}
}

impl ExtractComponent for RadialSineInstances {
    type Query = Read<Self>;

    type Filter = ();

    fn extract_component(item: bevy::ecs::query::QueryItem<Self::Query>) -> Self {
        *item
    }
}

impl InstanceCompute for RadialSineInstances {
    type Instance = ColorMeshInstance;

    fn shader() -> ShaderRef {
        "shader/radial_sine.wgsl".into()
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
        .insert(RadialSineInstances {
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
        .insert(RadialSineInstances {
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
        .insert(RadialSineInstances {
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
        .insert(RadialSineInstances {
            tint: Vec3::new(0.0, 0.0, 1.0),
            normal: Vec3::Z,
            tangent: Vec3::Y,
            ..default()
        });
}

fn instance_compute_time(time: Res<Time>, mut query_uniform: Query<&mut RadialSineInstances>) {
    for mut uniform in query_uniform.iter_mut() {
        uniform.time = time.seconds_since_startup() as f32;
    }
}
