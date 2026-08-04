use ecs::{
    component::Component,
    events::{
        event_channel::{update_event_channel, EventChannel},
        event_writer::EventWriter,
        Event,
    },
    resource::{ResMut, Resource},
    system::schedule::UpdateGroup,
    world::World,
    IntoSystemConfig,
};
use facet::Facet;
use log::info;
use runner::AppExit;

use essential::{
    assets::{
        asset_server::AssetServer, asset_store::AssetStore, handle::AssetLifetimeEvent, Asset,
    },
    time::Time,
};

use crate::{plugins::PluginsState, runner::run_once, sub_app::SubApps};

pub mod plugins;
pub mod runner;
pub mod sub_app;

// Re-export the most commonly needed types so users don't have to know the module layout.
pub use plugins::Plugin;
pub use sub_app::{ExtractFn, SubApp, SubAppLabel};

pub(crate) struct HokeyPokeyPlugin;
impl Plugin for HokeyPokeyPlugin {
    fn build(&self, _: &mut App) {}
}

/// The top-level container for the game engine.
///
/// An `App` owns a set of [`SubApp`]s and a list of [`Plugin`]s.  Call
/// [`run`](App::run) to hand control over to the configured runner (typically
/// the window event loop).
///
/// Every `App` has a main sub-app — the game world — and may hold further
/// sub-apps, each with its own [`World`] and schedules.  The `App`'s own
/// world-facing methods (`insert_resource`, `add_system`, …) all target the
/// main sub-app; reach the others through [`sub_app_mut`](App::sub_app_mut).
///
/// # Typical setup
/// ```ignore
/// use app::App;
/// use app::plugins::TimePlugin;
///
/// let mut app = App::empty();
/// app.register_plugin(TimePlugin);
/// app.run();
/// ```
pub struct App {
    runner: runner::RunnerFn,
    sub_apps: SubApps,
    accumulated_fixed_time: f32,
    plugins: Vec<Box<dyn Plugin>>,
    plugin_state: PluginsState,
}

impl App {
    pub fn new() -> App {
        Self {
            runner: Box::new(runner::run_once),
            sub_apps: SubApps::new(),
            accumulated_fixed_time: 0.0,
            plugins: Vec::new(),
            plugin_state: PluginsState::Building,
        }
    }

    /// The main (game) sub-app.
    pub fn main(&self) -> &SubApp {
        &self.sub_apps.main
    }

    /// The main (game) sub-app.
    pub fn main_mut(&mut self) -> &mut SubApp {
        &mut self.sub_apps.main
    }

    /// The main sub-app's world.
    pub fn world(&self) -> &World {
        self.sub_apps.main.world()
    }

    /// The main sub-app's world.
    pub fn world_mut(&mut self) -> &mut World {
        self.sub_apps.main.world_mut()
    }

    /// Adds a sub-app under `label`, replacing any existing one.
    ///
    /// Sub-apps run sequentially after the main sub-app, in insertion order.
    pub fn insert_sub_app(&mut self, label: SubAppLabel, sub_app: SubApp) -> &mut Self {
        info!("Registering sub-app: {}", label);
        self.sub_apps.insert(label, sub_app);
        self
    }

    pub fn get_sub_app(&self, label: SubAppLabel) -> Option<&SubApp> {
        self.sub_apps.get(label)
    }

    pub fn get_sub_app_mut(&mut self, label: SubAppLabel) -> Option<&mut SubApp> {
        self.sub_apps.get_mut(label)
    }

    /// Returns the sub-app registered under `label`.
    ///
    /// # Panics
    /// Panics if no such sub-app has been inserted — usually a plugin ordering
    /// problem, where the plugin that owns the sub-app has not been registered
    /// yet.
    pub fn sub_app_mut(&mut self, label: SubAppLabel) -> &mut SubApp {
        self.sub_apps
            .get_mut(label)
            .unwrap_or_else(|| panic!("Sub-app {label} not found"))
    }

    /// Builds and registers a [`Plugin`].
    ///
    /// Calls [`Plugin::build`] immediately, then stores the plugin so that
    /// [`Plugin::ready`] and [`Plugin::finish`] can be polled later.
    pub fn register_plugin(&mut self, plugin: impl Plugin + 'static) -> &mut Self {
        info!("Registering plugin: {}", plugin.name());
        plugin.build(self);
        self.plugins.push(Box::new(plugin));
        self
    }

    /// Registers an asset type, creating its [`AssetStore`] and wiring up the tracking system.
    ///
    /// Requires [`AssetManagerPlugin`](plugins::AssetManagerPlugin) to already be registered.
    pub fn register_asset<A: Asset>(&mut self) -> &mut Self {
        let asset_store = AssetStore::<A>::new();
        let asset_server = self
            .get_resource_mut::<AssetServer>()
            .expect("Asset Server not found");

        asset_server.register_asset::<A>(&asset_store);

        self.add_system(
            UpdateGroup::Update,
            |mut asset_store: ResMut<AssetStore<A>>,
             asset_server: ResMut<AssetServer>,
             events: EventWriter<AssetLifetimeEvent>| {
                asset_store.track_assets(asset_server, events);
            },
        );

        self.sub_apps.main.insert_resource(asset_store);
        self
    }

    /// Hands control to the configured runner function, consuming the app.
    pub fn run(mut self) {
        let runner = std::mem::replace(&mut self.runner, Box::new(run_once));
        (runner)(self);
    }

