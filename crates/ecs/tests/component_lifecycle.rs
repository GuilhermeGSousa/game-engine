//! Component lifecycle callbacks: when they fire, what they can see, and
//! whether structural changes made *inside* callbacks (inserts, removals,
//! despawns) leave every other entity's location intact.

use ecs::component::{Component, ComponentLifecycleCallback};
use ecs::entity::Entity;
use ecs::resource::Resource;
use ecs::world::World;

#[derive(Component)]
struct Value(i32);

#[derive(Component)]
struct Extra;

#[derive(Component)]
struct Marker;

/// Records every lifecycle firing and whether the component could read
/// itself at that moment.
#[derive(Resource, Default)]
struct TrackLog {
    added: Vec<(Entity, bool)>,
    removed: Vec<(Entity, bool)>,
}

struct Tracked;

impl Component for Tracked {
    fn on_add() -> Option<ComponentLifecycleCallback> {
        Some(|mut world, context| {
            let readable = world
                .get_component_for_entity::<Tracked>(context.entity)
                .is_some();
            if let Some(log) = world.get_resource_mut::<TrackLog>() {
                log.added.push((context.entity, readable));
            }
        })
    }

    fn on_remove() -> Option<ComponentLifecycleCallback> {
        Some(|mut world, context| {
            let readable = world
                .get_component_for_entity::<Tracked>(context.entity)
                .is_some();
            if let Some(log) = world.get_resource_mut::<TrackLog>() {
                log.removed.push((context.entity, readable));
            }
        })
    }
}

#[derive(Resource, Default)]
struct SiblingLog(Vec<bool>);

/// Records whether the sibling `Marker` was visible from `on_add`.
struct WithSibling;

impl Component for WithSibling {
    fn on_add() -> Option<ComponentLifecycleCallback> {
        Some(|mut world, context| {
            let sibling = world
                .get_component_for_entity::<Marker>(context.entity)
                .is_some();
            if let Some(log) = world.get_resource_mut::<SiblingLog>() {
                log.0.push(sibling);
            }
        })
    }
}

#[derive(Resource, Default)]
struct CompanionLog {
    adds: u32,
    removes: u32,
}

struct Companion;

impl Component for Companion {
    fn on_add() -> Option<ComponentLifecycleCallback> {
        Some(|mut world, _context| {
            if let Some(log) = world.get_resource_mut::<CompanionLog>() {
                log.adds += 1;
            }
        })
    }

    fn on_remove() -> Option<ComponentLifecycleCallback> {
        Some(|mut world, _context| {
            if let Some(log) = world.get_resource_mut::<CompanionLog>() {
                log.removes += 1;
            }
        })
    }
}

/// Inserts a `Companion` from `on_add` (triggering its callbacks) and removes
/// it from `on_remove` with events suppressed. During a despawn the
/// `Companion` still gets one `on_remove` firing from despawn's own pass over
/// the component list captured at despawn start; the suppression prevents a
/// second firing from the removal itself.
struct Body;

impl Component for Body {
    fn on_add() -> Option<ComponentLifecycleCallback> {
        Some(|mut world, context| {
            world.insert(Companion, context.entity, true);
        })
    }

    fn on_remove() -> Option<ComponentLifecycleCallback> {
        Some(|mut world, context| {
            world.remove_component::<Companion>(context.entity, false);
        })
    }
}

/// Inserts a `Companion` from `on_add` with events suppressed.
struct Quiet;

impl Component for Quiet {
    fn on_add() -> Option<ComponentLifecycleCallback> {
        Some(|mut world, context| {
            world.insert(Companion, context.entity, false);
        })
    }
}

#[derive(Resource, Default)]
struct DespawnTarget(Option<Entity>);

/// Despawns the entity named in `DespawnTarget` (taken, so nested firings
/// see `None`) from `on_remove`.
struct Reaper;

