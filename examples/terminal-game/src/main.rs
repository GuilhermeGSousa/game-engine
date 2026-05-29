use std::f32::consts::PI;

use app::{
    plugins::{AssetManagerPlugin, TimePlugin, TransformPlugin},
    App,
};
use ecs::system::schedule::UpdateGroup;
use ecs::{command::CommandQueue, query::Query, resource::Res, Component, With};
use essential::{assets::asset_server::AssetServer, time::Time, transform::Transform};
use game_engine::DefaultPlugins;
use glam::{Quat, Vec3, Vec4};
use ratatui::crossterm::event::KeyCode;
use render::{
    assets::{material::StandardMaterial, mesh::Mesh, texture::Texture, vertex::Vertex},
    components::{
        camera::{Camera, RenderTarget},
        light::{LighType, Light, SpotLight},
        material_component::MaterialComponent,
        mesh_component::MeshComponent,
    },
    plugin::RenderPlugin,
    wgpu::naga::VectorSize::Quad,
};
use terminal_renderer::{
    terminal::TerminalContext, TerminalInput, TerminalOutput, TerminalRendererPlugin,
};

#[derive(Component)]
struct Cube;

fn main() {
    let mut app = App::new();
    app.register_plugin(AssetManagerPlugin)
        .register_plugin(TimePlugin)
        .register_plugin(RenderPlugin)
        .register_plugin(TransformPlugin)
        .register_plugin(TerminalRendererPlugin)
        .add_system(UpdateGroup::Startup, spawn_camera_terminal)
        .add_system(UpdateGroup::Startup, spawn_scene)
        .add_system(UpdateGroup::Update, rotate_cube)
        .add_system(UpdateGroup::Update, move_camera);

    // app.register_plugin(DefaultPlugins)
    //     .add_system(UpdateGroup::Startup, spawn_camera_windowed)
    //     .add_system(UpdateGroup::Startup, spawn_scene)
    //     .add_system(UpdateGroup::Update, rotate_cube);
    app.run();
}

fn spawn_camera_terminal(
    mut cmd: CommandQueue,
    asset_server: Res<AssetServer>,
    terminal: Res<TerminalContext>,
) {
    let terminal_size = terminal.size().unwrap();
    let rtt = asset_server.add(Texture::render_target(
        terminal_size.width as u32,
        terminal_size.height as u32,
    ));

    // Account for terminal cells being ~2x taller than wide
    let aspect = (terminal_size.width as f32 * 0.5) / terminal_size.height as f32;
    let camera = Camera {
        aspect,
        render_target: RenderTarget::texture(rtt),
        ..Camera::default()
    };
    cmd.spawn((
        camera,
        TerminalOutput,
        Transform::from_translation_rotation(Vec3::new(0.0, 0.0, 5.0), Quat::IDENTITY),
    ));
}

fn spawn_camera_windowed(mut cmd: CommandQueue) {
    let camera = Camera::default();
    cmd.spawn((
        camera,
        Transform::from_translation_rotation(Vec3::new(0.0, 0.0, 5.0), Quat::IDENTITY),
    ));
}

fn spawn_scene(mut cmd: CommandQueue, asset_server: Res<AssetServer>) {
    let mesh = asset_server.add(make_cube());
    let material = asset_server.add(StandardMaterial::new(None, None));
    cmd.spawn((
        Cube,
        MeshComponent { handle: mesh },
        MaterialComponent::<StandardMaterial> { handle: material },
        Transform::from_translation_rotation(Vec3::ZERO, Quat::IDENTITY),
    ));

    let light = Light {
        color: Vec4::new(1.0, 0.0, 1.0, 1.0),
        intensity: 10.0,
        light_type: LighType::Spot(SpotLight {
            cone_angle: 50.0 * PI / 180.0,
        }),
    };
    let mut light_transform =
        Transform::from_translation_rotation(Vec3::new(0.0, 2.0, 2.0), Quat::IDENTITY);
    light_transform.look_at(Vec3::ZERO, Vec3::Y);

    cmd.spawn((light, light_transform));
}

