use game_engine::{
    DefaultPlugins,
    app::App,
    ecs::system::schedule::UpdateGroup::{self, Startup},
    gameplay::movement::first_person_player_fly,
};

use crate::{
    character::{setup_character_animations, spawn_character, update_movement},
    scene::spawn_scene,
};

mod character;
mod scene;

fn main() {
    #[cfg(not(target_arch = "wasm32"))]
    std::env::set_current_dir(std::path::Path::new(env!("CARGO_MANIFEST_DIR")))
        .expect("Failed to set working directory");

    let mut app = App::new();
    app.register_plugin(DefaultPlugins::default());

    app.add_system(Startup, spawn_character)
        .add_system(Startup, spawn_scene)
        .add_system(UpdateGroup::Update, setup_character_animations)
        .add_system(UpdateGroup::Update, first_person_player_fly)
        .add_system(UpdateGroup::Update, update_movement);

    app.run();
}
