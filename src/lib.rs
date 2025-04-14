use essential::{assets::asset_server::AssetServer, time::Time, transform::Transform};
use std::sync::Arc;

use app::{
    plugins::{AssetManagerPlugin, TimePlugin},
    App,
};
use ecs::{query::Query, resource::Res};
use glam::{Quat, Vec2, Vec3};
use render::{
    components::camera::Camera,
    mesh::{vertex::Vertex, Mesh, MeshAsset, ModelAsset},
    plugin::RenderPlugin,
};

use window::{
    input::{Input, InputState},
    plugin::WindowPlugin,
};

const NUM_INSTANCES_PER_ROW: u32 = 10;
const INSTANCE_DISPLACEMENT: Vec3 = Vec3::new(0 as f32 * 0.5, 0.0, 0 as f32 * 0.5);

pub(crate) const VERTICES: &[Vertex] = &[
    // Changed
    Vertex {
        pos_coords: [-0.0868241, 0.49240386, 0.0],
        uv_coords: [0.4131759, 0.00759614],
    }, // A
    Vertex {
        pos_coords: [-0.49513406, 0.06958647, 0.0],
        uv_coords: [0.0048659444, 0.43041354],
    }, // B
    Vertex {
        pos_coords: [-0.21918549, -0.44939706, 0.0],
        uv_coords: [0.28081453, 0.949397],
    }, // C
    Vertex {
        pos_coords: [0.35966998, -0.3473291, 0.0],
        uv_coords: [0.85967, 0.84732914],
    }, // D
    Vertex {
        pos_coords: [0.44147372, 0.2347359, 0.0],
        uv_coords: [0.9414737, 0.2652641],
    }, // E
];

pub(crate) const INDICES: &[u32] = &[0, 1, 4, 1, 2, 4, 2, 3, 4];

#[cfg(target_arch = "wasm32")]
use wasm_bindgen::prelude::wasm_bindgen;
use winit::keyboard::{KeyCode, PhysicalKey};

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
        .add_system(app::update_group::UpdateGroup::Update, move_around)
        .add_system(app::update_group::UpdateGroup::Update, rotate_meshes);

    spawn_stuff(&mut app);

    app.run();
}

fn spawn_stuff(app: &mut app::App) {
    let mesh_asset = Arc::new(MeshAsset {
        vertices: VERTICES.to_vec(),
        indices: INDICES.to_vec(),
    });

    let server = app.get_mut_resource::<AssetServer>().unwrap();
    let handle = server.load::<ModelAsset>("res/teapot.obj");

    // Lets make a bunch of instances
    for z in 0..NUM_INSTANCES_PER_ROW {
        for x in 0..NUM_INSTANCES_PER_ROW {
            let pos = Vec3::Z * z as f32 + Vec3::X * x as f32 - INSTANCE_DISPLACEMENT;

            app.spawn((
                Mesh {
                    mesh_asset: mesh_asset.clone(),
                    handle: handle.clone(),
                },
                Transform::from_translation_rotation(pos, Quat::IDENTITY),
            ));
        }
    }

    let camera = Camera::new(1.0, 45.0, 0.1, 100.0);
    let cam_pos = Vec3::new(0.0, 0.0, 2.0);
    let cam_rot = Quat::look_at_rh(cam_pos, Vec3::ZERO, Vec3::Y);
    let camera_transform = Transform::from_translation_rotation(cam_pos, cam_rot);

    app.spawn((camera, camera_transform));
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

fn rotate_meshes(meshes: Query<(&Mesh, &mut Transform)>, time: Res<Time>) {
    for (_, transform) in meshes.iter() {
        transform.rotation *= Quat::from_axis_angle(Vec3::Y, time.delta() * 100.0);
    }
}
