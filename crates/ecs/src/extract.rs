//! Moving data from the main world into a sub-app's world.
//!
//! A sub-app owns its own [`World`], so its systems cannot reach the main
//! world's entities and resources through the usual parameters. The bridge is
//! the `Extract` schedule ([`UpdateGroup::Extract`](crate::system::schedule::UpdateGroup::Extract)):
//! while it runs, the main world is *moved* into the sub-app's world as a
//! [`MainWorld`] resource, so a single system can read the main world through
//! [`Extract`] and write the sub-app's world with ordinary parameters.
//!
//! ```ignore
//! fn extract_cameras(
//!     cameras: Extract<Query<(Entity, &Camera)>>,  // main world
//!     mut cmd: CommandQueue,                       // sub-app world
//! ) {
//!     for (entity, camera) in cameras.iter() { /* ... */ }
//! }
//! ```

use std::ops::{Deref, DerefMut};

use crate::{
    resource::Resource,
    system::{
        access::SystemAccess,
        input::{SystemInput, SystemInputData},
    },
    world::{UnsafeWorldCell, World},
};

/// The main world, temporarily moved into a sub-app's world for the duration of
/// the `Extract` schedule.
///
/// Inserted and removed by the sub-app's extract step; it is not present at any
/// other point in the frame, so only `Extract` systems may rely on it.
#[derive(Resource)]
pub struct MainWorld(pub World);

/// An empty world kept aside so the extract swap does not have to allocate.
///
/// The main world cannot simply be borrowed into the sub-app's resource map —
/// it has to be moved — which leaves a hole where the main world used to be.
/// This fills that hole for the duration of the swap and takes the main world's
/// place again once it is handed back.
#[derive(Resource, Default)]
pub struct ScratchMainWorld(pub World);

/// System parameter that reads `P` from the **main** world instead of the world
/// the system is running against.
///
/// Only valid inside the `Extract` schedule, which is the only point at which
/// the main world is available as a [`MainWorld`] resource.
///
/// # Read-only
///
/// `Extract` hands `P` a non-mutable [`UnsafeWorldCell`], so mutable parameters
/// (`Extract<ResMut<T>>`, `Extract<Query<&mut T>>`) panic on the
/// `UnsafeWorldCell::assert_mutable` debug assertion rather than quietly
/// aliasing the main world. Extraction is a copy *out* of the main world; a
/// system that needs to write back belongs in a main-world schedule.
pub struct Extract<'w, 's, P: SystemInput>(SystemInputData<'w, 's, P>);

impl<'w, 's, P: SystemInput> Extract<'w, 's, P> {
    pub fn into_inner(self) -> SystemInputData<'w, 's, P> {
        self.0
    }
}

impl<'w, 's, P: SystemInput> Deref for Extract<'w, 's, P> {
    type Target = SystemInputData<'w, 's, P>;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl<P: SystemInput> DerefMut for Extract<'_, '_, P> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.0
    }
}

