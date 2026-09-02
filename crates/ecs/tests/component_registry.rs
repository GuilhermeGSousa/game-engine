//! Covers the serde component registry: a registered type deserializes from
//! JSON and is applied; an unregistered type name is reported rather than
//! panicking.
use ecs::component::scene::{SceneComponent, SceneSpawnContext};
use ecs::{Component, Entity, World};
use serde::{Deserialize, Serialize};

#[derive(Component, Serialize, Deserialize, PartialEq, Debug)]
struct Registered {
    value: u32,
}

impl SceneComponent for Registered {
    fn apply(self, entity: Entity, ctx: &mut SceneSpawnContext<'_>) {
        ctx.insert(self, entity);
    }
}

#[test]
fn registered_component_is_deserialized_and_applied() {
    let mut world = World::default();
    world.register_component_type::<Registered>();
    let entity = world.spawn(());

    let applied = world.apply_scene_component("Registered", r#"{"value":42}"#, entity, &[entity]);

    assert!(
        applied,
        "a registered type name must be found in the registry"
    );
    assert_eq!(
        world.get_component_for_entity::<Registered>(entity),
        Some(&Registered { value: 42 }),
        "the JSON payload must be deserialized into the real component and inserted"
    );
}

#[test]
fn short_name_alias_resolves_for_blender_authored_extras() {
    let mut world = World::default();
    world.register_component_type::<Registered>();
    let entity = world.spawn(());

    // Blender authors the bare identifier, not the full Rust path.
    let applied = world.apply_scene_component("Registered", r#"{"value":7}"#, entity, &[entity]);

    assert!(
        applied,
        "a short, Blender-style name must resolve to the registered type"
    );
    assert_eq!(
        world.get_component_for_entity::<Registered>(entity),
        Some(&Registered { value: 7 }),
        "the short alias must apply the same component the full path would"
    );
}

#[test]
fn full_type_path_also_resolves() {
    let mut world = World::default();
    world.register_component_type::<Registered>();
    let entity = world.spawn(());

    let applied =
        world.apply_scene_component(Registered::name(), r#"{"value":8}"#, entity, &[entity]);

    assert!(
        applied,
        "the full type path is the canonical registry key and must resolve"
    );
}

#[test]
fn unregistered_component_name_is_reported_not_fatal() {
    let mut world = World::default();
    let entity = world.spawn(());

    let applied = world.apply_scene_component("NeverRegistered", "{}", entity, &[entity]);

    assert!(
        !applied,
        "an unregistered type name must report false rather than panicking"
    );
}

#[test]
fn malformed_json_does_not_panic() {
    let mut world = World::default();
    world.register_component_type::<Registered>();
    let entity = world.spawn(());

    let applied = world.apply_scene_component("Registered", "{ not json", entity, &[entity]);

    assert!(
        !applied,
        "malformed payloads must be skipped, not propagated as a panic"
    );
    assert!(
        world
            .get_component_for_entity::<Registered>(entity)
            .is_none(),
        "a failed deserialize must not leave a partial component behind"
    );
}
