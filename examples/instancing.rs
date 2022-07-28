use bevy::{
    core::Name,
    core_pipeline::{AlphaMask3d, Opaque3d, Transparent3d},
    math::{Quat, Vec3},
    pbr::{AlphaMode, DirectionalLight, DirectionalLightBundle},
    prelude::{
        default,
        shape::{Cube, Icosphere},
        App, Assets, Camera, Color, Commands, Component, EventWriter, Handle, Mesh,
        PerspectiveCameraBundle, Plugin, Res, ResMut, Transform, World,
    },
    render::{
        camera::{ActiveCamera, Camera3d, CameraTypePlugin, RenderTarget},
        render_graph::{NodeRunError, RenderGraph, RenderGraphContext, SlotValue},
        render_phase::RenderPhase,
        render_resource::Face,
        renderer::RenderContext,
        RenderApp, RenderStage,
    },
    window::{CreateWindow, PresentMode, WindowDescriptor, WindowId},
    DefaultPlugins,
};

use bevy_instancing::prelude::{
    BasicMaterial, BoardInstanceBundle, BoardMaterial, IndirectRenderingPlugin, InstanceBundle, CustomMaterialPlugin,
};

// Test indirect rendering
fn main() {
    let mut app = App::default();

    app.add_plugins(DefaultPlugins)
        .add_plugin(SecondWindowCameraPlugin)
        .add_plugin(IndirectRenderingPlugin)
        .add_plugin(CustomMaterialPlugin);

    app.add_startup_system(setup_instancing);

    app.run()
}

struct SecondWindowCameraPlugin;
impl Plugin for SecondWindowCameraPlugin {
    fn build(&self, app: &mut App) {
        // adds the `ActiveCamera<SecondWindowCamera3d>` resource and extracts the camera into the render world
        app.add_plugin(CameraTypePlugin::<SecondWindowCamera3d>::default());

        let render_app = app.sub_app_mut(RenderApp);

        // add `RenderPhase<Opaque3d>`, `RenderPhase<AlphaMask3d>` and `RenderPhase<Transparent3d>` camera phases
        render_app.add_system_to_stage(RenderStage::Extract, extract_second_camera_phases);

        // add a render graph node that executes the 3d subgraph
        let mut render_graph = render_app.world.resource_mut::<RenderGraph>();
        let second_window_node = render_graph.add_node("second_window_cam", SecondWindowDriverNode);
        render_graph
            .add_node_edge(
                bevy::core_pipeline::node::MAIN_PASS_DEPENDENCIES,
                second_window_node,
            )
            .unwrap();
        render_graph
            .add_node_edge(
                bevy::core_pipeline::node::CLEAR_PASS_DRIVER,
                second_window_node,
            )
            .unwrap();
    }
}

struct SecondWindowDriverNode;
impl bevy::render::render_graph::Node for SecondWindowDriverNode {
    fn run(
        &self,
        graph: &mut RenderGraphContext,
        _: &mut RenderContext,
        world: &World,
    ) -> Result<(), NodeRunError> {
        if let Some(camera) = world.resource::<ActiveCamera<SecondWindowCamera3d>>().get() {
            graph.run_sub_graph(
                bevy::core_pipeline::draw_3d_graph::NAME,
                vec![SlotValue::Entity(camera)],
            )?;
        }

        Ok(())
    }
}

fn extract_second_camera_phases(
    mut commands: Commands,
    active: Res<ActiveCamera<SecondWindowCamera3d>>,
) {
    if let Some(entity) = active.get() {
        commands.get_or_spawn(entity).insert_bundle((
            RenderPhase::<Opaque3d>::default(),
            RenderPhase::<AlphaMask3d>::default(),
            RenderPhase::<Transparent3d>::default(),
        ));
    }
}

#[derive(Component, Default)]
struct SecondWindowCamera3d;

