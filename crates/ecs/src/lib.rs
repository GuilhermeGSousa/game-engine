//! Core Entity Component System (ECS) implementation.
//!
//! This crate provides the fundamental building blocks for the game engine's
//! data-oriented architecture:
//!
//! - [`World`] — the central container holding all entities, components, and resources.
//! - [`Entity`] — a lightweight handle representing a game object.
//! - [`Component`] — trait for data attached to entities; derive with `#[derive(Component)]`.
//! - [`Resource`] — trait for globally-shared data; derive with `#[derive(Resource)]`.
//! - [`Query`] — type-safe iterator over entities matching a set of components.
//! - [`Event`] — trait for messages passed between systems; derive with `#[derive(Event)]`.
//! - [`Schedule`] — ordered collection of systems run each frame.

pub mod archetype;
pub mod command;
pub mod common;
pub mod component;
pub mod entity;
pub mod events;
pub mod query;
pub mod resource;
pub mod system;
pub mod table;
pub mod utilities;
pub mod world;

// Commonly-used re-exports so downstream crates don't need to know the module layout.
pub use command::CommandQueue;
pub use component::Component;
pub use entity::Entity;
pub use events::Event;
pub use query::{
    query_filter::{Added, Changed, Or, With, Without},
    Query,
};
pub use resource::{Res, ResMut, Resource};
pub use system::{schedule::Schedule, IntoSystem, IntoSystemConfig, System, SystemConfig};
pub use world::World;

