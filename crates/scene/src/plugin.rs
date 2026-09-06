use app::{plugins::Plugin, schedule_groups::Update, App};

use essential::transform::Transform;
use mesh::{mesh::MeshComponent, SkeletonComponent};
use render::components::camera::Camera;
use render::components::light::Light;
use render::components::material::MaterialComponent;
use render::components::render_entity::SyncWithRenderWorld;

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

        app.register_scene_component::<Transform>();
        app.register_scene_component::<MeshComponent>();
        app.register_scene_component::<MaterialComponent>();
        app.register_scene_component::<Camera>();
        app.register_scene_component::<Light>();
        app.register_scene_component::<SyncWithRenderWorld>();
        app.register_scene_component::<SkeletonComponent>();

        app.add_system(Update, spawn_scene_components);
    }
}
