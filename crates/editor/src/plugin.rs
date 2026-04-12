use app::plugins::Plugin;
use essential::assets::asset_server::AssetServer;
use render::assets::texture::Texture;

use crate::layout::{EditorRttHandle, spawn_editor_ui};

pub struct EditorPlugin;

impl Plugin for EditorPlugin {
    fn build(&self, app: &mut app::App) {
        app.add_system(app::update_group::UpdateGroup::Startup, spawn_editor_ui);
    }

    fn finish(&self, app: &mut app::App) {
        let handle = app
            .get_resource::<AssetServer>()
            .expect("AssetServer not found — register AssetManagerPlugin before EditorPlugin")
            .add(Texture::render_target(1280, 720));
        app.insert_resource(EditorRttHandle(handle));
    }
}
