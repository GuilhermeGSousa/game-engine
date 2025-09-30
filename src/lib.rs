use std::f32::consts::PI;

use essential::{
    assets::asset_server::AssetServer,
    time::Time,
    transform::{GlobalTranform, Transform},
};

use app::{
    plugins::{AssetManagerPlugin, TimePlugin, TransformPlugin},
    App,
};
use ecs::{
    command::CommandQueue,
    component::Component,
    entity::Entity,
    query::{query_filter::With, Query},
    resource::{Res, ResMut},
};
use glam::{Quat, Vec3, Vec4};
use physics::{physics_state::PhysicsState, plugin::PhysicsPlugin, rigid_body::RigidBody};
use render::{
    assets::texture::TextureUsageSettings,
    components::{
        camera::Camera,
        light::{LighType, Light, SpotLight},
        mesh_component::MeshComponent,
        render_entity::RenderEntity,
        skybox::Skybox,
    },
    plugin::RenderPlugin,
};

use ui::plugin::UIPlugin;
use wgpu_types::{
    Extent3d, TextureDescriptor, TextureDimension, TextureFormat, TextureUsages,
    TextureViewDescriptor, TextureViewDimension,
};
use window::{
    input::{Input, InputState},
    plugin::WindowPlugin,
};

use winit::keyboard::{KeyCode, PhysicalKey};

use crate::game_ui::render_ui;

pub mod game_ui;

#[allow(dead_code)]

// const MESH_ASSET: &str = "res/sphere.obj";
// const GLB_ASSET: &str = "res/duck.glb";
const GROUND_ASSET: &str = "res/ground.obj";
const SKYBOX_TEXTURE: &str = "res/Ryfjallet-cubemap.png";

#[derive(Component)]
struct Player;

pub fn run_game() {
    cfg_if::cfg_if! {
        if #[cfg(target_arch = "wasm32")] {
            std::panic::set_hook(Box::new(console_error_panic_hook::hook));
            console_log::init_with_level(log::Level::Debug).expect("Couldn't initialize logger");
        } else {
            env_logger::init();
        }
    }

    let mut app = App::empty();
    app.register_plugin(TimePlugin)
        .register_plugin(AssetManagerPlugin)
        .register_plugin(WindowPlugin)
        .register_plugin(RenderPlugin)
        .register_plugin(TransformPlugin)
        .register_plugin(UIPlugin)
        .register_plugin(PhysicsPlugin)
        .add_system(app::update_group::UpdateGroup::Update, move_around)
        .add_system(
            app::update_group::UpdateGroup::Update,
            spawn_on_button_press,
        )
        .add_system(
            app::update_group::UpdateGroup::Update,
            despawn_on_button_press,
        )
        .add_system(app::update_group::UpdateGroup::Update, spawn_with_collider)
        // .add_system(app::update_group::UpdateGroup::Update, move_light_to_player)
        .add_system(app::update_group::UpdateGroup::Render, render_ui)
        .add_system(app::update_group::UpdateGroup::Startup, spawn_floor)
        .add_system(app::update_group::UpdateGroup::Startup, spawn_player);

    app.run();
}

fn spawn_player(mut cmd: CommandQueue, asset_server: Res<AssetServer>) {
    let camera = Camera::default();
    let skybox = Skybox {
        texture: asset_server.load_with_usage_settings(
            SKYBOX_TEXTURE,
            TextureUsageSettings {
                texture_descriptor: TextureDescriptor {
                    label: Some("cubemap_texture"),
                    size: Extent3d {
                        width: 256,
                        height: 256,
                        depth_or_array_layers: 6,
                    },

                    mip_level_count: 1,
                    sample_count: 1,
                    dimension: TextureDimension::D2,
                    format: TextureFormat::Rgba8UnormSrgb,
                    usage: TextureUsages::TEXTURE_BINDING | TextureUsages::COPY_DST,
                    view_formats: &[],
                },
                texture_view_descriptor: TextureViewDescriptor {
                    dimension: Some(TextureViewDimension::Cube),
                    ..Default::default()
                },
            },
        ),
    };

    let light = Light {
        color: Vec4::new(1.0, 0.0, 1.0, 1.0),
        intensity: 10.0,
        light_type: LighType::Spot(SpotLight {
            cone_angle: 50.0 * PI / 180.0,
        }),
    };

    let mut light_transform =
        Transform::from_translation_rotation(Vec3::new(0.0, 2.0, 0.0), Quat::IDENTITY);
    light_transform.rotation = Quat::from_euler(glam::EulerRot::XYZ, PI / 2.0, 0.0, 0.0);

    let parent = cmd.spawn((
        Player,
        Transform::from_translation_rotation(Vec3::ZERO, Quat::IDENTITY),
    ));

    let child = cmd.spawn((
        camera,
        skybox,
        Transform::from_translation_rotation(Vec3::new(0.0, 2.0, 0.0), Quat::IDENTITY),
        RenderEntity::new(),
    ));

    cmd.add_child(parent, child);

    cmd.spawn((light, light_transform, RenderEntity::new()));
}