impl<P: SystemInput + 'static> SystemInput for Extract<'_, '_, P> {
    type State = P::State;
    type Data<'world, 'state> = Extract<'world, 'state, P>;

    fn init_state() -> Self::State {
        P::init_state()
    }

    fn get_data<'world, 'state>(
        state: &'state mut Self::State,
        world: UnsafeWorldCell<'world>,
    ) -> Self::Data<'world, 'state> {
        let main_world = world
            .world()
            .get_resource::<MainWorld>()
            .expect("MainWorld not found — `Extract` is only valid in the `Extract` update group");

        Extract(P::get_data(state, main_world.0.as_unsafe_world_cell()))
    }

    fn fill_access(access: &mut SystemAccess) {
        // Deliberately *not* `P::fill_access`: those component and resource ids
        // describe the main world, and registering them here would make the
        // sub-app's scheduler order extract systems against unrelated sub-app
        // systems that happen to touch the same types. Reading `MainWorld` is
        // the only access this system really has on the world it runs in.
        access.read_resource::<MainWorld>();
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        component::Component,
        query::Query,
        resource::{Res, ResMut},
        system::{executor::single_thread::SingleThreadedExecutor, schedule::Schedule},
        Entity,
    };

    #[derive(Resource)]
    struct Score(u32);

    #[derive(Resource)]
    struct ExtractedScore(u32);

    #[derive(Component)]
    struct Position {
        x: f32,
    }

    #[derive(Resource, Default)]
    struct ExtractedPositions(Vec<f32>);

    /// Moves `main` into `sub` as `MainWorld`, runs `f`, then moves it back.
    fn with_main_world(main: &mut World, sub: &mut World, f: impl FnOnce(&mut World)) {
        let scratch = sub
            .remove_resource::<ScratchMainWorld>()
            .unwrap_or_default();
        let moved = std::mem::replace(main, scratch.0);
        sub.insert_resource(MainWorld(moved));

        f(sub);

        let MainWorld(moved) = sub.remove_resource::<MainWorld>().unwrap();
        let scratch = std::mem::replace(main, moved);
        sub.insert_resource(ScratchMainWorld(scratch));
    }

    fn extract_score(score: Extract<Res<Score>>, mut out: ResMut<ExtractedScore>) {
        // `Extract<Res<T>>` derefs through to `&T`.
        let score: &Score = &score;
        out.0 = score.0;
    }

    #[test]
    fn extract_reads_resource_from_main_world() {
        let mut main = World::new();
        main.insert_resource(Score(42));

        let mut sub = World::new();
        sub.insert_resource(ExtractedScore(0));

        let mut schedule = Schedule::new();
        schedule.add_system(extract_score);
        let mut schedule = schedule.compile::<SingleThreadedExecutor>();

        with_main_world(&mut main, &mut sub, |sub| schedule.run(sub));

        assert_eq!(sub.get_resource::<ExtractedScore>().unwrap().0, 42);
        // The main world must come back intact, not as the scratch world.
        assert_eq!(main.get_resource::<Score>().unwrap().0, 42);
    }

    fn extract_positions(
        positions: Extract<Query<(Entity, &Position)>>,
        mut out: ResMut<ExtractedPositions>,
    ) {
        out.0.clear();
        for (_, position) in positions.iter() {
            out.0.push(position.x);
        }
    }

    #[test]
    fn extract_queries_entities_from_main_world() {
        let mut main = World::new();
        main.spawn((Position { x: 1.0 },));
        main.spawn((Position { x: 2.0 },));

        let mut sub = World::new();
        sub.init_resource::<ExtractedPositions>();
        // An entity in the sub-app's own world must not be visible to the
        // extract query — that query runs against the main world.
        sub.spawn((Position { x: 99.0 },));

        let mut schedule = Schedule::new();
        schedule.add_system(extract_positions);
        let mut schedule = schedule.compile::<SingleThreadedExecutor>();

        with_main_world(&mut main, &mut sub, |sub| schedule.run(sub));

        let mut extracted = sub.get_resource::<ExtractedPositions>().unwrap().0.clone();
        extracted.sort_by(f32::total_cmp);
        assert_eq!(extracted, vec![1.0, 2.0]);
    }

    #[test]
    fn extract_round_trip_leaves_main_world_untouched() {
        let mut main = World::new();
        main.insert_resource(Score(7));
        let entity = main.spawn((Position { x: 3.0 },));

        let mut sub = World::new();
        sub.init_resource::<ExtractedPositions>();

        let mut schedule = Schedule::new();
        schedule.add_system(extract_positions);
        let mut schedule = schedule.compile::<SingleThreadedExecutor>();

        for _ in 0..3 {
            with_main_world(&mut main, &mut sub, |sub| schedule.run(sub));
        }

        assert_eq!(main.get_resource::<Score>().unwrap().0, 7);
        assert_eq!(
            main.get_component_for_entity::<Position>(entity).unwrap().x,
            3.0
        );
        // The scratch world is parked in the sub-app between extracts, ready to
        // be swapped in again without allocating.
        assert!(sub.get_resource::<ScratchMainWorld>().is_some());
        assert!(sub.get_resource::<MainWorld>().is_none());
    }

    fn extract_score_mutably(mut score: Extract<ResMut<Score>>) {
        let score: &mut Score = &mut score;
        score.0 += 1;
    }

    /// `Extract` is read-only: the main world is handed over as a non-mutable
    /// cell, so a mutable parameter trips `UnsafeWorldCell::assert_mutable`.
    #[test]
    #[cfg(debug_assertions)]
    #[should_panic(expected = "UnsafeWorldCell is not mutable")]
    fn extract_rejects_mutable_access() {
        let mut main = World::new();
        main.insert_resource(Score(1));

        let mut sub = World::new();

        let mut schedule = Schedule::new();
        schedule.add_system(extract_score_mutably);
        let mut schedule = schedule.compile::<SingleThreadedExecutor>();

        with_main_world(&mut main, &mut sub, |sub| schedule.run(sub));
    }

    #[test]
    #[should_panic(expected = "MainWorld not found")]
    fn extract_outside_extract_schedule_panics() {
        let mut sub = World::new();
        sub.insert_resource(ExtractedScore(0));

        let mut schedule = Schedule::new();
        schedule.add_system(extract_score);
        schedule.compile::<SingleThreadedExecutor>().run(&mut sub);
    }
}
