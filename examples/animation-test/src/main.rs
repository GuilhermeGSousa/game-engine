use color::Color;
#[cfg(not(target_arch = "wasm32"))]
use game_engine::essential::assets::{asset_server::AssetServer, ContentAssetRoot};
use game_engine::{
    app::{
        schedule_groups::{Startup, Update},
        App,
    },
    ecs::command::CommandQueue,
    render::components::light::{Light, LightType},
    DefaultPlugins,
};
use gameplay::{movement::first_person_player_fly, player::spawn_first_person_player};
use glam::Vec3;

use debug_gizmos::DebugGizmosPlugin;
use world_grid::WorldGrid;

use crate::demo_overlay::{draw_entity_gizmos, spawn_overlay, update_overlay};
use crate::movement_animation::{setup_animations, spawn_character, update_movement};

mod demo_overlay;
mod movement_animation;

fn main() {
    cfg_if::cfg_if! {
        if #[cfg(target_arch = "wasm32")] {
            std::panic::set_hook(Box::new(console_error_panic_hook::hook));
            console_log::init_with_level(log::Level::Debug).expect("Couldn't initialize logger");
        } else {
            env_logger::init();
        }
    }

    let mut app = App::new();
    app.register_plugin(DefaultPlugins::default());
    app.register_plugin(DebugGizmosPlugin);
    set_own_content_root(&mut app);

    // Startup: camera + light, the animated character, and the debug overlay.
    app.add_system(Startup, spawn_camera)
        .add_system(Startup, spawn_character)
        .add_system(Startup, spawn_overlay);

    // Update: fly camera, blend-space graph setup, movement input, and the
    // debug overlay / gizmos.
    app.add_system(Update, first_person_player_fly)
        .add_system(Update, setup_animations)
        .add_system(Update, update_movement)
        .add_system(Update, update_overlay)
        .add_system(Update, draw_entity_gizmos);

    app.run();
}

/// Points this example's `AssetServer` at its own
/// `<exe-dir>/animation-test-content/content/` (populated by `build.rs`) rather
/// than the bare exe-dir default, which every example in this workspace shares
/// (they all build into one Cargo `target/` directory). Must run before any
/// system calls `.load()` — here, in `main()` before `app.run()`, is early
/// enough. wasm serves each example from its own Trunk-built origin already,
/// so there is nothing to disambiguate there.
#[cfg(not(target_arch = "wasm32"))]
fn set_own_content_root(app: &mut App) {
    let Some(asset_server) = app.get_resource::<AssetServer>() else {
        return;
    };
    let exe_dir = std::env::current_exe()
        .ok()
        .and_then(|exe| exe.parent().map(|p| p.to_path_buf()))
        .unwrap_or_else(|| std::path::PathBuf::from("."));
    asset_server.set_content_root(ContentAssetRoot::Directory(
        exe_dir.join(format!("{}-content", env!("CARGO_PKG_NAME"))),
    ));
}

#[cfg(target_arch = "wasm32")]
fn set_own_content_root(_app: &mut App) {}

fn spawn_camera(mut cmd: CommandQueue) {
    cmd.spawn(WorldGrid::default());
    // First-person fly camera with a headlight so the character is lit wherever you look.
    spawn_first_person_player(
        &mut cmd,
        Vec3::new(0.0, 1.0, 0.0),
        Light {
            color: Color::rgba(1.0, 1.0, 1.0, 1.0),
            intensity: 20.0,
            light_type: LightType::Point,
            shadowmaps_enabled: false,
        },
    );
}
