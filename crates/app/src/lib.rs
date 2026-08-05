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

use crate::{
    plugins::PluginsState,
    runner::run_once,
    schedule_groups::{
        FixedUpdate, LateFixedUpdate, LateRender, LateUpdate, Render, Startup, Update,
    },
};

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
    world: World,
    accumulated_fixed_time: f32,
    plugins: Vec<Box<dyn Plugin>>,
    plugin_state: PluginsState,
}

impl App {
    pub fn new() -> App {
        let mut world = World::new();
        world.init_resource::<Schedules>();
        Self {
            runner: Box::new(runner::run_once),
            world,
            accumulated_fixed_time: 0.0,
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

        self.world.insert_resource(asset_store);
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

    /// Registers an event type, creating its [`EventChannel`] resource and a flush system.
    ///
    /// Call this once per event type before any system uses [`EventWriter`] or [`EventReader`].
    pub fn register_event<T: Event + 'static>(&mut self) -> &mut Self {
        let event_channel = EventChannel::<T>::new();

        self.insert_resource(event_channel);
        self.add_system(LateUpdate, update_event_channel::<T>);
        self
    }

    /// Inserts a resource into the world (replacing any existing one of the same type).
    pub fn insert_resource<R: Resource>(&mut self, value: R) -> &mut Self {
        self.world.insert_resource(value);
        self
    }

    pub fn register_reflection<T: Component + for<'a> Facet<'a>>(&mut self) -> &mut Self {
        self.world.register_reflection::<T>();
        self
    }

    pub fn remove_resource<R: Resource>(&mut self) -> Option<R> {
        self.world.remove_resource()
    }

    pub fn get_resource<R: Resource>(&self) -> Option<&R> {
        self.world.get_resource()
    }

    pub fn get_resource_mut<R: Resource>(&mut self) -> Option<&mut R> {
        self.world.get_resource_mut()
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

        let time = self
            .get_resource::<Time>()
            .expect("Time resource not found");

        self.accumulated_fixed_time += time.delta().as_secs_f32();

        let mut schedules = self
            .remove_resource::<CompiledSchedules>()
            .expect("Compiled schedules not found!");

        while self.accumulated_fixed_time >= Time::fixed_delta_time() {
            profiling::scope!("fixed_update_step");
            if let Some(schedule) = schedules.get_mut(FixedUpdate) {
                profiling::scope!("schedule::fixed_update");
                schedule.run(&mut self.world)
            }

            if let Some(schedule) = schedules.get_mut(LateFixedUpdate) {
                profiling::scope!("schedule::late_fixed_update");
                schedule.run(&mut self.world)
            }

            self.accumulated_fixed_time -= Time::fixed_delta_time();
        }

        let fixed_overstep = self.accumulated_fixed_time;
        if let Some(time) = self.get_resource_mut::<Time>() {
            time.set_fixed_overstep(fixed_overstep);
        }

        if let Some(schedule) = schedules.get_mut(Update) {
            profiling::scope!("schedule::update");
            schedule.run(&mut self.world)
        }

        if let Some(schedule) = schedules.get_mut(LateUpdate) {
            profiling::scope!("schedule::late_update");
            schedule.run(&mut self.world)
        }

        if let Some(schedule) = schedules.get_mut(Render) {
            profiling::scope!("schedule::render");
            schedule.run(&mut self.world)
        }

        if let Some(schedule) = schedules.get_mut(LateRender) {
            profiling::scope!("schedule::late_render");
            schedule.run(&mut self.world)
        }

        self.world.insert_resource(schedules);

        {
            profiling::scope!("world_tick");
            self.world.tick();
        }

        // The frame ends here: present_window has already run (LateRender),
        // and this also marks frames for the headless runner.
        profiling::finish_frame!();
    }

    /// Registers component lifecycle callbacks (`on_add` / `on_remove`) for `T`.
    pub fn register_component_lifecycle<T: Component>(&mut self) -> &mut Self {
        self.world.register_component_lifetimes::<T>();
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

        self.compile_schedules();

        let mut schedules = self
            .remove_resource::<CompiledSchedules>()
            .expect("Compiled schedules not found!");

        if let Some(schedule) = schedules.get_mut(Startup) {
            schedule.run(&mut self.world);
        }

        self.insert_resource(schedules);
    }

    fn compile_schedules(&mut self) {
        let schedules = self
            .remove_resource::<Schedules>()
            .expect("Schedules resource not found!");

        #[cfg(all(feature = "multithreaded", not(target_arch = "wasm32")))]
        let compiled_schedules = schedules.compile::<MultiThreadedExecutor>();
        #[cfg(not(all(feature = "multithreaded", not(target_arch = "wasm32"))))]
        let compiled_schedules = schedules.compile::<SingleThreadedExecutor>();

        self.insert_resource(compiled_schedules);
    }
}

impl Default for App {
    fn default() -> Self {
        Self::new()
    }
}
