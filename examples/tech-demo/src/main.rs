#[cfg(not(target_arch = "wasm32"))]
use game_engine::essential::assets::{ContentAssetRoot, asset_server::AssetServer};
use game_engine::{
    DefaultPlugins,
    app::{
        App,
        schedule_groups::{Startup, Update},
    },
    ui::frame_stats_overlay::FrameStatsOverlayPlugin,
};

use crate::{
    character::{
        PlayerSpawner, face_camera_direction, setup_character_animations, spawn_character,
        update_movement,
    },
    scene::spawn_scene,
    ui::{spawn_grounded_overlay, update_grounded_overlay},
};

mod character;
mod scene;
mod ui;

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
    app.register_plugin(DefaultPlugins::default())
        .register_plugin(FrameStatsOverlayPlugin);

    set_own_content_root(&mut app);
    app.register_component::<PlayerSpawner>();

    app.add_system(Update, spawn_character)
        .add_system(Startup, spawn_scene)
        .add_system(Startup, spawn_grounded_overlay)
        .add_system(Update, setup_character_animations)
        .add_system(Update, update_movement)
        .add_system(Update, face_camera_direction)
        .add_system(Update, update_grounded_overlay);

    app.run();
}

/// Points this example's `AssetServer` at its own
/// `<exe-dir>/tech-demo-content/content/` (populated by `build.rs`) rather
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
