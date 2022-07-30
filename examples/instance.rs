use bevy::{
    core::Name,
    core_pipeline::{AlphaMask3d, Opaque3d, Transparent3d},
    math::{Quat, Vec3},
    pbr::{AlphaMode, DirectionalLight, DirectionalLightBundle},
    prelude::{
        default,
        shape::{Cube, Icosphere, Quad, Torus, UVSphere},
        App, AssetServer, Assets, Camera, Color, Commands, Component, EventWriter, Handle, Mesh,
        PerspectiveCameraBundle, PerspectiveProjection, Plugin, Res, ResMut, Transform, World,
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
    BasicMaterial, BasicMaterialPlugin, ColorInstanceBundle, CustomMaterial, CustomMaterialPlugin,
    IndirectRenderingPlugin, MeshInstanceBundle, TextureMaterial, TextureMaterialPlugin,
};

const USE_SECOND_CAMERA: bool = false;

// Test indirect rendering
fn main() {
    let mut app = App::default();

    app.add_plugins(DefaultPlugins)
        .add_plugin(SecondWindowCameraPlugin)
        .add_plugin(IndirectRenderingPlugin)
        .add_plugin(BasicMaterialPlugin)
        .add_plugin(CustomMaterialPlugin)
        .add_plugin(TextureMaterialPlugin);

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
    asset_server: Res<AssetServer>,
    mut create_window_events: EventWriter<CreateWindow>,
    mut meshes: ResMut<Assets<Mesh>>,
    mut board_materials: ResMut<Assets<CustomMaterial>>,
    mut texture_materials: ResMut<Assets<TextureMaterial>>,
    mut commands: Commands,
) {
    let window_id = WindowId::new();

    // Perspective camera
    commands.spawn_bundle(PerspectiveCameraBundle::<Camera3d> {
        transform: Transform::from_xyz(-50.0, 50.0, 50.0)
            .looking_at(Vec3::new(0.0, 12.0, 0.0), Vec3::Y),
        perspective_projection: PerspectiveProjection {
            fov: 15.0f32.to_radians(),
            ..default()
        },
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

    if USE_SECOND_CAMERA {
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
            perspective_projection: PerspectiveProjection {
                fov: 15.0f32.to_radians(),
                ..default()
            },
            transform: Transform::from_xyz(50.0, 0.0, 50.0)
                .looking_at(Vec3::new(0.0, 12.0, 0.0), Vec3::Y),
            marker: SecondWindowCamera3d,
            ..PerspectiveCameraBundle::new()
        });
    }

    // Populate scene
    let mesh_cube = meshes.add(Cube::default().into());
    let mesh_quad = meshes.add(Quad::default().into());

    let mesh_icosphere = meshes.add(
        Icosphere {
            radius: 0.5,
            ..default()
        }
        .into(),
    );

    let mesh_uv_sphere = meshes.add(
        UVSphere {
            radius: 0.5,
            ..default()
        }
        .into(),
    );

    let mesh_torus = meshes.add(
        Torus {
            radius: 0.25 + 0.125,
            ring_radius: 0.125,
            ..default()
        }
        .into(),
    );

    let meshes = [
        mesh_cube,
        mesh_quad,
        mesh_icosphere,
        mesh_uv_sphere,
        mesh_torus,
    ];

    let material_basic = Handle::<BasicMaterial>::default();

    let material_opaque_no_cull = board_materials.add(CustomMaterial {
        alpha_mode: AlphaMode::Opaque,
        cull_mode: None,
    });

    let material_mask_no_cull = board_materials.add(CustomMaterial {
        alpha_mode: AlphaMode::Mask(0.5),
        cull_mode: None,
    });

    let material_blend_no_cull = board_materials.add(CustomMaterial {
        alpha_mode: AlphaMode::Blend,
        cull_mode: None,
    });

    let material_opaque_cull_front = board_materials.add(CustomMaterial {
        alpha_mode: AlphaMode::Opaque,
        cull_mode: Some(Face::Front),
    });

    let material_mask_cull_front = board_materials.add(CustomMaterial {
        alpha_mode: AlphaMode::Mask(0.5),
        cull_mode: Some(Face::Front),
    });

    let material_blend_cull_front = board_materials.add(CustomMaterial {
        alpha_mode: AlphaMode::Blend,
        cull_mode: Some(Face::Front),
    });

    let material_opaque_cull_back = board_materials.add(CustomMaterial {
        alpha_mode: AlphaMode::Opaque,
        cull_mode: Some(Face::Back),
    });

    let material_mask_cull_back = board_materials.add(CustomMaterial {
        alpha_mode: AlphaMode::Mask(0.5),
        cull_mode: Some(Face::Back),
    });

    let material_blend_cull_back = board_materials.add(CustomMaterial {
        alpha_mode: AlphaMode::Blend,
        cull_mode: Some(Face::Back),
    });

    let custom_materials: &[Handle<CustomMaterial>] = &[
        material_opaque_no_cull,
        material_mask_no_cull,
        material_blend_no_cull,
        material_opaque_cull_front,
        material_mask_cull_front,
        material_blend_cull_front,
        material_opaque_cull_back,
        material_mask_cull_back,
        material_blend_cull_back,
    ];

    let material_texture_1 = texture_materials.add(TextureMaterial {
        texture: asset_server.load("texture/text_1.png"),
        alpha_mode: AlphaMode::Opaque,
        cull_mode: Some(Face::Back),
    });

    let material_texture_2 = texture_materials.add(TextureMaterial {
        texture: asset_server.load("texture/text_2.png"),
        alpha_mode: AlphaMode::Mask(0.2),
        cull_mode: Some(Face::Back),
    });

    let material_texture_3 = texture_materials.add(TextureMaterial {
        texture: asset_server.load("texture/text_3.png"),
        alpha_mode: AlphaMode::Blend,
        cull_mode: Some(Face::Back),
    });

    let material_texture_smiley = texture_materials.add(TextureMaterial {
        texture: asset_server.load("texture/text_smiley.png"),
        alpha_mode: AlphaMode::Opaque,
        cull_mode: Some(Face::Back),
    });

    let texture_materials = &[
        material_texture_1,
        material_texture_2,
        material_texture_3,
        material_texture_smiley,
    ];

    let colors = &[
        Color::rgba(1.0, 1.0, 1.0, 0.5),
        Color::rgba(1.0, 0.0, 0.0, 0.5),
        Color::rgba(0.0, 1.0, 0.0, 0.5),
        Color::BLUE,
    ];

    for (x, mesh) in meshes.into_iter().enumerate() {
        commands
            .spawn()
            .insert(Name::new("Basic Instance"))
            .insert_bundle(MeshInstanceBundle::<BasicMaterial> {
                mesh: mesh.clone(),
                material: material_basic.clone(),
                transform: Transform::from_xyz(x as f32 * 1.5, 0.0, 0.0).into(),
                ..default()
            });

        for (z, color) in colors.into_iter().copied().enumerate() {
            let mut y = 0;
            for material in custom_materials.iter() {
                commands
                    .spawn()
                    .insert(Name::new(format!("Custom Instance ({x:}, {y:}, {z:})")))
                    .insert_bundle(ColorInstanceBundle {
                        instance_bundle: MeshInstanceBundle {
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

                y += 1;
            }

            for material in texture_materials.iter() {
                commands
                    .spawn()
                    .insert(Name::new(format!("Texture Instance ({x:}, {y:}, {z:})")))
                    .insert_bundle(ColorInstanceBundle {
                        instance_bundle: MeshInstanceBundle {
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

                y += 1;
            }
        }
    }
}
