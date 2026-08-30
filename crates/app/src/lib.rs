#[cfg(all(feature = "multithreaded", not(target_arch = "wasm32")))]
use ecs::system::executor::multi_thread::MultiThreadedExecutor;
#[cfg(not(all(feature = "multithreaded", not(target_arch = "wasm32"))))]
use ecs::system::executor::single_thread::SingleThreadedExecutor;
use ecs::{
    component::Component,
    events::{
        event_channel::{update_event_channel, EventChannel},
        event_writer::EventWriter,
        Event,
    },
    resource::{ResMut, Resource},
    system::schedule::{CompiledSchedules, ScheduleLabel, Schedules},
    IntoSystemConfig, World,
};
use facet::Facet;
use log::info;
use runner::AppExit;

use essential::assets::{
    asset_server::AssetServer, asset_store::AssetStore, handle::AssetLifetimeEvent, Asset,
};

use crate::{
    plugins::PluginsState,
    runner::run_once,
    schedule_groups::{LateUpdate, Update},
    subapp::{RenderApp, SubApp, SubAppLabel, SubApps},
};

pub mod extractor;
pub mod main_schedule;
pub mod plugins;
pub mod runner;
pub mod schedule_groups;
pub mod subapp;

// Re-export the most commonly needed types so users don't have to know the module layout.
pub use plugins::Plugin;

pub(crate) struct HokeyPokeyPlugin;
impl Plugin for HokeyPokeyPlugin {
    fn build(&self, _: &mut App) {}
}

pub(crate) fn compile(schedules: Schedules, world: &mut World) -> CompiledSchedules {
    #[cfg(all(feature = "multithreaded", not(target_arch = "wasm32")))]
    {
        schedules.compile::<MultiThreadedExecutor>(world)
    }
    #[cfg(not(all(feature = "multithreaded", not(target_arch = "wasm32"))))]
    {
        schedules.compile::<SingleThreadedExecutor>(world)
    }
}

/// The top-level container for the game engine.
///
/// An `App` owns a [`World`], a set of per-group [`Schedule`]s, and a list of
/// [`Plugin`]s.  Call [`run`](App::run) to hand control over to the configured
/// runner (typically the window event loop).
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
    subapps: SubApps,
    plugins: Vec<Box<dyn Plugin>>,
    plugin_state: PluginsState,
}

impl App {
    pub fn new() -> App {
        Self {
            runner: Box::new(runner::run_once),
            subapps: SubApps::default(),
            plugins: Vec::new(),
            plugin_state: PluginsState::Building,
        }
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
            Update,
            |mut asset_store: ResMut<AssetStore<A>>,
             asset_server: ResMut<AssetServer>,
             events: EventWriter<AssetLifetimeEvent>| {
                asset_store.track_assets(asset_server, events);
            },
        );

        self.main_mut().insert_resource(asset_store);
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

    /// Registers a system in the schedule identified by `update_group`.
    pub fn add_system<M>(
        &mut self,
        update_group: impl ScheduleLabel,
        system: impl IntoSystemConfig<M> + 'static,
    ) -> &mut Self {
        self.get_resource_mut::<Schedules>()
            .expect("Schedules resource not found!")
            .add_system(update_group, system);

        self
    }

    /// Registers a system in the schedule identified by `update_group`, on the
    /// render subapp rather than the main one (e.g. systems meant to run in
    /// the [`Extract`](schedule_groups::Extract) schedule).
    pub fn add_render_system<M>(
        &mut self,
        update_group: impl ScheduleLabel,
        system: impl IntoSystemConfig<M> + 'static,
    ) -> &mut Self {
        self.subapps
            .sub_app_mut(RenderApp)
            .add_system(update_group, system);

        self
    }

    /// Registers an event type, creating its [`EventChannel`] resource and a flush system.
    ///
    /// Call this once per event type before any system uses [`EventWriter`] or [`EventReader`].
    pub fn register_event<T: Event + 'static>(&mut self) -> &mut Self {
        let event_channel = EventChannel::<T>::new();