fn spawn_floor(
    mut _cmd: CommandQueue,
    mut _physics_state: ResMut<PhysicsState>,
    _asset_server: Res<AssetServer>,
) {
    // let height = 1.0;
    // let ground_mesh = asset_server.load::<ObjAsset>(GROUND_ASSET);
    // let ground_transform =
    //     Transform::from_translation_rotation(Vec3::Y * (-2.0 * height), Quat::IDENTITY);
    // let ground_colider = physics_state.make_cuboid(100.0, height, 100.0, &ground_transform, None);

    // cmd.spawn((
    //     MeshComponent {
    //         handle: ground_mesh,
    //     },
    //     RenderEntity::Uninitialized,
    //     ground_colider,
    //     ground_transform,
    // ));
}

fn move_around(
    players: Query<&mut Transform, With<Player>>,
    cameras: Query<&mut Transform, With<Camera>>,
    input: Res<Input>,
    time: Res<Time>,
) {
    let mut player_transform = players.iter().next().unwrap();
    let mut camera_transform = cameras.iter().next().unwrap();

    let displacement = 10.0 * time.delta().as_secs_f32();

    let key_d = input.get_key_state(PhysicalKey::Code(KeyCode::KeyD));
    let key_a = input.get_key_state(PhysicalKey::Code(KeyCode::KeyA));
    let key_w = input.get_key_state(PhysicalKey::Code(KeyCode::KeyW));
    let key_s = input.get_key_state(PhysicalKey::Code(KeyCode::KeyS));

    if key_d == InputState::Pressed || key_d == InputState::Down {
        let right = player_transform.right();
        player_transform.translation += right * displacement;
    }

    if key_a == InputState::Pressed || key_a == InputState::Down {
        let left = player_transform.left();
        player_transform.translation += left * displacement;
    }

    if key_w == InputState::Pressed || key_w == InputState::Down {
        let forward = player_transform.forward();
        player_transform.translation += forward * displacement;
    }

    if key_s == InputState::Pressed || key_s == InputState::Down {
        let back = player_transform.backward();
        player_transform.translation += back * displacement;
    }

    let sensitivity = -0.5;
    let mouse_delta = input.mouse_delta();
    let yaw_delta = sensitivity * mouse_delta.x * time.delta().as_secs_f32();
    let pitch_delta = sensitivity * mouse_delta.y * time.delta().as_secs_f32();
    player_transform.rotation *= Quat::from_axis_angle(Vec3::Y, yaw_delta);
    camera_transform.rotation *= Quat::from_axis_angle(Vec3::X, pitch_delta);
}

fn spawn_on_button_press(
    cameras: Query<(&Camera, &Transform)>,
    mut _cmd: CommandQueue,
    input: Res<Input>,
    _asset_server: Res<AssetServer>,
) {
    let (_, pos) = cameras.iter().next().expect("No camera found");
    let key_p = input.get_key_state(PhysicalKey::Code(KeyCode::KeyP));

    let mut mesh_transform = pos.clone();
    mesh_transform.translation = pos.translation + pos.forward() * 50.0;
    if key_p == InputState::Pressed {
        // let handle = asset_server.load::<ObjAsset>(MESH_ASSET);
        // cmd.spawn((
        //     MeshComponent { handle },
        //     mesh_transform,
        //     RenderEntity::new(),
        // ));
    }
}

fn spawn_with_collider(
    cameras: Query<(&Camera, &GlobalTranform)>,
    mut _cmd: CommandQueue,
    input: Res<Input>,
    _asset_server: Res<AssetServer>,
    mut physics_state: ResMut<PhysicsState>,
) {
    let (_, pos) = cameras.iter().next().expect("No camera found");

    let key_r = input.get_key_state(PhysicalKey::Code(KeyCode::KeyR));

    if key_r == InputState::Pressed {
        // let spawn_point = pos.translation() + pos.forward() * 10.0;
        // let cube_transform = Transform::from_translation_rotation(spawn_point, Quat::IDENTITY);
        // let mut rigid_body = RigidBody::new(&cube_transform, &mut physics_state);

        // let collider = physics_state.make_sphere(&mut rigid_body, 1.0);

        // cmd.spawn((
        //     MeshComponent {
        //         handle: asset_server.load::<ObjAsset>(MESH_ASSET),
        //     },
        //     rigid_body,
        //     collider,
        //     cube_transform.clone(),
        //     RenderEntity::new(),
        // ));
    }
}

fn despawn_on_button_press(
    meshes: Query<(Entity, &MeshComponent, &mut Transform)>,
    mut cmd: CommandQueue,
    input: Res<Input>,
) {
    let key_d = input.get_key_state(PhysicalKey::Code(KeyCode::KeyL));

    if key_d == InputState::Pressed {
        for (entity, _, _) in meshes.iter() {
            cmd.despawn(entity);
        }
    }
}

#[allow(dead_code)]
fn move_light_to_player(
    cameras: Query<(&Camera, &Transform)>,
    light: Query<(&Light, &mut Transform)>,
) {
    let (_, transform_cam) = cameras.iter().next().unwrap();
    let (_, mut transform_light) = light.iter().next().unwrap();

    transform_light.translation =
        transform_cam.translation + transform_cam.forward() * 2.0 + transform_cam.up();
}