#[cfg(test)]
mod tests {
    use crate::{
        command::CommandQueue,
        component::Component,
        entity::Entity,
        events::{event_channel::EventChannel, Event},
        query::{
            query_filter::{Added, Changed, Or, With, Without},
            Query,
        },
        resource::{Res, ResMut, Resource},
        system::{executor::single_thread::SingleThreadedExecutor, schedule::Schedule},
        world::World,
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

    fn system_query_add_hp(query: Query<(&mut Health,)>) {
        for _ in query.iter() {}
    }

    fn system_query_hp_changed(query: Query<(&Health,), Changed<Health>>) {
        for _ in query.iter() {
            println!("Health change detected");
        }
    }

    fn system_filter_or(query: Query<Entity, Or<(With<Health>, With<Position>)>>) {
        for entity in query.iter() {
            println!("Entity {:?}", entity);
        }
    }

    fn spawn(mut cmd: CommandQueue) {
        cmd.spawn((Position { x: 0.0, y: 0.0 }, Health));
    }

    #[test]
    fn remove_last_component_leaves_empty_entity() {
        let mut world = World::new();

        let entity = world.spawn((Position { x: 1.0, y: 2.0 },));
        world.remove_component::<Position>(entity);

        assert!(
            world.get_component_for_entity::<Position>(entity).is_none(),
            "component should be gone after removal"
        );
    }

    #[test]
    fn test_query() {
        let mut world = World::new();
        let mut schedule = Schedule::new();

        schedule.add_system(system_query_pos_hp);
        schedule.add_system(system_query_pos);

        world.spawn((Health, Position { x: 10.0, y: 20.0 }));
        world.spawn((Position { x: 20.0, y: 20.0 },));

        schedule.compile::<SingleThreadedExecutor>().run(&mut world);
    }

    #[test]
    fn test_added() {
        let mut world = World::new();
        let mut schedule = Schedule::new();

        schedule.add_system(system_query_added);

        world.spawn((Position { x: 0.0, y: 0.0 },));
        let mut compiled_schedule = schedule.compile::<SingleThreadedExecutor>();
        compiled_schedule.run(&mut world);

        world.tick();

        world.spawn((Position { x: 0.0, y: 0.0 },));
        compiled_schedule.run(&mut world);

        world.tick();
        compiled_schedule.run(&mut world);
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

        schedule.compile::<SingleThreadedExecutor>().run(&mut world);

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

    #[test]
    fn insert_twice() {
        let mut world = World::new();

        let entity = world.spawn(Health);

        world.insert_component(Position { x: 0.0, y: 0.0 }, entity);
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
    fn test_change_detection() {
        let mut world = World::new();

        world.spawn((Health, Position { x: 10.0, y: 20.0 }));
        world.spawn((Health, Position { x: 10.0, y: 20.0 }));
        world.spawn((Health, Position { x: 10.0, y: 20.0 }));

        world.tick();
        let mut schedule = Schedule::new();
        schedule.add_system(system_query_add_hp);
        schedule.add_system(system_query_hp_changed);

        schedule.compile::<SingleThreadedExecutor>().run(&mut world);
    }

    #[test]
    fn without_filter_prunes_archetypes() {
        let mut world = World::new();

        world.spawn((Health, Position { x: 1.0, y: 2.0 }));
        world.spawn((Position { x: 3.0, y: 4.0 },));
        world.spawn((Position { x: 5.0, y: 6.0 },));

        let with_health = Query::<&Position, With<Health>>::new(world.as_unsafe_world_cell_mut());
        assert_eq!(with_health.iter().count(), 1);

        let without_health =
            Query::<&Position, Without<Health>>::new(world.as_unsafe_world_cell_mut());
        assert_eq!(without_health.iter().count(), 2);
    }

    #[test]
    fn mutable_query_writes_persist() {
        let mut world = World::new();

        let entity = world.spawn((Position { x: 1.0, y: 2.0 },));
        world.spawn((Position { x: 10.0, y: 20.0 },));

        {
            let query = Query::<&mut Position>::new(world.as_unsafe_world_cell_mut());
            for mut position in query.iter() {
                position.x += 100.0;
            }
        }

        assert_eq!(
            world.get_component_for_entity::<Position>(entity).unwrap().x,
            101.0
        );
    }

    #[derive(Resource, Default)]
    struct MatchCount(usize);

    fn count_positions(query: Query<&Position>, mut count: ResMut<MatchCount>) {
        count.0 = query.iter().count();
    }

    #[test]
    fn cached_query_state_sees_archetypes_created_between_runs() {
        // The cached QueryState only inspects newly-appended archetypes each
        // run; make sure an archetype created after the first run is still
        // matched on the second run.
        let mut world = World::new();
        world.insert_resource(MatchCount::default());
        world.spawn((Position { x: 0.0, y: 0.0 },));

        let mut schedule = Schedule::new();
        schedule.add_system(count_positions);
        let mut compiled = schedule.compile::<SingleThreadedExecutor>();

        compiled.run(&mut world);
        assert_eq!(world.get_resource::<MatchCount>().unwrap().0, 1);

        // New archetype (Position + Health) appears after the first run.
        world.spawn((Position { x: 1.0, y: 1.0 }, Health));
        compiled.run(&mut world);
        assert_eq!(world.get_resource::<MatchCount>().unwrap().0, 2);
    }

    #[test]
    fn test_or_query() {
        let mut world = World::new();

        world.spawn((Health, Position { x: 10.0, y: 20.0 }));
        world.spawn((Health, Position { x: 10.0, y: 20.0 }));
        world.spawn(Position { x: 10.0, y: 20.0 });

        let mut schedule = Schedule::new();
        schedule.add_system(system_filter_or);

        schedule.compile::<SingleThreadedExecutor>().run(&mut world);
    }

    #[test]
    fn test_add_children() {
        let mut world = World::new();

        let entity_parent = world.spawn((Health, Position { x: 10.0, y: 20.0 }));
        let entity_child_1 = world.spawn((Health, Position { x: 10.0, y: 20.0 }));
        let entity_child_2 = world.spawn((Health, Position { x: 10.0, y: 20.0 }));

        world.add_child(entity_parent, entity_child_1);
        world.add_child(entity_parent, entity_child_2);
    }

    #[derive(Resource)]
    struct Score(u32);

    #[derive(Resource)]
    struct DoubleScore(u32);

    #[test]
    fn insert_and_get_resource() {
        let mut world = World::new();
        world.insert_resource(Score(42));
        assert_eq!(world.get_resource::<Score>().unwrap().0, 42);
    }

    #[test]
    fn remove_resource() {
        let mut world = World::new();
        world.insert_resource(Score(10));
        let removed = world.remove_resource::<Score>();
        assert!(removed.is_some());
        assert_eq!(removed.unwrap().0, 10);
        assert!(world.get_resource::<Score>().is_none());
    }

    #[test]
    fn overwrite_resource() {
        let mut world = World::new();
        world.insert_resource(Score(1));
        world.insert_resource(Score(99));
        assert_eq!(world.get_resource::<Score>().unwrap().0, 99);
    }

    fn read_score(score: Res<Score>, mut result: ResMut<DoubleScore>) {
        result.0 = score.0 * 2;
    }

    #[test]
    fn resource_system_read_write() {
        let mut world = World::new();
        world.insert_resource(Score(5));
        world.insert_resource(DoubleScore(0));

        let mut schedule = Schedule::new();
        schedule.add_system(read_score);
        schedule.compile::<SingleThreadedExecutor>().run(&mut world);

        assert_eq!(world.get_resource::<DoubleScore>().unwrap().0, 10);
    }

    // ----- event tests -----

    #[derive(Event)]
    struct PlayerDied {
        score: u32,
    }

    fn send_death(mut writer: crate::events::event_writer::EventWriter<PlayerDied>) {
        writer.write(PlayerDied { score: 77 });
    }

    fn count_deaths(
        reader: crate::events::event_reader::EventReader<PlayerDied>,
        mut counter: ResMut<Score>,
    ) {
        for event in reader.read() {
            counter.0 += event.score;
        }
    }

    #[test]
    fn events_sent_and_read_same_frame() {
        let mut world = World::new();
        world.insert_resource(EventChannel::<PlayerDied>::new());
        world.insert_resource(Score(0));

        let mut schedule = Schedule::new();
        schedule.add_system(send_death);
        schedule.add_system(count_deaths);
        schedule.compile::<SingleThreadedExecutor>().run(&mut world);

        assert_eq!(world.get_resource::<Score>().unwrap().0, 77);
    }

    #[test]
    fn events_flushed_next_frame() {
        let mut world = World::new();
        world.insert_resource(EventChannel::<PlayerDied>::new());
        world.insert_resource(Score(0));

        let mut frame1 = Schedule::new();
        frame1.add_system(send_death);
        let mut frame1 = frame1.compile::<SingleThreadedExecutor>();

        let mut flush = Schedule::new();
        flush.add_system(crate::events::event_channel::update_event_channel::<PlayerDied>);
        let mut flush = flush.compile::<SingleThreadedExecutor>();

        let mut frame2 = Schedule::new();
        frame2.add_system(count_deaths);
        let mut frame2 = frame2.compile::<SingleThreadedExecutor>();

        frame1.run(&mut world);
        flush.run(&mut world);
        frame2.run(&mut world);

        // After flushing, count_deaths should see 0 events.
        assert_eq!(world.get_resource::<Score>().unwrap().0, 0);
    }

    // ----- remove_component tests -----

    #[test]
    fn remove_component_leaves_other_components() {
        let mut world = World::new();
        let e = world.spawn((Health, Position { x: 3.0, y: 4.0 }));

        world.remove_component::<Health>(e);

        let q = Query::<&Position>::new(world.as_unsafe_world_cell_mut());
        assert_eq!(q.iter().count(), 1);

        // Entity should no longer appear in a query that requires Health.
        let q2 = Query::<Entity, With<Health>>::new(world.as_unsafe_world_cell_mut());
        assert_eq!(q2.iter().count(), 0);
    }

    // ----- Without filter test -----

    #[test]
    fn without_filter() {
        let mut world = World::new();
        world.spawn((Health, Position { x: 1.0, y: 0.0 }));
        world.spawn(Position { x: 2.0, y: 0.0 });

        // Only the entity without Health should be returned.
        let q = Query::<&Position, crate::query::query_filter::Without<Health>>::new(
            world.as_unsafe_world_cell_mut(),
        );
        let results: Vec<_> = q.iter().collect();
        assert_eq!(results.len(), 1);
        assert_eq!(results[0].x, 2.0);
    }

    // ----- Added filter after insert test -----

    #[test]
    fn added_filter_survives_insert() {
        let mut world = World::new();

        world.tick();

        let entity = world.spawn(Health);

        // Health was added this tick — the filter must match.
        let q = Query::<Entity, Added<Health>>::new(world.as_unsafe_world_cell_mut());
        assert_eq!(q.iter().count(), 1);

        // Insert a second component on the same entity (moves it to a new archetype).
        world.insert_component(Position { x: 1.0, y: 2.0 }, entity);

        // Health was still added this tick — filter must still match.
        let q = Query::<Entity, Added<Health>>::new(world.as_unsafe_world_cell_mut());
        assert_eq!(q.iter().count(), 1);
    }

    // ----- tick preservation test -----

    #[test]
    fn component_ticks_preserved_after_sibling_insert() {
        use crate::component::ComponentId;

        let mut world = World::new();

        // Advance to tick 1 so the added_tick for Health is distinct from the initial
        // changed_tick (which is always 0), making each assertion unambiguous.
        world.tick();

        let entity = world.spawn(Health);

        // Health was just added at tick 1.
        assert!(world.was_component_added(entity, ComponentId::of::<Health>()));
        assert!(world.was_component_changed(entity, ComponentId::of::<Health>()));

        // Advance to tick 2 before inserting Position.
        world.tick();

        // Insert a second component — entity migrates to a new archetype.
        world.insert_component(Position { x: 1.0, y: 2.0 }, entity);

        // Health's added_tick must still be tick 1, not the new tick 2.
        assert!(!world.was_component_added(entity, ComponentId::of::<Health>()));
        // Health was never mutated, so changed_tick is still 0 — not the current tick 2.
        assert!(!world.was_component_changed(entity, ComponentId::of::<Health>()));

        // Position was freshly added at tick 2.
        assert!(world.was_component_added(entity, ComponentId::of::<Position>()));
        assert!(world.was_component_changed(entity, ComponentId::of::<Position>()));
    }

    // ----- get_entity test -----

    #[test]
    fn get_entity_returns_none_for_missing_component() {
        let mut world = World::new();
        let e_with = world.spawn((Health, Position { x: 5.0, y: 0.0 }));
        let e_without = world.spawn(Position { x: 6.0, y: 0.0 });

        let q = Query::<&Health>::new(world.as_unsafe_world_cell_mut());
        assert!(q.get_entity(e_with).is_some());
        assert!(q.get_entity(e_without).is_none());
    }
}
