pub mod archetype;
pub mod bundle;
pub mod common;
pub mod component;
pub mod entity;
pub mod entity_store;
pub mod events;
pub mod query;
pub mod resource;
pub mod system;
pub mod table;
pub mod world;

#[cfg(test)]
mod tests {
    use crate::{
        component::Component,
        query::Query,
        resource::{Res, Resource},
        system::{schedule::Schedule, system_input::StaticSystemInput},
        world::World,
    };

    #[derive(Component)]
    struct Health;

    #[derive(Component)]
    struct Position {
        pub x: f32,
        pub y: f32,
    }

    #[derive(Resource)]
    struct Time {
        pub time: f32,
    }

    impl Time {
        fn new() -> Self {
            Self { time: 10.0 }
        }
    }

    fn system_query(query: Query<(&Position, &mut Health)>) {
        for (position, hp) in query.iter() {
            print!("{}", position.x);
        }
    }

    fn system_query_2(query: Query<(&Position)>) {
        for (position) in query.iter() {
            print!("{}", position.x);
        }
    }

    fn system_query_3(res: StaticSystemInput<Res<'static, Time>>) {}

    #[test]
    fn test_query() {
        let mut world = World::new();
        let mut schedule = Schedule::new();

        schedule.add_system(system_query);
        schedule.add_system(system_query_2);
        schedule.add_system(system_query_3);

        world.spawn((Health, Position { x: 10.0, y: 20.0 }));
        world.spawn((Position { x: 20.0, y: 20.0 },));

        schedule.run(&mut world);
    }
}
