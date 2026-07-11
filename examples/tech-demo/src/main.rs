use game_engine::{
    DefaultPlugins,
    app::App,
    ecs::system::schedule::UpdateGroup::{self, Startup},
    gameplay::movement::first_person_player_fly,
};

use crate::character::spawn_character;

mod character;

fn main() {
    #[cfg(not(target_arch = "wasm32"))]
    std::env::set_current_dir(std::path::Path::new(env!("CARGO_MANIFEST_DIR")))
        .expect("Failed to set working directory");

    let mut app = App::new();
    app.register_plugin(DefaultPlugins::default());

    app.add_system(Startup, spawn_character)
        .add_system(UpdateGroup::Update, first_person_player_fly);

    app.run();
}
