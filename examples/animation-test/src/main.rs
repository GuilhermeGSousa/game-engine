use color::LinearRgba;
use game_engine::{
    app::App,
    ecs::{command::CommandQueue, system::schedule::UpdateGroup},
    render::components::light::{Light, LightType},
    DefaultPlugins,
};
use gameplay::{movement::first_person_player_fly, player::spawn_first_person_player};
use glam::Vec3;

use debug_gizmos::plugin::DebugGizmosPlugin;

use crate::demo_overlay::{spawn_entity_gizmos, spawn_overlay, update_overlay};
use crate::movement_animation::{
    setup_animations, setup_state_machine, spawn_character, update_movement,
};

mod demo_overlay;
mod movement_animation;

fn main() {
    #[cfg(not(target_arch = "wasm32"))]
    std::env::set_current_dir(std::path::Path::new(env!("CARGO_MANIFEST_DIR")))
        .expect("Failed to set working directory");

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

    // Startup: camera + light, the animated character, and the debug overlay.
    app.add_system(UpdateGroup::Startup, spawn_camera)
        .add_system(UpdateGroup::Startup, spawn_character)
        .add_system(UpdateGroup::Startup, spawn_overlay);

    // Update: fly camera, the animation state-machine setup chain, FSM input, and the
    // debug overlay / gizmos.
    app.add_system(UpdateGroup::Update, first_person_player_fly)
        .add_system(UpdateGroup::Update, setup_state_machine)
        .add_system(UpdateGroup::Update, setup_animations)
        .add_system(UpdateGroup::Update, update_movement)
        .add_system(UpdateGroup::Update, update_overlay)
        .add_system(UpdateGroup::Update, spawn_entity_gizmos);

    app.run();
}

fn spawn_camera(mut cmd: CommandQueue) {
    // First-person fly camera with a headlight so the character is lit wherever you look.
    spawn_first_person_player(
        &mut cmd,
        Vec3::new(0.0, 1.0, 0.0),
        Light {
            color: LinearRgba::new(1.0, 1.0, 1.0, 1.0),
            intensity: 20.0,
            light_type: LightType::Point,
        },
    );
}
