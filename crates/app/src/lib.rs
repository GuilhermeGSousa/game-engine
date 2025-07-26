use ecs::{
    bundle::ComponentBundle,
    component::Component,
    events::{
        event_channel::{update_event_channel, EventChannel},
        Event,
    },
    resource::{ResMut, Resource},
    system::{schedule::Schedule, IntoSystem},
    world::World,
};
use runner::{run_once, AppExit};
#[cfg(target_arch = "wasm32")]
use wasm_bindgen::prelude::*;

use essential::{
    assets::{asset_server::AssetServer, asset_store::AssetStore, Asset},
    time::Time,
};
use plugins::Plugin;
use std::mem::replace;
use update_group::UpdateGroup;

pub mod plugins;
pub mod runner;
pub mod update_group;

pub struct App {
    runner: runner::RunnerFn,
    world: World,
    startup_schedule: Schedule,
    update_schedule: Schedule,
    fixed_update_schedule: Schedule,
    late_update_schedule: Schedule,
    late_fixed_update_schedule: Schedule,
    render_schedule: Schedule,
    late_render_schedule: Schedule,

    accumulated_fixed_time: f32,
}

impl App {
    pub fn empty() -> App {
        Self {
            runner: Box::new(runner::run_once),
            world: World::new(),
            startup_schedule: Schedule::default(),
            update_schedule: Schedule::default(),
            fixed_update_schedule: Schedule::default(),
            late_update_schedule: Schedule::default(),
            late_fixed_update_schedule: Schedule::default(),
            render_schedule: Schedule::default(),
            late_render_schedule: Schedule::default(),
            accumulated_fixed_time: 0.0,
        }
    }

    pub fn register_plugin(&mut self, plugin: impl Plugin) -> &mut Self {
        plugin.build(self);
        self
    }
    pub fn register_asset<A: Asset>(&mut self) -> &mut Self {
        let asset_store = AssetStore::<A>::new();
        let asset_server = self
            .get_mut_resource::<AssetServer>()
            .expect("Asset Server not found");

        asset_server.register_asset::<A>(&asset_store);

        self.update_schedule.add_system(
            |mut asset_store: ResMut<AssetStore<A>>, asset_server: ResMut<AssetServer>| {
                asset_store.track_assets(asset_server);
            },
        );

        self.world.insert_resource(asset_store);
        self
    }

    pub fn run(&mut self) {
        self.startup_schedule.run(&mut self.world);
        let runner = replace(&mut self.runner, Box::new(run_once));
        let app = replace(self, App::empty());
        (runner)(app);
    }

    pub fn set_runner(&mut self, f: impl FnOnce(App) -> AppExit + 'static) -> &mut Self {
        self.runner = Box::new(f);
        self
    }

    pub fn spawn<T: ComponentBundle>(&mut self, bundle: T) -> &mut Self {
        self.world.spawn(bundle);
        self
    }

    pub fn add_system<M>(
        &mut self,
        update_group: UpdateGroup,
        system: impl IntoSystem<M> + 'static,
    ) -> &mut Self {
        match update_group {
            UpdateGroup::Startup => self.startup_schedule.add_system(system),
            UpdateGroup::Update => self.update_schedule.add_system(system),
            UpdateGroup::FixedUpdate => self.fixed_update_schedule.add_system(system),
            UpdateGroup::LateUpdate => self.late_update_schedule.add_system(system),
            UpdateGroup::LateFixedUpdate => self.late_fixed_update_schedule.add_system(system),
            UpdateGroup::Render => self.render_schedule.add_system(system),
            UpdateGroup::LateRender => self.late_render_schedule.add_system(system),
        };
        self
    }

    pub fn add_system_first<M>(
        &mut self,
        update_group: UpdateGroup,
        system: impl IntoSystem<M> + 'static,
    ) -> &mut Self {
        match update_group {
            UpdateGroup::Startup => self.startup_schedule.add_system_first(system),
            UpdateGroup::Update => self.update_schedule.add_system_first(system),
            UpdateGroup::FixedUpdate => self.fixed_update_schedule.add_system_first(system),
            UpdateGroup::LateUpdate => self.late_update_schedule.add_system_first(system),
            UpdateGroup::LateFixedUpdate => {
                self.late_fixed_update_schedule.add_system_first(system)
            }
            UpdateGroup::Render => self.render_schedule.add_system_first(system),
            UpdateGroup::LateRender => self.late_render_schedule.add_system_first(system),
        };
        self
    }

    pub fn register_event<T: Event + 'static>(&mut self) -> &mut Self {
        let event_channel = EventChannel::<T>::new();

        self.insert_resource(event_channel);
        self.add_system(UpdateGroup::LateUpdate, update_event_channel::<T>);
        self
    }

    pub fn insert_resource<R: Resource>(&mut self, value: R) -> &mut Self {
        self.world.insert_resource(value);
        self
    }

    pub fn remove_resource<R: Resource>(&mut self) -> Option<R> {
        self.world.remove_resource()
    }

    pub fn get_resource<R: Resource>(&self) -> Option<&R> {
        self.world.get_resource()
    }

    pub fn get_mut_resource<R: Resource>(&mut self) -> Option<&mut R> {
        self.world.get_resource_mut()
    }

    pub fn update(&mut self) {
        {
            let time = self
                .get_resource::<Time>()
                .expect("Time resource not found");

            let delta_time = time.delta();
            self.accumulated_fixed_time += delta_time;
            while self.accumulated_fixed_time >= Time::fixed_delta_time() {
                self.fixed_update_schedule.run(&mut self.world);
                self.accumulated_fixed_time -= Time::fixed_delta_time();
                self.late_fixed_update_schedule.run(&mut self.world);
            }
        }
        self.update_schedule.run(&mut self.world);
        self.late_update_schedule.run(&mut self.world);
        self.render_schedule.run(&mut self.world);
        self.late_render_schedule.run(&mut self.world);
        self.world.tick();
    }

    pub fn register_component_lifecycle<T: Component>(&mut self) -> &mut Self {
        self.world.register_component_lifetimes::<T>();
        self
    }
}
