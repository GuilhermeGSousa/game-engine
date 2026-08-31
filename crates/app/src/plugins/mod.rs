use essential::assets::asset_server::{handle_asset_load_events, AssetServer};
use essential::assets::handle::AssetLifetimeEvent;
use essential::time::{FrameStats, Time};

use ecs::resource::{Res, ResMut};
use essential::transform::systems::{propagate_global_transforms, update_simple_entities};
use essential::transform::Transform;

use crate::schedule_groups::{LateUpdate, Update};
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

    fn name(&self) -> &'static str {
        std::any::type_name::<Self>()
    }
}

/// Plugin that inserts a [`Time`] resource and an `update_time` system,
/// plus a [`FrameStats`] rolling window of frame times.
pub struct TimePlugin;

fn update_time(mut time: ResMut<Time>) {
    time.update();
}

fn update_frame_stats(time: Res<Time>, mut stats: ResMut<FrameStats>) {
    let delta = time.delta();
    stats.push(delta);

    // Opt-in visibility without any UI: RUST_LOG=info prints a summary once
    // per second.
    if stats.tick_summary(delta) {
        log::info!(
            "frame: {:.2} ms avg / {:.2} ms p99 / {:.2} ms max ({:.0} FPS)",
            stats.average_ms(),
            stats.percentile_ms(0.99),
            stats.max_ms(),
            stats.fps(),
        );
    }
}

impl Plugin for TimePlugin {
    fn build(&self, app: &mut App) {
        app.insert_resource(Time::new());
        app.insert_resource(FrameStats::new());
        app.add_system(Update, update_time);
        app.add_system(LateUpdate, update_frame_stats);
    }
}

/// Plugin that inserts an [`AssetServer`] resource and the asset-event handler.
pub struct AssetManagerPlugin;

impl Plugin for AssetManagerPlugin {
    fn build(&self, app: &mut App) {
        app.insert_resource(AssetServer::new());
        app.register_event::<AssetLifetimeEvent>();
        app.add_system(LateUpdate, handle_asset_load_events);
    }
}

/// Plugin that registers [`Transform`] lifecycle callbacks and the global-transform
/// propagation systems.
pub struct TransformPlugin;

impl Plugin for TransformPlugin {
    fn build(&self, app: &mut App) {
        app.register_component_lifetimes::<Transform>();
        app.add_system(LateUpdate, update_simple_entities)
            .add_system(LateUpdate, propagate_global_transforms);
    }
}