impl Component for Reaper {
    fn on_remove() -> Option<ComponentLifecycleCallback> {
        Some(|mut world, _context| {
            let target = world
                .get_resource_mut::<DespawnTarget>()
                .and_then(|target| target.0.take());
            if let Some(target) = target {
                world.despawn(target);
            }
        })
    }
}

fn value_of(world: &World, entity: Entity) -> Option<i32> {
    world
        .get_component_for_entity::<Value>(entity)
        .map(|value| value.0)
}

#[test]
fn spawn_fires_on_add_with_bundle_fully_inserted() {
    let mut world = World::new();
    world.register_component_lifetimes::<Tracked>();
    world.register_component_lifetimes::<WithSibling>();
    world.insert_resource(TrackLog::default());
    world.insert_resource(SiblingLog::default());

    let first = world.spawn((WithSibling, Tracked, Marker));
    let second = world.spawn((Marker, Tracked, WithSibling));

    let log = world.get_resource::<TrackLog>().unwrap();
    assert_eq!(
        log.added,
        vec![(first, true), (second, true)],
        "on_add should fire once per spawn, for the right entity, with the component readable"
    );

    let siblings = world.get_resource::<SiblingLog>().unwrap();
    assert_eq!(
        siblings.0,
        vec![true, true],
        "the whole bundle should be inserted before any on_add fires, regardless of bundle order"
    );
}

#[test]
fn insert_fires_on_add() {
    let mut world = World::new();
    world.register_component_lifetimes::<Tracked>();
    world.insert_resource(TrackLog::default());

    let entity = world.spawn((Value(1),));
    world.insert(Tracked, entity);

    let log = world.get_resource::<TrackLog>().unwrap();
    assert_eq!(log.added, vec![(entity, true)]);
}

#[test]
fn remove_component_fires_on_remove_after_removal() {
    let mut world = World::new();
    world.register_component_lifetimes::<Tracked>();
    world.insert_resource(TrackLog::default());

    let entity = world.spawn((Tracked, Value(1)));
    world.remove_component::<Tracked>(entity);

    let log = world.get_resource::<TrackLog>().unwrap();
    assert_eq!(
        log.removed,
        vec![(entity, false)],
        "remove_component fires on_remove after the component is gone"
    );
}

#[test]
fn despawn_fires_on_remove_while_still_readable() {
    let mut world = World::new();
    world.register_component_lifetimes::<Tracked>();
    world.insert_resource(TrackLog::default());

    let entity = world.spawn((Tracked, Value(1)));
    world.despawn(entity);

    let log = world.get_resource::<TrackLog>().unwrap();
    assert_eq!(
        log.removed,
        vec![(entity, true)],
        "despawn fires on_remove while the component is still readable"
    );
}

#[test]
fn on_add_can_insert_a_companion_and_trigger_its_callbacks() {
    let mut world = World::new();
    world.register_component_lifetimes::<Body>();
    world.register_component_lifetimes::<Companion>();
    world.insert_resource(CompanionLog::default());

    let entity = world.spawn((Body, Value(1)));

    assert!(
        world
            .get_component_for_entity::<Companion>(entity)
            .is_some(),
        "Body::on_add should have inserted the Companion"
    );
    assert_eq!(value_of(&world, entity), Some(1));
    let log = world.get_resource::<CompanionLog>().unwrap();
    assert_eq!(
        log.adds, 1,
        "the nested insert should fire Companion::on_add exactly once"
    );
}

#[test]
fn suppressed_insert_does_not_fire_callbacks() {
    let mut world = World::new();
    world.register_component_lifetimes::<Quiet>();
    world.register_component_lifetimes::<Companion>();
    world.insert_resource(CompanionLog::default());

    let entity = world.spawn((Quiet, Value(1)));

    assert!(
        world
            .get_component_for_entity::<Companion>(entity)
            .is_some()
    );
    let log = world.get_resource::<CompanionLog>().unwrap();
    assert_eq!(
        log.adds, 0,
        "trigger_events = false must not fire the companion's on_add"
    );
}

