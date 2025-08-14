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
pub mod utilities;
pub mod world;

#[cfg(test)]
mod tests {
    use crate::{
        command::CommandQueue, component::Component, entity::Entity, query::Query,
        query_filter::Added, system::schedule::Schedule, world::World,
    };

    #[derive(Component)]
    struct Health;

    #[derive(Component)]
    struct Position {
        pub x: f32,
        pub y: f32,
    }

    fn system_query_pos_hp(query: Query<(Entity, &Position, &mut Health)>) {
        for (_, position, _) in query.iter() {
            print!("{}", position.x);
        }
    }

    fn system_query_pos(query: Query<(Entity, &Position)>) {
        for (_, position) in query.iter() {
            print!("{}", position.x);
        }
    }

    fn system_query_added(query: Query<(&Position,), Added<(Position,)>>) {
        for _ in query.iter() {
            print!("Found Added");
        }
    }

    fn spawn(mut cmd: CommandQueue) {
        cmd.spawn((Position { x: 0.0, y: 0.0 }, Health));
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

        world.spawn((Position { x: 0.0, y: 0.0 },));
        let e2 = world.spawn((Position { x: 0.0, y: 0.0 },));
        world.spawn((Position { x: 0.0, y: 0.0 },));

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

        let query = Query::<(&Position, &Health)>::new(world.as_unsafe_world_cell_mut());

        assert_eq!(query.iter().count(), 1);

        for (position, _) in query.iter() {
            assert_eq!(position.x, 0.0);
            assert_eq!(position.y, 0.0);
        }
    }

    #[test]
    fn insert_component_on_new_archetype() {
        let mut world = World::new();

        let entity = world.spawn(Health);

        world.insert_component(Position { x: 10.0, y: 11.0 }, entity);

        let query = Query::<(&Position, &Health)>::new(world.as_unsafe_world_cell_mut());

        let mut count = 0;
        for (pos, _) in query.iter() {
            assert_eq!(pos.x, 10.0);
            assert_eq!(pos.y, 11.0);
            count += 1;
        }

        assert_eq!(count, 1);
    }

    #[test]
    fn insert_component_on_existing_archetype() {
        let mut world = World::new();

        let entity = world.spawn(Health);
        world.spawn((Health, Position { x: 10.0, y: 11.0 }));

        world.insert_component(Position { x: 10.0, y: 11.0 }, entity);

        let query = Query::<(&Position, &Health)>::new(world.as_unsafe_world_cell_mut());

        let mut count = 0;
        for (pos, _) in query.iter() {
            assert_eq!(pos.x, 10.0);
            assert_eq!(pos.y, 11.0);
            count += 1;
        }

        assert_eq!(count, 2);
    }
}
