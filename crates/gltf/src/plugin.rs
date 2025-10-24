use app::plugins::Plugin;

use crate::loader::GLTFScene;
use crate::loader::spawn_gltf_components;

pub struct GLTFPlugin;

impl Plugin for GLTFPlugin {
    fn build(&self, app: &mut app::App) {
        app.register_asset::<GLTFScene>();
        app.add_system(
            app::update_group::UpdateGroup::Update,
            spawn_gltf_components,
        );
    }
}