    /// Replaces the default runner with a custom one (e.g. a window event loop).
    pub fn set_runner(&mut self, f: impl FnOnce(App) -> AppExit + 'static) -> &mut Self {
        self.runner = Box::new(f);
        self
    }

    /// Registers a system in the given [`UpdateGroup`] on the main sub-app.
    ///
    /// To register a system on another sub-app, go through
    /// [`sub_app_mut`](App::sub_app_mut).
    pub fn add_system<M>(
        &mut self,
        update_group: UpdateGroup,
        system: impl IntoSystemConfig<M> + 'static,
    ) -> &mut Self {
        self.sub_apps.main.add_system(update_group, system);
        self
    }

    /// Registers an event type, creating its [`EventChannel`] resource and a flush system.
    ///
    /// Call this once per event type before any system uses [`EventWriter`] or [`EventReader`].
    pub fn register_event<T: Event + 'static>(&mut self) -> &mut Self {
        let event_channel = EventChannel::<T>::new();

        self.insert_resource(event_channel);
        self.add_system(UpdateGroup::LateUpdate, update_event_channel::<T>);
        self
    }

    /// Inserts a resource into the main sub-app's world (replacing any existing
    /// one of the same type).
    pub fn insert_resource<R: Resource>(&mut self, value: R) -> &mut Self {
        self.sub_apps.main.insert_resource(value);
        self
    }

    pub fn register_reflection<T: Component + for<'a> Facet<'a>>(&mut self) -> &mut Self {
        self.sub_apps.main.register_reflection::<T>();
        self
    }

    pub fn remove_resource<R: Resource>(&mut self) -> Option<R> {
        self.sub_apps.main.remove_resource()
    }

    pub fn get_resource<R: Resource>(&self) -> Option<&R> {
        self.sub_apps.main.get_resource()
    }

    pub fn get_resource_mut<R: Resource>(&mut self) -> Option<&mut R> {
        self.sub_apps.main.get_resource_mut()
    }

    pub fn with_resource<R: Resource, F, T: Resource>(&mut self, f: F)
    where
        F: FnOnce(R) -> T,
    {
        let Some(resource) = self.remove_resource::<R>() else {
            return;
        };
        let output = f(resource);
        self.insert_resource(output);
    }

    /// Runs one frame.
    ///
    /// The main sub-app runs FixedUpdate (as many times as needed), Update,
    /// LateUpdate, Render and LateRender.  Each additional sub-app then
    /// extracts from the main world and runs its own schedules.  Every world's
    /// tick is advanced at the end.
    pub fn update(&mut self) {
        profiling::scope!("App::update");

        let time = self
            .get_resource::<Time>()
            .expect("Time resource not found");

        self.accumulated_fixed_time += time.delta().as_secs_f32();

        while self.accumulated_fixed_time >= Time::fixed_delta_time() {
            profiling::scope!("fixed_update_step");
            self.sub_apps.main.run_fixed_update();
            self.accumulated_fixed_time -= Time::fixed_delta_time();
        }

        let fixed_overstep = self.accumulated_fixed_time;
        if let Some(time) = self.get_resource_mut::<Time>() {
            time.set_fixed_overstep(fixed_overstep);
        }

        self.sub_apps.main.run_update();
        self.sub_apps.main.run_render();

        // Sub-apps run after the main render phase, so that ordering still
        // holds once render systems move out of the main world.
        self.sub_apps.update_sub_apps();

        {
            profiling::scope!("world_tick");
            self.sub_apps.main.tick();
        }

        // The frame ends here: present_window has already run (LateRender),
        // and this also marks frames for the headless runner.
        profiling::finish_frame!();
    }

    /// Registers component lifecycle callbacks (`on_add` / `on_remove`) for `T`
    /// on the main sub-app's world.
    pub fn register_component_lifecycle<T: Component>(&mut self) -> &mut Self {
        self.sub_apps.main.register_component_lifecycle::<T>();
        self
    }

    /// Polls each plugin's [`ready`](Plugin::ready) method and transitions the state machine.
    ///
    /// Returns the current [`PluginsState`].
    pub fn plugin_state(&mut self) -> PluginsState {
        let next_state = match self.plugin_state {
            PluginsState::Building => {
                if self.plugins.iter().all(|plugin| plugin.ready(self)) {
                    PluginsState::Ready
                } else {
                    PluginsState::Building
                }
            }
            state => state,
        };

        self.plugin_state = next_state;

        next_state
    }

    /// Calls [`Plugin::finish`] on every registered plugin, then compiles and
    /// runs the `Startup` schedule of every sub-app.
    ///
    /// Should be called once after all plugins have been registered and all async work is ready.
    pub fn finish_plugin_build(&mut self) {
        let mut hokeypokey: Box<dyn Plugin> = Box::new(HokeyPokeyPlugin);
        let mut i = 0;
        while i < self.plugins.len() {
            core::mem::swap(&mut self.plugins[i], &mut hokeypokey);
            hokeypokey.finish(self);
            core::mem::swap(&mut self.plugins[i], &mut hokeypokey);
            i += 1;
        }

        self.plugin_state = PluginsState::Finished;

        // Main sub-app first in both passes: its startup systems set up the
        // state the other sub-apps will extract from.
        self.sub_apps
            .for_each(|sub_app| sub_app.compile_schedules());
        self.sub_apps.for_each(|sub_app| sub_app.run_startup());
    }
}

impl Default for App {
    fn default() -> Self {
        Self::new()
    }
}
