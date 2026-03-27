use essential::assets::asset_server::{handle_asset_load_events, AssetServer};
use essential::time::Time;

use ecs::resource::ResMut;
use essential::transform::systems::{propagate_global_transforms, update_simple_entities};
use essential::transform::Transform;

use crate::update_group::UpdateGroup;
use crate::App;

/// Describes the current phase of plugin initialisation.
#[derive(PartialEq, Eq, Debug, Clone, Copy, PartialOrd, Ord)]
pub enum PluginsState {
    /// Plugins are still being built (waiting for async resources, etc.).
    Building,
    /// All plugins have reported `ready == true`; [`Plugin::finish`] can be called.
    Ready,
    /// [`Plugin::finish`] has been called on every plugin; the app is fully initialised.
    Finished,
}

/// Trait for modular pieces of engine functionality.
///
/// Implement `Plugin` to bundle related systems, resources, and configuration into a
/// reusable unit.  Register plugins with [`App::register_plugin`](crate::App::register_plugin).
///
/// # Lifecycle
/// 1. [`build`](Plugin::build) is called immediately on registration.
/// 2. [`ready`](Plugin::ready) is polled until all plugins return `true`.
/// 3. [`finish`](Plugin::finish) is called once to complete any deferred setup.
pub trait Plugin {
    /// Adds systems, resources, and other configuration to the app.
    fn build(&self, app: &mut App);

    /// Returns `true` once any async initialisation this plugin requires is complete.
    ///
    /// Defaults to `true` (synchronous plugins are always ready immediately).
    fn ready(&self, _app: &App) -> bool {
        true
    }

    /// Called after all plugins are ready; perform final, order-sensitive setup here.
    fn finish(&self, _app: &mut App) {}
}

/// Plugin that inserts a [`Time`] resource and an `update_time` system.
pub struct TimePlugin;

fn update_time(mut time: ResMut<Time>) {
    time.update();
}

impl Plugin for TimePlugin {
    fn build(&self, app: &mut App) {
        app.insert_resource(Time::new());
        app.add_system(UpdateGroup::Update, update_time);
    }
}

/// Plugin that inserts an [`AssetServer`] resource and the asset-event handler.
pub struct AssetManagerPlugin;

impl Plugin for AssetManagerPlugin {
    fn build(&self, app: &mut App) {
        app.insert_resource(AssetServer::new());
        app.add_system(UpdateGroup::LateUpdate, handle_asset_load_events);
    }
}

/// Plugin that registers [`Transform`] lifecycle callbacks and the global-transform
/// propagation systems.
pub struct TransformPlugin;

impl Plugin for TransformPlugin {
    fn build(&self, app: &mut App) {
        app.register_component_lifecycle::<Transform>();
        app.add_system(UpdateGroup::LateUpdate, update_simple_entities)
            .add_system(UpdateGroup::LateUpdate, propagate_global_transforms);
    }
}
