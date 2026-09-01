pub use animation;
pub use app;
pub use color;
pub use director;
pub use ecs;
pub use essential;
pub use gameplay;
use gameplay::GameplayPlugin;
pub use mesh;
pub use physics;
pub use render;
pub use scene;
pub use skybox;
pub use ui;
pub use window;
pub use world_grid;

use animation::plugin::AnimationPlugin;
use app::{
    main_schedule::MainSchedulePlugin,
    plugins::{AssetManagerPlugin, TimePlugin, TransformPlugin},
    App, Plugin,
};
use director::CameraDirectorPlugin;
use physics::plugin::PhysicsPlugin;
use render::{
    assets::material::StandardMaterial, plugin::RenderPlugin,
    shadow_pipeline::ShadowPipelinePlugin, MaterialPlugin,
};
use scene::plugin::ScenePlugin;
use skybox::plugin::SkyboxPlugin;
use ui::plugin::UIPlugin;
use window::plugin::WindowPlugin;
use world_grid::plugin::WorldGridPlugin;

/// Registers all standard engine plugins in the conventional order.
#[derive(Default)]
pub struct DefaultPlugins {
    headless: bool,
}

impl DefaultPlugins {
    pub fn headless() -> Self {
        Self { headless: true }
    }
}

impl Plugin for DefaultPlugins {
    fn build(&self, app: &mut App) {
        app.register_plugin(MainSchedulePlugin)
            .register_plugin(AssetManagerPlugin)
            .register_plugin(TimePlugin);

        if !self.headless {
            app.register_plugin(WindowPlugin);
        }
        // CameraPlugin goes before RenderPlugin so it demotes stray window
        // cameras before `camera_added` gives them render resources.
        app.register_plugin(TransformPlugin)
            .register_plugin(CameraDirectorPlugin)
            .register_plugin(RenderPlugin)
            .register_plugin(SkyboxPlugin)
            .register_plugin(ShadowPipelinePlugin)
            .register_plugin(MaterialPlugin::<StandardMaterial>::default());

        app.register_plugin(PhysicsPlugin)
            .register_plugin(AnimationPlugin)
            .register_plugin(ScenePlugin)
            .register_plugin(WorldGridPlugin)
            .register_plugin(GameplayPlugin);

        if !self.headless {
            app.register_plugin(UIPlugin);
        }
    }
}
