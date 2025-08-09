use essential::{assets::asset_server::AssetServer, time::Time, transform::Transform};

use app::{
    plugins::{AssetManagerPlugin, TimePlugin},
    App,
};
use ecs::{
    command::CommandQueue,
    entity::Entity,
    query::Query,
    resource::{Res, ResMut},
};
use glam::{Quat, Vec2, Vec3};
use physics::{physics_state::PhysicsState, plugin::PhysicsPlugin, rigid_body::RigidBody};
use render::{
    assets::mesh::Mesh,
    components::{
        camera::Camera, light::Light, mesh_component::MeshComponent, render_entity::RenderEntity,
    },
    plugin::RenderPlugin,
};

use ui::plugin::UIPlugin;
use window::{
    input::{Input, InputState},
    plugin::WindowPlugin,
};

#[cfg(target_arch = "wasm32")]
use wasm_bindgen::prelude::wasm_bindgen;
use winit::keyboard::{KeyCode, PhysicalKey};

use crate::game_ui::render_ui;

pub mod game_ui;

#[cfg_attr(target_arch = "wasm32", wasm_bindgen(start))]
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
        .add_system(app::update_group::UpdateGroup::Update, move_light_to_player)
        .add_system(app::update_group::UpdateGroup::Render, render_ui)
        .add_system(app::update_group::UpdateGroup::Startup, spawn_floor);

    spawn_player(&mut app);

    app.run();
}

fn spawn_player(app: &mut app::App) {
    let camera = Camera::new(1.0, 45.0, 0.1, 100.0);
    let cam_pos = Vec3::new(0.0, 2.0, 0.0);
    let cam_rot = Quat::look_at_rh(Vec3::X, Vec3::ZERO, Vec3::Y);
    let camera_transform = Transform::from_translation_rotation(cam_pos, cam_rot);

    let light = Light::Point;
    app.spawn((camera, camera_transform.clone(), RenderEntity::new()));
    app.spawn((light, camera_transform.clone(), RenderEntity::new()));
}

fn spawn_floor(mut cmd: CommandQueue, mut physics_state: ResMut<PhysicsState>) {
    let height = 1.0;
    let ground_transform =
        Transform::from_translation_rotation(Vec3::Y * (-2.0 * height), Quat::IDENTITY);
    let ground_colider = physics_state.make_cuboid(100.0, height, 100.0, &ground_transform, None);

    cmd.spawn((ground_colider, ground_transform));
}

fn move_around(cameras: Query<(&Camera, &mut Transform)>, input: Res<Input>, time: Res<Time>) {
    let (_, transform) = cameras.iter().next().unwrap();

    let displacement = 50.0 * time.delta();

    let key_d = input.get_key_state(PhysicalKey::Code(KeyCode::KeyD));
    let key_a = input.get_key_state(PhysicalKey::Code(KeyCode::KeyA));
    let key_w = input.get_key_state(PhysicalKey::Code(KeyCode::KeyW));
    let key_s = input.get_key_state(PhysicalKey::Code(KeyCode::KeyS));

    if key_d == InputState::Pressed || key_d == InputState::Down {
        transform.translation += transform.right() * displacement;
    }

    if key_a == InputState::Pressed || key_a == InputState::Down {
        transform.translation += transform.left() * displacement;
    }

    if key_w == InputState::Pressed || key_w == InputState::Down {
        transform.translation += transform.forward() * displacement;
    }

    if key_s == InputState::Pressed || key_s == InputState::Down {
        transform.translation += transform.backward() * displacement;
    }

    transform.translation.y = 0.0; // Keep the camera on the ground

    let mouse_delta = input.get_mouse_delta();
    let rotation_delta = mouse_delta.x * 10.0 * time.delta();
    if mouse_delta != Vec2::ZERO {
        transform.rotation *= Quat::from_axis_angle(Vec3::Y, rotation_delta);
    }
}

fn spawn_on_button_press(
    cameras: Query<(&Camera, &Transform)>,
    mut cmd: CommandQueue,
    input: Res<Input>,
    asset_server: Res<AssetServer>,
) {
    let (_, pos) = cameras.iter().next().expect("No camera found");
    let key_p = input.get_key_state(PhysicalKey::Code(KeyCode::KeyP));

    if key_p == InputState::Pressed {
        let handle = asset_server.load::<Mesh>("res/cube.obj");
        cmd.spawn((MeshComponent { handle }, pos.clone(), RenderEntity::new()));
    }
}

fn spawn_with_collider(
    cameras: Query<(&Camera, &Transform)>,
    mut cmd: CommandQueue,
    input: Res<Input>,
    asset_server: Res<AssetServer>,
    mut physics_state: ResMut<PhysicsState>,
) {
    let (_, pos) = cameras.iter().next().expect("No camera found");

    let key_r = input.get_key_state(PhysicalKey::Code(KeyCode::KeyR));

    if key_r == InputState::Pressed {
        let spawn_point = pos.translation + pos.forward() * 10.0 + pos.up() * 5.0;
        let cube_transform = Transform::from_translation_rotation(spawn_point, Quat::IDENTITY);
        let mut rigid_body = RigidBody::new(&cube_transform, &mut physics_state);

        let collider = physics_state.make_sphere(&mut rigid_body, 1.0);

        cmd.spawn((
            MeshComponent {
                handle: asset_server.load::<Mesh>("res/cube.obj"),
            },
            rigid_body,
            collider,
            cube_transform.clone(),
            RenderEntity::new(),
        ));
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

fn move_light_to_player(
    cameras: Query<(&Camera, &Transform)>,
    light: Query<(&Light, &mut Transform)>,
) {
    let (_, transform_cam) = cameras.iter().next().unwrap();
    let (_, transform_light) = light.iter().next().unwrap();

    transform_light.translation = transform_cam.translation + transform_cam.forward() * 5.0;
}
