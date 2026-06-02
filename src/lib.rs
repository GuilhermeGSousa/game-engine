pub use animation;
pub use app;
pub use ecs;
pub use essential;
pub use gltf_loader;
pub use obj_loader;
pub use physics;
pub use render;
pub use skybox;
pub use ui;
pub use window;

use animation::plugin::AnimationPlugin;
use app::{
    plugins::{AssetManagerPlugin, TimePlugin, TransformPlugin},
    App, Plugin,
};
use gltf_loader::plugin::GLTFPlugin;
use obj_loader::plugin::OBJPlugin;
use physics::plugin::PhysicsPlugin;
use render::{assets::material::StandardMaterial, plugin::RenderPlugin, MaterialPlugin};
use skybox::plugin::SkyboxPlugin;
use ui::plugin::UIPlugin;
use window::plugin::WindowPlugin;

/// Registers all standard engine plugins in the conventional order.
pub struct DefaultPlugins;

impl Plugin for DefaultPlugins {
    fn build(&self, app: &mut App) {
        app.register_plugin(AssetManagerPlugin)
            .register_plugin(TimePlugin)
            .register_plugin(WindowPlugin)
            .register_plugin(RenderPlugin)
            .register_plugin(MaterialPlugin::<StandardMaterial>::new())
            .register_plugin(TransformPlugin)
            .register_plugin(PhysicsPlugin)
            .register_plugin(AnimationPlugin)
            .register_plugin(GLTFPlugin)
            .register_plugin(OBJPlugin)
            .register_plugin(SkyboxPlugin)
            .register_plugin(UIPlugin);
    }
}
