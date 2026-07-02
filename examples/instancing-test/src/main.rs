use color::LinearRgba;
use ecs::{
    command::CommandQueue,
    resource::{Res, ResMut, Resource},
    system::schedule::UpdateGroup,
};
use essential::{assets::asset_server::AssetServer, transform::Transform};
use game_engine::DefaultPlugins;
use gameplay::{movement::first_person_player_fly, player::spawn_first_person_player};
use glam::Vec3;
use mesh::{Mesh, MeshComponent, Vertex};
use render::{
    assets::material::StandardMaterial,
    components::light::{Light, LightType},
    resources::DrawCallStats,
    MaterialComponent,
};

// A GRID_SIZE^3 lattice of cubes, all sharing one mesh asset and one material
// asset, spaced SPACING apart. Every cube is a separate entity, but since they
// share (mesh_asset_id, material_asset_id), instancing should collapse them into
// a single `draw_indexed` call per camera instead of one per cube — see
// `report_draw_calls` below.
const GRID_SIZE: i32 = 10;
const SPACING: f32 = 2.5;

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

    let mut app = app::App::new();
    app.register_plugin(DefaultPlugins::default());
    app.insert_resource(DrawCallLog::default());

    app.add_system(UpdateGroup::Startup, spawn_camera)
        .add_system(UpdateGroup::Startup, spawn_cubes)
        .add_system(UpdateGroup::Update, first_person_player_fly)
        .add_system(UpdateGroup::Update, report_draw_calls);

    app.run();
}

fn spawn_camera(mut cmd: CommandQueue) {
    let grid_extent = (GRID_SIZE as f32 - 1.0) * SPACING * 0.5;
    spawn_first_person_player(
        &mut cmd,
        Vec3::new(0.0, grid_extent + 5.0, grid_extent + 30.0),
        Light {
            color: LinearRgba::new(1.0, 1.0, 1.0, 1.0),
            intensity: 800.0,
            light_type: LightType::Point,
        },
    );
}

fn spawn_cubes(mut cmd: CommandQueue, asset_server: Res<AssetServer>) {
    let mesh_handle = asset_server.add(cube_mesh());
    let material_handle = asset_server.add(
        StandardMaterial::default().with_base_color_factor(LinearRgba::new(0.2, 0.6, 1.0, 1.0)),
    );

    let offset = (GRID_SIZE as f32 - 1.0) * SPACING * 0.5;

    for x in 0..GRID_SIZE {
        for y in 0..GRID_SIZE {
            for z in 0..GRID_SIZE {
                let translation = Vec3::new(
                    x as f32 * SPACING - offset,
                    y as f32 * SPACING - offset,
                    z as f32 * SPACING - offset,
                );
                cmd.spawn((
                    MeshComponent {
                        handle: mesh_handle.clone(),
                    },
                    MaterialComponent {
                        handle: material_handle.clone(),
                    },
                    Transform::from_translation(translation),
                ));
            }
        }
    }
}

#[derive(Resource, Default)]
struct DrawCallLog {
    frames_since_log: u32,
}

// Logs the engine's draw-call counter roughly once a second so the instancing
// win is visible without needing RUST_LOG=debug.
fn report_draw_calls(mut log_state: ResMut<DrawCallLog>, stats: Res<DrawCallStats>) {
    log_state.frames_since_log += 1;
    if log_state.frames_since_log >= 60 {
        log_state.frames_since_log = 0;
        log::info!(
            "draw calls last frame: {} ({}^3 = {} cubes spawned)",
            stats.draw_calls,
            GRID_SIZE,
            GRID_SIZE * GRID_SIZE * GRID_SIZE,
        );
    }
}

// A unit cube (24 vertices, one per face-corner so each face gets its own
// normal/tangent/bitangent) centered on the origin.
fn cube_mesh() -> Mesh {
    // (normal, tangent, bitangent) per face; tangent x bitangent == normal so the
    // winding order below is consistently outward-facing (CCW as seen from outside).
    let faces: [(Vec3, Vec3, Vec3); 6] = [
        (Vec3::X, -Vec3::Z, Vec3::Y),
        (-Vec3::X, Vec3::Z, Vec3::Y),
        (Vec3::Y, Vec3::X, -Vec3::Z),
        (-Vec3::Y, Vec3::X, Vec3::Z),
        (Vec3::Z, Vec3::X, Vec3::Y),
        (-Vec3::Z, -Vec3::X, Vec3::Y),
    ];

    let mut vertices = Vec::with_capacity(24);
    let mut indices = Vec::with_capacity(36);

    for (normal, tangent, bitangent) in faces {
        let base = vertices.len() as u32;
        let corners = [(-1.0, -1.0), (1.0, -1.0), (1.0, 1.0), (-1.0, 1.0)];
        let uvs = [[0.0, 1.0], [1.0, 1.0], [1.0, 0.0], [0.0, 0.0]];

        for ((su, sv), uv) in corners.into_iter().zip(uvs) {
            let position = normal * 0.5 + tangent * (su * 0.5) + bitangent * (sv * 0.5);
            vertices.push(Vertex {
                pos_coords: position.into(),
                uv_coords: uv,
                normal: normal.into(),
                tangent: tangent.into(),
                bitangent: bitangent.into(),
                bone_indices: [0; Vertex::MAX_AFFECTED_BONES],
                bone_weights: [0.0; Vertex::MAX_AFFECTED_BONES],
            });
        }

        indices.extend_from_slice(&[base, base + 1, base + 2, base, base + 2, base + 3]);
    }

    Mesh { vertices, indices }
}
