pub mod archetype;
pub mod bundle;
pub mod common;
pub mod component;
pub mod entity;
pub mod query;
pub mod resource;
pub mod schedule;
pub mod system;
pub mod system_input;
pub mod table;
pub mod world;

#[cfg(test)]
mod tests {

    use crate::{
        component::Component,
        query::Query,
        resource::{Res, Resource},
        schedule,
        system::{FunctionSystem, ScheduledSystem, System},
        world,
    };

    #[derive(Component)]
    struct Health;

    #[derive(Component)]
    struct Position;

    #[derive(Resource)]
    struct Time {
        pub time: f32,
    }

    impl Time {
        fn new() -> Self {
            Self { time: 10.0 }
        }
    }

    fn system_easy() {
        // Do something to test this system
    }

    fn system_intermediate(query: Query<(Position,)>) {
        // Do something to test this system
    }

    #[test]
    fn spawn_entity() {
        let mut world = world::World::new();

        let a: Box<dyn Fn() + 'static> = Box::new(system_easy);
        let b: Box<dyn Fn(Query<(Position,)>) + 'static> = Box::new(system_intermediate);

        let a = FunctionSystem::new(system_easy);
        let b = FunctionSystem::new(system_intermediate);
        let mut schedule = schedule::Scheduler::default()
            .add_scheduled_system(ScheduledSystem::new(a))
            .add_scheduled_system(ScheduledSystem::new(b));
        //let schedule = schedule::Scheduler::default().add_system(system_intermediate);

        world.spawn((Health, Position));

        world.insert_resource(Time::new());

        schedule.run(&mut world);
    }
}