fn setup_instancing(
    mut create_window_events: EventWriter<CreateWindow>,
    mut meshes: ResMut<Assets<Mesh>>,
    mut board_materials: ResMut<Assets<BoardMaterial>>,
    mut commands: Commands,
) {
    let window_id = WindowId::new();

    // Perspective camera
    commands.spawn_bundle(PerspectiveCameraBundle::<Camera3d> {
        transform: Transform::from_xyz(-50.0, 50.0, 50.0).looking_at(Vec3::ZERO, Vec3::Y),
        ..PerspectiveCameraBundle::new()
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

    // sends out a "CreateWindow" event, which will be received by the windowing backend
    create_window_events.send(CreateWindow {
        id: window_id,
        descriptor: WindowDescriptor {
            width: 800.,
            height: 600.,
            present_mode: PresentMode::Immediate,
            title: "Second window".to_string(),
            ..default()
        },
    });

    // second window camera
    commands.spawn_bundle(PerspectiveCameraBundle {
        camera: Camera {
            target: RenderTarget::Window(window_id),
            ..default()
        },
        transform: Transform::from_xyz(50.0, 0.0, 50.0).looking_at(Vec3::ZERO, Vec3::Y),
        marker: SecondWindowCamera3d,
        ..PerspectiveCameraBundle::new()
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

    let meshes = [mesh_cube, mesh_sphere];

    let material_basic = Handle::<BasicMaterial>::default();

    let material_opaque_no_cull = board_materials.add(BoardMaterial {
        alpha_mode: AlphaMode::Opaque,
        cull_mode: None,
    });

    let material_mask_no_cull = board_materials.add(BoardMaterial {
        alpha_mode: AlphaMode::Mask(0.5),
        cull_mode: None,
    });

    let material_blend_no_cull = board_materials.add(BoardMaterial {
        alpha_mode: AlphaMode::Blend,
        cull_mode: None,
    });

    let material_opaque_cull_front = board_materials.add(BoardMaterial {
        alpha_mode: AlphaMode::Opaque,
        cull_mode: Some(Face::Front),
    });

    let material_mask_cull_front = board_materials.add(BoardMaterial {
        alpha_mode: AlphaMode::Mask(0.5),
        cull_mode: Some(Face::Front),
    });

    let material_blend_cull_front = board_materials.add(BoardMaterial {
        alpha_mode: AlphaMode::Blend,
        cull_mode: Some(Face::Front),
    });

    let material_opaque_cull_back = board_materials.add(BoardMaterial {
        alpha_mode: AlphaMode::Opaque,
        cull_mode: Some(Face::Back),
    });

    let material_mask_cull_back = board_materials.add(BoardMaterial {
        alpha_mode: AlphaMode::Mask(0.5),
        cull_mode: Some(Face::Back),
    });

    let material_blend_cull_back = board_materials.add(BoardMaterial {
        alpha_mode: AlphaMode::Blend,
        cull_mode: Some(Face::Back),
    });

    let materials = [
        material_opaque_no_cull,
        material_mask_no_cull,
        material_blend_no_cull.clone(),
        material_opaque_cull_front,
        material_mask_cull_front,
        material_blend_cull_front.clone(),
        material_opaque_cull_back,
        material_mask_cull_back,
        material_blend_cull_back.clone(),
    ];

    let colors = [
        Color::WHITE,
        Color::RED,
        Color::GREEN,
        Color::BLUE,
        Color::BLACK,
    ];

    for (x, mesh) in meshes.into_iter().enumerate() {
        commands
            .spawn()
            .insert(Name::new("Cube Instance"))
            .insert_bundle(InstanceBundle::<BasicMaterial> {
                mesh: mesh.clone(),
                material: material_basic.clone(),
                transform: Transform::from_xyz(x as f32 * 1.5, 0.0, 0.0).into(),
                ..default()
            });

        for (y, material) in materials.iter().enumerate() {
            for (z, mut color) in colors.into_iter().enumerate() {
                if *material == material_blend_no_cull
                    || *material == material_blend_cull_front
                    || *material == material_blend_cull_back
                {
                    color.set_a(0.5);
                }
                commands
                    .spawn()
                    .insert(Name::new(format!("Cube Instance ({x:}, {y:}, {z:})")))
                    .insert_bundle(BoardInstanceBundle {
                        instance_bundle: InstanceBundle {
                            mesh: mesh.clone(),
                            material: material.clone(),
                            transform: Transform::from_xyz(
                                x as f32 * 1.5,
                                1.5 + y as f32 * 1.5,
                                z as f32 * -1.5,
                            )
                            .into(),
                            ..default()
                        },
                        mesh_instance_color: color.into(),
                    });
            }
        }
    }
}
