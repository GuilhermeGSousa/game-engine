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
        schedule, world,
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

    fn system_easy(input: ()) {
        // Do something to test this system
    }

    fn system_intermediate(query: Query<(&Position,)>) {
        // Do something to test this system
    }

    fn system_intermediate_2(time: Res<Time>) {
        // Do something to test this system
    }

    fn system_intermediate_hard(a: (Query<(&Position,)>, Res<Time>)) {
        // Do something to test this system
    }

    fn system_intermediate_hard_2(a: Query<(&Position,)>, time: Res<Time>) {
        // Do something to test this system
    }

    #[test]
    fn spawn_entity() {
        let mut world = world::World::new();

        let mut schedule = schedule::Schedule::new();
        schedule
            .add_system(system_easy)
            .add_system(system_intermediate)
            .add_system(system_intermediate_2)
            .add_system(system_intermediate_hard)
            .add_system(system_intermediate_hard_2);

        world.spawn((Health, Position));

        world.insert_resource(Time::new());

        schedule.run(&mut world);
    }
}
