use ecs::{
    system::schedule::{CompiledSchedules, Schedules},
    Component, Resource, World,
};
use essential::time::Time;
use facet::Facet;

#[derive(Default)]
pub struct SubApps {
    main: SubApp,
    render: SubApp,
    accumulated_fixed_time: f32,
}

impl SubApps {
    pub fn main(&self) -> &SubApp {
        &self.main
    }

    pub fn main_mut(&mut self) -> &mut SubApp {
        &mut self.main
    }

    pub fn render(&self) -> &SubApp {
        &self.render
    }

    pub fn render_mut(&mut self) -> &mut SubApp {
        &mut self.render
    }

    pub fn update(&mut self) {
        let time = self
            .main
            .get_resource::<Time>()
            .expect("Time resource not found");

        self.accumulated_fixed_time += time.delta().as_secs_f32();

        let mut schedules = self
            .main
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
    }
}

pub struct SubApp {
    world: World,
}

impl Default for SubApp {
    fn default() -> Self {
        let mut world = World::new();
        world.init_resource::<Schedules>();
        Self { world }
    }
}

impl SubApp {
    pub fn insert_resource<R: Resource>(&mut self, value: R) -> &mut Self {
        self.world.insert_resource(value);
        self
    }
    pub fn remove_resource<R: Resource>(&mut self) -> Option<R> {
        self.world.remove_resource::<R>()
    }

    pub fn get_resource<R: Resource>(&self) -> Option<&R> {
        self.world.get_resource::<R>()
    }

    pub fn get_resource_mut<R: Resource>(&mut self) -> Option<&mut R> {
        self.world.get_resource_mut::<R>()
    }

    pub fn register_component_lifetimes<T: Component>(&mut self) -> &mut Self {
        self.world.register_component_lifetimes::<T>();
        self
    }

    pub fn register_reflection<T: Component + for<'a> Facet<'a>>(&mut self) -> &mut Self {
        self.world.register_reflection::<T>();
        self
    }
}
