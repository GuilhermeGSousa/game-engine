use std::f32::consts::PI;

use app::App;
use color::LinearRgba;
use debug_gizmos::components::GizmoSphere;
use ecs::{
    command::CommandQueue,
    query::Query,
    resource::Res,
    system::schedule::UpdateGroup,
    Component, With,
};
use essential::{assets::asset_server::AssetServer, time::Time, transform::Transform};
use game_engine::{gltf_loader::loader::GLTFSpawnerComponent, DefaultPlugins};
use glam::{Quat, Vec3, Vec4};
use render::{
    assets::{material::StandardMaterial, mesh::Mesh, vertex::Vertex},
    components::{
        light::{LighType, Light, SpotLight},
        material_component::MaterialComponent,
        mesh_component::MeshComponent,
    },
};

#[cfg(feature = "terminal")]
use ecs::{resource::ResMut, IntoSystemConfig};
#[cfg(feature = "terminal")]
use render::{
    assets::texture::Texture,
    components::camera::{Camera, RenderTarget},
    wgpu,
};
#[cfg(feature = "terminal")]
use ratatui::{
    crossterm::event::KeyCode,
    layout::{Constraint, Layout},
    style::Stylize,
    text::{Line as TextLine, Span},
};
#[cfg(feature = "terminal")]
use terminal_renderer::{
    frame::TerminalFrame, print_terminal_frame, terminal::TerminalContext, TerminalInput,
    TerminalOutput, TerminalRendererPlugin,
};

#[cfg(not(feature = "terminal"))]
use debug_gizmos::plugin::DebugGizmosPlugin;
#[cfg(not(feature = "terminal"))]
use gameplay::{movement::first_person_player_fly, player::spawn_first_person_player};

const SPONZA_PATH: &str = "res/Sponza/Sponza.gltf";

#[derive(Component)]
struct Cube;

fn main() {
    cfg_if::cfg_if! {
        if #[cfg(target_arch = "wasm32")] {
            std::panic::set_hook(Box::new(console_error_panic_hook::hook));
            console_log::init_with_level(log::Level::Debug).expect("Couldn't initialize logger");
        } else {
            env_logger::init();
        }
    }

    #[cfg(not(target_arch = "wasm32"))]
    std::env::set_current_dir(std::path::Path::new(env!("CARGO_MANIFEST_DIR")))
        .expect("Failed to set working directory");

    let mut app = App::new();

    #[cfg(feature = "terminal")]
    {
        app.register_plugin(DefaultPlugins::headless())
            .register_plugin(TerminalRendererPlugin)
            .add_system(UpdateGroup::Startup, spawn_camera_terminal)
            .add_system(UpdateGroup::Startup, spawn_scene)
            .add_system(UpdateGroup::Update, rotate_cube)
            .add_system(UpdateGroup::Update, move_camera);
        app.add_system(
            UpdateGroup::LateRender,
            draw_terminal.after(print_terminal_frame),
        );
    }

    #[cfg(not(feature = "terminal"))]
    {
        app.register_plugin(DefaultPlugins::default())
            .add_system(UpdateGroup::Startup, spawn_camera_windowed)
            .add_system(UpdateGroup::Startup, spawn_scene)
            .add_system(UpdateGroup::Update, rotate_cube)
            .add_system(UpdateGroup::Update, first_person_player_fly);
        app.register_plugin(DebugGizmosPlugin);
    }

    app.run();
}

#[cfg(feature = "terminal")]
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
        clear_color: wgpu::Color::BLACK,
        ..Camera::default()
    };
    cmd.spawn((
        camera,
        Light {
            color: Vec4::new(1.0, 1.0, 1.0, 1.0),
            intensity: 1.0,
            light_type: LighType::Point,
        },
        TerminalOutput,
        Transform::from_translation_rotation(Vec3::new(0.0, 0.0, 5.0), Quat::IDENTITY),
    ));
}

#[cfg(not(feature = "terminal"))]
fn spawn_camera_windowed(mut cmd: CommandQueue) {
    spawn_first_person_player(&mut cmd, Vec3::new(0.0, 2.0, 0.0));
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
        Transform::from_translation_rotation(Vec3::new(0.0, 2.0, 0.0), Quat::IDENTITY);
    light_transform.look_at(Vec3::ZERO, Vec3::Y);

    cmd.spawn(GizmoSphere {
        center: light_transform.translation,
        radius: 0.1,
        color: LinearRgba::GREEN,
    });
    cmd.spawn((light, light_transform));

    cmd.spawn(GLTFSpawnerComponent(asset_server.load(SPONZA_PATH)));
}

fn rotate_cube(cubes: Query<&mut Transform, With<Cube>>, time: Res<Time>) {
    let delta = time.delta().as_secs_f32();
    for mut transform in cubes.iter() {
        transform.rotation = transform.rotation * Quat::from_rotation_y(delta);
    }
}

#[cfg(feature = "terminal")]
fn move_camera(
    cameras: Query<&mut Transform, With<Camera>>,
    input: Res<TerminalInput>,
    time: Res<Time>,
) {
    let speed = 5.0 * time.delta().as_secs_f32();
    let rot_speed = 2.0 * time.delta().as_secs_f32();

    for mut transform in cameras.iter() {
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

#[cfg(feature = "terminal")]
fn draw_terminal(mut terminal: ResMut<TerminalContext>, terminal_frame: Res<TerminalFrame>) {
    terminal
        .draw(|frame| {
            if let Some(data) = terminal_frame.current_frame() {
                let vertical =
                    Layout::vertical([Constraint::Length(1), Constraint::Fill(1)]).spacing(1);
                let horizontal = Layout::horizontal([Constraint::Percentage(100)]).spacing(1);
                let [top, main] = frame.area().layout(&vertical);
                let [area] = main.layout(&horizontal);

                let title = TextLine::from_iter([
                    Span::from("This is a Widget!").bold(),
                    Span::from(" (Press 'ESC' to quit)"),
                ]);

                frame.render_widget(title.centered(), top);
                frame.render_widget(data, area);
            }
        })
        .unwrap();
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

    let faces: [([f32; 3], [usize; 4]); 6] = [
        ([0.0, 0.0, -1.0], [0, 3, 2, 1]),
        ([0.0, 0.0, 1.0], [4, 5, 6, 7]),
        ([-1.0, 0.0, 0.0], [0, 4, 7, 3]),
        ([1.0, 0.0, 0.0], [1, 2, 6, 5]),
        ([0.0, -1.0, 0.0], [0, 1, 5, 4]),
        ([0.0, 1.0, 0.0], [3, 7, 6, 2]),
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
        indices.extend_from_slice(&[base, base + 1, base + 2, base, base + 2, base + 3]);
    }

    let mut mesh = Mesh { vertices, indices };
    mesh.compute_tangents();
    mesh
}