#[test]
fn on_remove_removing_a_sibling_component_during_despawn() {
    let mut world = World::new();
    world.register_component_lifetimes::<Body>();
    world.register_component_lifetimes::<Companion>();
    world.insert_resource(CompanionLog::default());

    // Two entities share the archetype so the swap-remove paths engage.
    let doomed = world.spawn((Body, Value(1)));
    let survivor = world.spawn((Body, Value(2)));

    // Body::on_remove removes Companion mid-despawn, migrating the entity to
    // another archetype while despawn is in flight.
    world.despawn(doomed);

    assert_eq!(
        value_of(&world, doomed),
        None,
        "doomed entity should be gone"
    );
    assert_eq!(
        value_of(&world, survivor),
        Some(2),
        "the surviving entity's location must not be corrupted by the mid-despawn migration"
    );
    let log = world.get_resource::<CompanionLog>().unwrap();
    assert_eq!(log.adds, 2);
    assert_eq!(
        log.removes, 1,
        "despawn fires on_remove once for every component captured at despawn \
         start — the suppressed removal inside Body::on_remove must not add a \
         second firing"
    );
}

#[test]
fn on_remove_despawning_another_entity() {
    let mut world = World::new();
    world.register_component_lifetimes::<Reaper>();
    world.insert_resource(DespawnTarget::default());

    // The victim sits in row 0; the reaper is the archetype's last row, so
    // the victim's swap-removal moves the reaper mid-despawn.
    let victim = world.spawn((Reaper, Value(1)));
    let reaper = world.spawn((Reaper, Value(2)));
    world.get_resource_mut::<DespawnTarget>().unwrap().0 = Some(victim);

    world.despawn(reaper);

    assert_eq!(value_of(&world, victim), None, "victim should be despawned");
    assert_eq!(value_of(&world, reaper), None, "reaper should be despawned");
}

#[test]
fn noop_remove_of_missing_component_is_harmless() {
    let mut world = World::new();

    // `first` is not the archetype's last row: a buggy no-op removal would
    // clobber `last`'s location with `first`'s.
    let first = world.spawn((Value(1), Marker));
    let last = world.spawn((Value(2), Marker));

    world.remove_component::<Extra>(first);

    assert_eq!(value_of(&world, first), Some(1));
    assert_eq!(value_of(&world, last), Some(2));

    world.get_component_for_entity_mut::<Value>(last).unwrap().0 = 20;
    assert_eq!(
        value_of(&world, first),
        Some(1),
        "mutating the last entity must not write through a corrupted location"
    );
    assert_eq!(value_of(&world, last), Some(20));
}

#[test]
fn remove_component_keeps_swapped_entity_intact() {
    let mut world = World::new();

    let first = world.spawn((Value(1), Extra));
    let last = world.spawn((Value(2), Extra));

    world.remove_component::<Extra>(first);

    assert_eq!(value_of(&world, first), Some(1));
    assert!(world.get_component_for_entity::<Extra>(first).is_none());
    assert_eq!(value_of(&world, last), Some(2));
    assert!(world.get_component_for_entity::<Extra>(last).is_some());
}

#[test]
fn despawn_keeps_swapped_entity_intact() {
    let mut world = World::new();

    let first = world.spawn((Value(1), Extra));
    let last = world.spawn((Value(2), Extra));

    world.despawn(first);

    assert_eq!(value_of(&world, first), None);
    assert_eq!(value_of(&world, last), Some(2));
}

#[test]
fn callbacks_balance_across_mixed_operations() {
    let mut world = World::new();
    world.register_component_lifetimes::<Tracked>();
    world.insert_resource(TrackLog::default());

    let a = world.spawn((Tracked, Value(1)));
    let _b = world.spawn((Tracked, Value(2)));
    let c = world.spawn((Tracked, Value(3)));

    world.remove_component::<Tracked>(a);
    world.despawn(c);

    let log = world.get_resource::<TrackLog>().unwrap();
    assert_eq!(log.added.len(), 3);
    assert_eq!(
        log.removed.len(),
        2,
        "one remove_component + one despawn should each fire on_remove exactly once"
    );
}
