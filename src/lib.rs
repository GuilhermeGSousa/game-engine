pub use animation;
pub use app;
pub use color;
pub use ecs;
pub use essential;
pub use gameplay;
use gameplay::GameplayPlugin;
pub use gltf_loader;
#[cfg(not(target_arch = "wasm32"))]
pub use physics;
pub use mesh;
pub use obj_loader;
pub use render;
pub use skybox;
pub use ui;
pub use window;
pub use world_grid;

use animation::plugin::AnimationPlugin;
use app::{
    plugins::{AssetManagerPlugin, TimePlugin, TransformPlugin},
    App, Plugin,
};
use gltf_loader::plugin::GLTFPlugin;
#[cfg(not(target_arch = "wasm32"))]
use physics::plugin::PhysicsPlugin;
use obj_loader::plugin::OBJPlugin;
use render::{assets::material::StandardMaterial, plugin::RenderPlugin, MaterialPlugin};
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
        app.register_plugin(AssetManagerPlugin)
            .register_plugin(TimePlugin);

        if !self.headless {
            app.register_plugin(WindowPlugin);
        }
        app.register_plugin(RenderPlugin)
            .register_plugin(SkyboxPlugin)
            .register_plugin(MaterialPlugin::<StandardMaterial>::new())
            .register_plugin(TransformPlugin);

        // Physics is not supported on the web (Jolt's C++ requires thread
        // primitives that wasm32 toolchains do not provide).
        #[cfg(not(target_arch = "wasm32"))]
        app.register_plugin(PhysicsPlugin);

        app.register_plugin(AnimationPlugin)
            .register_plugin(GLTFPlugin)
            .register_plugin(OBJPlugin)
            .register_plugin(WorldGridPlugin)
            .register_plugin(GameplayPlugin);

        if !self.headless {
            app.register_plugin(UIPlugin);
        }
    }
}
