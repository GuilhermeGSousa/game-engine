use bevy_ecs::schedule::{IntoSystemConfigs, Schedule};
use bevy_ecs::system::Resource;
use bevy_ecs::world::{Mut, World};
use runner::{run_once, AppExit};
#[cfg(target_arch = "wasm32")]
use wasm_bindgen::prelude::*;

use plugin::Plugin;
use std::mem::replace;
use update_group::UpdateGroup;

pub mod plugin;
pub mod runner;
pub mod update_group;

pub struct App {
    runner: runner::RunnerFn,
    world: World,
    update_schedule: Schedule,
}

impl App {
    pub fn empty() -> App {
        Self {
            runner: Box::new(runner::run_once),
            world: World::default(),
            update_schedule: Schedule::default(),
        }
    }

    pub fn register_plugin(&mut self, plugin: impl Plugin) -> &mut Self {
        plugin.build(self);
        self
    }

    pub fn run(&mut self) {
        let runner = replace(&mut self.runner, Box::new(run_once));
        let app = replace(self, App::empty());
        (runner)(app);
    }

    pub fn set_runner(&mut self, f: impl FnOnce(App) -> AppExit + 'static) -> &mut Self {
        self.runner = Box::new(f);
        self
    }

    pub fn add_system<T>(
        &mut self,
        update_group: UpdateGroup,
        system: impl IntoSystemConfigs<T>,
    ) -> &mut Self {
        self.update_schedule.add_systems(system);
        self
    }

    pub fn insert_non_send_resource<R: 'static>(&mut self, value: R) -> &mut Self {
        self.world.insert_non_send_resource(value);
        self
    }

    pub fn insert_resource<R: Resource>(&mut self, value: R) -> &mut Self {
        self.world.insert_resource(value);
        self
    }

    pub fn remove_non_send_resource<R: 'static>(&mut self) -> Option<R> {
        self.world.remove_non_send_resource()
    }

    pub fn resource<R: Resource>(&self) -> &R {
        self.world.resource()
    }

    pub fn get_mut_resource<R: Resource>(&mut self) -> Option<Mut<'_, R>> {
        self.world.get_resource_mut()
    }

    pub fn update(&mut self) {
        self.update_schedule.run(&mut self.world);
    }
}
