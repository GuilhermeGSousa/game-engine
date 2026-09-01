use app::{plugins::Plugin, schedule_groups::Update, App};

use crate::scene::Scene;
use crate::spawner::spawn_scene_components;

pub struct ScenePlugin;

impl Plugin for ScenePlugin {
    fn build(&self, app: &mut App) {
        // Mesh, Texture and StandardMaterial are already registered by
        // RenderPlugin / MaterialPlugin; re-registering here would swap their
        // populated AssetStore for an empty one and add a duplicate tracking
        // system, breaking in-flight loads.
        app.register_asset::<Scene>();
        app.add_system(Update, spawn_scene_components);
    }
}
