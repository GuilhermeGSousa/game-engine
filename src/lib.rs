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
use physics::plugin::PhysicsPlugin;
use render::{
    assets::mesh::Mesh,
    components::{camera::Camera, mesh_component::MeshComponent, render_entity::RenderEntity},
    plugin::RenderPlugin,
};

use ui::plugin::UIPlugin;
use window::{
    input::{Input, InputState},
    plugin::WindowPlugin,
};

const NUM_INSTANCES_PER_ROW: u32 = 10;
const INSTANCE_DISPLACEMENT: Vec3 = Vec3::new(0 as f32 * 0.5, 0.0, 0 as f32 * 0.5);

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
        .add_system(app::update_group::UpdateGroup::Update, rotate_meshes)
        .add_system(app::update_group::UpdateGroup::Render, render_ui);

    spawn_player(&mut app);

    app.run();
}

fn spawn_player(app: &mut app::App) {
    let camera = Camera::new(1.0, 45.0, 0.1, 100.0);
    let cam_pos = Vec3::new(0.0, 0.0, 2.0);
    let cam_rot = Quat::look_at_rh(cam_pos, Vec3::ZERO, Vec3::Y);
    let camera_transform = Transform::from_translation_rotation(cam_pos, cam_rot);

    app.spawn((camera, camera_transform, RenderEntity::new()));
}

fn move_around(cameras: Query<(&Camera, &mut Transform)>, input: Res<Input>) {
    let (_, transform) = cameras.iter().next().unwrap();

    let key_d = input.get_key_state(PhysicalKey::Code(KeyCode::KeyD));
    let key_a = input.get_key_state(PhysicalKey::Code(KeyCode::KeyA));
    let key_w = input.get_key_state(PhysicalKey::Code(KeyCode::KeyW));
    let key_s = input.get_key_state(PhysicalKey::Code(KeyCode::KeyS));

    if key_d == InputState::Pressed || key_d == InputState::Down {
        transform.translation += transform.right() * 0.1;
    }

    if key_a == InputState::Pressed || key_a == InputState::Down {
        transform.translation += transform.left() * 0.1;
    }

    if key_w == InputState::Pressed || key_w == InputState::Down {
        transform.translation += transform.forward() * 0.1;
    }

    if key_s == InputState::Pressed || key_s == InputState::Down {
        transform.translation += transform.backward() * 0.1;
    }

    transform.translation.y = 0.0; // Keep the camera on the ground

    let mouse_delta = input.get_mouse_delta();
    if mouse_delta != Vec2::ZERO {
        transform.rotation *= Quat::from_axis_angle(Vec3::Y, mouse_delta.x * 0.01);
    }
}

fn spawn_on_button_press(
    cameras: Query<(&Camera, &mut Transform)>,
    mut cmd: CommandQueue,
    input: Res<Input>,
    asset_server: ResMut<AssetServer>,
) {
    let (_, pos) = cameras.iter().next().expect("No camera found");
    let key_p = input.get_key_state(PhysicalKey::Code(KeyCode::KeyP));

    if key_p == InputState::Pressed {
        let handle = asset_server.load::<Mesh>("res/cube.obj");
        cmd.spawn((MeshComponent { handle }, pos.clone(), RenderEntity::new()));
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

fn rotate_meshes(meshes: Query<(Entity, &MeshComponent, &mut Transform)>, time: Res<Time>) {
    for (_, _, transform) in meshes.iter() {
        transform.rotation *= Quat::from_axis_angle(Vec3::Y, time.delta() * 100.0);
    }
}
