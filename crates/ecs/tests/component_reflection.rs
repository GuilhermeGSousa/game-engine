//! Covers the read side of the scene-component registry: which registered
//! components an entity carries, and what their current values are.
//!
//! This is what lets a tool inspect the live world instead of a parallel copy
//! of the scene file it was spawned from.
use ecs::component::scene::{SceneComponent, SceneSpawnContext};
use ecs::{Component, Entity, World};
use serde::{Deserialize, Serialize};

#[derive(Component, Serialize, Deserialize, PartialEq, Debug)]
struct Health {
    current: u32,
    max: u32,
}

impl SceneComponent for Health {
    fn apply(self, entity: Entity, ctx: &mut SceneSpawnContext<'_>) {
        ctx.insert(self, entity);
    }
}

#[derive(Component, Serialize, Deserialize, PartialEq, Debug)]
struct Armour {
    plates: u8,
}

impl SceneComponent for Armour {
    fn apply(self, entity: Entity, ctx: &mut SceneSpawnContext<'_>) {
        ctx.insert(self, entity);
    }
}

/// A plain component that is never registered as a scene component — engine
/// plumbing, from the inspector's point of view.
#[derive(Component)]
struct Plumbing;

fn names(world: &World, entity: Entity) -> Vec<&'static str> {
    let mut names: Vec<_> = world
        .component_types(entity)
        .map(|info| info.short())
        .collect();
    names.sort_unstable();
    names
}

#[test]
fn lists_the_registered_components_an_entity_carries() {
    let mut world = World::default();
    world.register_component_type::<Health>();
    world.register_component_type::<Armour>();

    let entity = world.spawn((
        Health {
            current: 3,
            max: 10,
        },
        Armour { plates: 2 },
    ));

    assert_eq!(
        names(&world, entity),
        vec!["Armour", "Health"],
        "every registered component on the entity must be listed"
    );
}

#[test]
fn reads_a_component_value_as_json() {
    let mut world = World::default();
    world.register_component_type::<Health>();
    let entity = world.spawn(Health {
        current: 3,
        max: 10,
    });

    assert_eq!(
        world.read_component(entity, "Health"),
        Some(serde_json::json!({ "current": 3, "max": 10 })),
        "the short alias must read the live component value"
    );
    assert_eq!(
        world.read_component(entity, Health::name()),
        Some(serde_json::json!({ "current": 3, "max": 10 })),
        "the canonical full type path must read the same value"
    );
}

#[test]
fn reads_reflect_later_mutations() {
    let mut world = World::default();
    world.register_component_type::<Health>();
    let entity = world.spawn(Health {
        current: 3,
        max: 10,
    });

    world
        .get_component_for_entity_mut::<Health>(entity)
        .unwrap()
        .current = 9;

    assert_eq!(
        world.read_component(entity, "Health"),
        Some(serde_json::json!({ "current": 9, "max": 10 })),
        "reads must see the world as it is now, not as it was spawned"
    );
}

#[test]
fn unregistered_components_are_invisible() {
    let mut world = World::default();
    world.register_component_type::<Health>();

    let entity = world.spawn((Health { current: 1, max: 1 }, Plumbing));

    assert_eq!(
        names(&world, entity),
        vec!["Health"],
        "a component that was never registered as a scene component must not be listed"
    );
}

#[test]
fn an_entity_with_no_registered_components_lists_nothing() {
    let mut world = World::default();
    world.register_component_type::<Health>();

    let entity = world.spawn(Plumbing);

    assert!(
        names(&world, entity).is_empty(),
        "an entity carrying only unregistered components must list nothing"
    );
}

#[test]
fn reading_a_component_the_entity_does_not_have_is_none() {
    let mut world = World::default();
    world.register_component_type::<Health>();
    world.register_component_type::<Armour>();
    let entity = world.spawn(Health { current: 1, max: 1 });

    assert_eq!(
        world.read_component(entity, "Armour"),
        None,
        "a registered type the entity lacks must read as None, not a default value"
    );
}

#[test]
fn reading_an_unregistered_name_is_none() {
    let mut world = World::default();
    let entity = world.spawn(Plumbing);

    assert_eq!(
        world.read_component(entity, "NeverRegistered"),
        None,
        "an unknown type name must report None rather than panicking"
    );
}

#[test]
fn a_stale_entity_lists_nothing_and_reads_nothing() {
    let mut world = World::default();
    world.register_component_type::<Health>();
    let entity = world.spawn(Health { current: 1, max: 1 });
    world.despawn(entity);

    assert!(
        names(&world, entity).is_empty(),
        "a despawned entity must not report components"
    );
    assert_eq!(
        world.read_component(entity, "Health"),
        None,
        "reading through a stale entity handle must not panic"
    );
}

#[test]
fn component_ids_include_components_that_are_not_scene_components() {
    let mut world = World::default();
    world.register_component_type::<Health>();

    let entity = world.spawn((
        Health {
            current: 3,
            max: 10,
        },
        Plumbing,
    ));

    let mut ids = world.component_ids(entity).to_vec();
    ids.sort();
    let mut expected = vec![
        std::any::TypeId::of::<Health>(),
        std::any::TypeId::of::<Plumbing>(),
    ];
    expected.sort();
    assert_eq!(ids, expected);
}

#[test]
fn a_stale_entity_has_no_component_ids() {
    let mut world = World::default();
    let entity = world.spawn((Plumbing,));
    world.despawn(entity);

    assert!(world.component_ids(entity).is_empty());
}

#[test]
fn type_info_is_only_known_for_scene_components() {
    let mut world = World::default();
    world.register_component_type::<Health>();
    world.spawn((Health { current: 1, max: 1 }, Plumbing));

    assert_eq!(
        world
            .type_info(std::any::TypeId::of::<Health>())
            .map(|info| info.short()),
        Some("Health")
    );
    assert!(
        world
            .type_info(std::any::TypeId::of::<Plumbing>())
            .is_none()
    );
}