fn rotate_cube(cubes: Query<&mut Transform, With<Cube>>, time: Res<Time>) {
    let delta = time.delta().as_secs_f32();
    for mut transform in cubes.iter() {
        transform.rotation = transform.rotation * Quat::from_rotation_y(delta);
    }
}

fn move_camera(
    cameras: Query<&mut Transform, With<Camera>>,
    input: Res<TerminalInput>,
    time: Res<Time>,
) {
    let speed = 5.0 * time.delta().as_secs_f32();
    let rot_speed = 2.0 * time.delta().as_secs_f32();

    for mut transform in cameras.iter() {
        // WASD: translate along the camera's local axes
        if input.is_key_active(KeyCode::Char('z')) {
            let fwd = transform.forward();
            transform.translation += fwd * speed;
        }
        if input.is_key_active(KeyCode::Char('s')) {
            let bwd = transform.backward();
            transform.translation += bwd * speed;
        }
        if input.is_key_active(KeyCode::Char('q')) {
            let left = transform.left();
            transform.translation += left * speed;
        }
        if input.is_key_active(KeyCode::Char('d')) {
            let right = transform.right();
            transform.translation += right * speed;
        }

        // Arrow keys: yaw (left/right around world Y) and pitch (up/down around local X)
        if input.is_key_active(KeyCode::Left) {
            transform.rotation = Quat::from_rotation_y(rot_speed) * transform.rotation;
        }
        if input.is_key_active(KeyCode::Right) {
            transform.rotation = Quat::from_rotation_y(-rot_speed) * transform.rotation;
        }
        if input.is_key_active(KeyCode::Up) {
            transform.rotation = transform.rotation * Quat::from_rotation_x(-rot_speed);
        }
        if input.is_key_active(KeyCode::Down) {
            transform.rotation = transform.rotation * Quat::from_rotation_x(rot_speed);
        }
    }
}

fn make_cube() -> Mesh {
    let positions: [[f32; 3]; 8] = [
        [-0.5, -0.5, -0.5],
        [0.5, -0.5, -0.5],
        [0.5, 0.5, -0.5],
        [-0.5, 0.5, -0.5],
        [-0.5, -0.5, 0.5],
        [0.5, -0.5, 0.5],
        [0.5, 0.5, 0.5],
        [-0.5, 0.5, 0.5],
    ];

    // (face normal, 4 vertex indices forming a quad)
    let faces: [([f32; 3], [usize; 4]); 6] = [
        ([0.0, 0.0, -1.0], [0, 3, 2, 1]), // back
        ([0.0, 0.0, 1.0], [4, 5, 6, 7]),  // front
        ([-1.0, 0.0, 0.0], [0, 4, 7, 3]), // left
        ([1.0, 0.0, 0.0], [1, 2, 6, 5]),  // right
        ([0.0, -1.0, 0.0], [0, 1, 5, 4]), // bottom
        ([0.0, 1.0, 0.0], [3, 7, 6, 2]),  // top
    ];

    let uvs: [[f32; 2]; 4] = [[0.0, 1.0], [1.0, 1.0], [1.0, 0.0], [0.0, 0.0]];

    let mut vertices = Vec::with_capacity(24);
    let mut indices = Vec::with_capacity(36);

    for (normal, corner_indices) in faces {
        let base = vertices.len() as u32;
        for (i, &vi) in corner_indices.iter().enumerate() {
            vertices.push(Vertex {
                pos_coords: positions[vi],
                uv_coords: uvs[i],
                normal,
                ..Vertex::default()
            });
        }
        // Two triangles per face
        indices.extend_from_slice(&[base, base + 1, base + 2, base, base + 2, base + 3]);
    }

    Mesh { vertices, indices }
}