        self.insert_resource(event_channel);
        self.add_system(LateUpdate, update_event_channel::<T>);
        self
    }

    /// Inserts a resource into the main world (replacing any existing one of the same type).
    pub fn insert_resource<R: Resource>(&mut self, value: R) -> &mut Self {
        self.main_mut().insert_resource(value);
        self
    }

    // Registers reflection for a component that implements the Facet trait
    // Allows the user to spawn that component through json (see [`CommandQueue`])
    pub fn register_reflection<T: Component + for<'a> Facet<'a>>(&mut self) -> &mut Self {
        self.main_mut().register_reflection::<T>();
        self
    }

    pub fn remove_resource<R: Resource>(&mut self) -> Option<R> {
        self.main_mut().remove_resource()
    }

    pub fn get_resource<R: Resource>(&self) -> Option<&R> {
        self.main().get_resource()
    }

    pub fn get_resource_mut<R: Resource>(&mut self) -> Option<&mut R> {
        self.main_mut().get_resource_mut()
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

    /// Runs all per-frame schedules: FixedUpdate (as many times as needed), Update,
    /// LateUpdate, Render, LateRender.  Also advances the world tick at the end.
    pub fn update(&mut self) {
        profiling::scope!("App::update");

        self.subapps.update();

        // The frame ends here: present_window has already run (LateRender),
        // and this also marks frames for the headless runner.
        profiling::finish_frame!();
    }

    pub fn main(&self) -> &SubApp {
        self.subapps.main()
    }

    pub fn main_mut(&mut self) -> &mut SubApp {
        self.subapps.main_mut()
    }

    /// Shorthand for [`sub_app`](Self::sub_app)`(`[`RenderApp`]`)`.
    pub fn render(&self) -> &SubApp {
        self.subapps.sub_app(RenderApp)
    }

    /// Shorthand for [`sub_app_mut`](Self::sub_app_mut)`(`[`RenderApp`]`)`.
    pub fn render_mut(&mut self) -> &mut SubApp {
        self.subapps.sub_app_mut(RenderApp)
    }

    /// Returns the [`SubApp`] registered under `label`, panicking if there is none.
    pub fn sub_app(&self, label: impl SubAppLabel) -> &SubApp {
        self.subapps.sub_app(label)
    }

    /// Returns the [`SubApp`] registered under `label`, panicking if there is none.
    pub fn sub_app_mut(&mut self, label: impl SubAppLabel) -> &mut SubApp {
        self.subapps.sub_app_mut(label)
    }

    pub fn get_sub_app(&self, label: impl SubAppLabel) -> Option<&SubApp> {
        self.subapps.get_sub_app(label)
    }

    pub fn get_sub_app_mut(&mut self, label: impl SubAppLabel) -> Option<&mut SubApp> {
        self.subapps.get_sub_app_mut(label)
    }

    /// Registers `sub_app` under `label`, replacing any existing sub-app with that label.
    pub fn insert_sub_app(&mut self, label: impl SubAppLabel, sub_app: SubApp) -> &mut Self {
        self.subapps.insert_sub_app(label, sub_app);
        self
    }

    /// Removes and returns the [`SubApp`] registered under `label`, if any.
    pub fn remove_sub_app(&mut self, label: impl SubAppLabel) -> Option<SubApp> {
        self.subapps.remove_sub_app(label)
    }

    /// Registers component lifecycle callbacks (`on_add` / `on_remove`) for `T`.
    pub fn register_component_lifetimes<T: Component>(&mut self) -> &mut Self {
        self.main_mut().register_component_lifetimes::<T>();
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

    /// Calls [`Plugin::finish`] on every registered plugin, then runs the `Startup` schedule.
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

        self.main_mut().compile_schedules();
        self.compile_sub_app_schedules();

        self.subapps.startup();
    }

    fn compile_sub_app_schedules(&mut self) {
        for sub_app in self.subapps.iter_sub_apps_mut() {
            sub_app.compile_schedules();
        }
    }

    /// Sets the extract closure for the [`RenderApp`] sub-app: it copies data
    /// from the main world into the render world each frame, before the render
    /// schedules run.
    pub fn set_extract_fn(
        &mut self,
        extract_fn: impl FnMut(&mut World, &mut World) + Send + 'static,
    ) {
        self.subapps
            .sub_app_mut(RenderApp)
            .set_extract(Box::new(extract_fn));
    }
}

impl Default for App {
    fn default() -> Self {
        Self::new()
    }
}
