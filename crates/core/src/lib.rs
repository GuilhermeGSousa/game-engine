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
        schedule::{self, Schedule},
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

    fn system_query(query: Query<(&Position,)>) {
        //for (position,) in query.iter() {}
    }

    #[test]
    fn test_query() {
        let mut world = world::World::new();
        let mut schedule = Schedule::new();
        schedule.add_system(system_query);

        world.spawn((Health, Position));
        world.spawn((Position,));

        schedule.run(&mut world);
    }
}
