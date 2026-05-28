use app::{
    plugins::{AssetManagerPlugin, TimePlugin, TransformPlugin},
    App,
};
use ecs::{command::CommandQueue, query::Query, resource::Res, Component};
use essential::{assets::asset_server::AssetServer, time::Time, transform::Transform};
use glam::{Quat, Vec3, Vec4};
use render::{
    assets::{
        material::StandardMaterial,
        mesh::Mesh,
        texture::Texture,
        vertex::Vertex,
    },
    components::{
        camera::{Camera, RenderTarget},
        light::{LighType, Light},
        material_component::MaterialComponent,
        mesh_component::MeshComponent,
    },
    plugin::RenderPlugin,
};
use terminal_renderer::{readback::TerminalRenderState, TerminalOutput, TerminalRendererPlugin};
use ecs::system::schedule::UpdateGroup;

#[derive(Component)]
struct Cube;

fn main() {
    let mut app = App::new();
    app.register_plugin(AssetManagerPlugin)
        .register_plugin(TimePlugin)
        .register_plugin(RenderPlugin)
        .register_plugin(TransformPlugin)
        .register_plugin(TerminalRendererPlugin)
        .add_system(UpdateGroup::Startup, spawn_scene)
        .add_system(UpdateGroup::Update, rotate_cube);
    app.run();
}

fn spawn_scene(
    mut cmd: CommandQueue,
    asset_server: Res<AssetServer>,
    state: Res<TerminalRenderState>,
) {
    let rtt = asset_server.add(Texture::render_target(state.width, state.height));

    // Account for terminal cells being ~2x taller than wide
    let aspect = (state.width as f32 * 0.5) / state.height as f32;
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

    let mesh = asset_server.add(make_cube());
    let material = asset_server.add(StandardMaterial::new(None, None));
    cmd.spawn((
        Cube,
        MeshComponent { handle: mesh },
        MaterialComponent::<StandardMaterial> { handle: material },
        Transform::from_translation_rotation(Vec3::ZERO, Quat::IDENTITY),
    ));

    cmd.spawn((
        Light {
            color: Vec4::ONE,
            intensity: 8.0,
            light_type: LighType::Point,
        },
        Transform::from_translation_rotation(Vec3::new(3.0, 3.0, 3.0), Quat::IDENTITY),
    ));
}

fn rotate_cube(cubes: Query<&mut Transform, ecs::With<Cube>>, time: Res<Time>) {
    let delta = time.delta().as_secs_f32();
    for mut transform in cubes.iter() {
        transform.rotation = transform.rotation * Quat::from_rotation_y(delta);
    }
}

fn make_cube() -> Mesh {
    let positions: [[f32; 3]; 8] = [
        [-0.5, -0.5, -0.5],
        [ 0.5, -0.5, -0.5],
        [ 0.5,  0.5, -0.5],
        [-0.5,  0.5, -0.5],
        [-0.5, -0.5,  0.5],
        [ 0.5, -0.5,  0.5],
        [ 0.5,  0.5,  0.5],
        [-0.5,  0.5,  0.5],
    ];

    // (face normal, 4 vertex indices forming a quad)
    let faces: [([f32; 3], [usize; 4]); 6] = [
        ([ 0.0,  0.0, -1.0], [0, 3, 2, 1]), // back
        ([ 0.0,  0.0,  1.0], [4, 5, 6, 7]), // front
        ([-1.0,  0.0,  0.0], [0, 4, 7, 3]), // left
        ([ 1.0,  0.0,  0.0], [1, 2, 6, 5]), // right
        ([ 0.0, -1.0,  0.0], [0, 1, 5, 4]), // bottom
        ([ 0.0,  1.0,  0.0], [3, 7, 6, 2]), // top
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
