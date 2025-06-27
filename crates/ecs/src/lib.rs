pub mod archetype;
pub mod bundle;
pub mod command;
pub mod common;
pub mod component;
pub mod entity;
pub mod entity_store;
pub mod events;
pub mod query;
pub mod query_filter;
pub mod resource;
pub mod system;
pub mod table;
pub mod world;

#[cfg(test)]
mod tests {
    use crate::{
        command::CommandQueue,
        component::Component,
        entity::Entity,
        query::Query,
        query_filter::Added,
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

    fn system_query_pos_hp(query: Query<(Entity, &Position, &mut Health)>) {
        for (entity, position, hp) in query.iter() {
            print!("{}", position.x);
        }
    }

    fn system_query_pos(query: Query<(Entity, &Position)>) {
        for (entity, position) in query.iter() {
            print!("{}", position.x);
        }
    }

    fn system_query_time(res: Res<Time>) {}

    fn system_query_added(query: Query<(&Position,), Added<(Position,)>>) {
        for _ in query.iter() {
            print!("Found Added");
        }
    }

    fn spawn(mut cmd: CommandQueue) {
        let entity = cmd.spawn((Position { x: 0.0, y: 0.0 }, Health));
    }

    #[test]
    fn test_query() {
        let mut world = World::new();
        let mut schedule = Schedule::new();

        schedule.add_system(system_query_pos_hp);
        schedule.add_system(system_query_pos);

        world.spawn((Health, Position { x: 10.0, y: 20.0 }));
        world.spawn((Position { x: 20.0, y: 20.0 },));

        schedule.run(&mut world);
    }

    #[test]
    fn test_added() {
        let mut world = World::new();
        let mut schedule = Schedule::new();

        schedule.add_system(system_query_added);

        world.spawn((Position { x: 0.0, y: 0.0 },));
        schedule.run(&mut world);

        world.tick();

        world.spawn((Position { x: 0.0, y: 0.0 },));
        schedule.run(&mut world);

        world.tick();
        schedule.run(&mut world);
    }

    #[test]
    fn spawn_despawn() {
        let mut world = World::new();

        let e1 = world.spawn((Position { x: 0.0, y: 0.0 },));
        let e2 = world.spawn((Position { x: 0.0, y: 0.0 },));
        let e3 = world.spawn((Position { x: 0.0, y: 0.0 },));

        world.despawn(e2);

        let e4 = world.spawn((Position { x: 0.0, y: 0.0 },));

        assert!(e4.generation().get() == e2.generation().get() + 1);
        assert!(e4.index() == e2.index());
    }

    #[test]
    fn spawn_from_system() {
        let mut world = World::new();
        let mut schedule = Schedule::new();
        schedule.add_system(spawn);

        schedule.run(&mut world);

        let query = Query::<(&Position, &Health,)>::new(world.as_unsafe_world_cell_mut());

        assert_eq!(query.iter().count(), 1);

        for (position, hp,) in query.iter() {
            assert_eq!(position.x, 0.0);
            assert_eq!(position.y, 0.0);
        }
    }
}
