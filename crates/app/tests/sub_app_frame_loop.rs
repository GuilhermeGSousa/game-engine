//! End-to-end coverage of the two-world frame loop: an [`App`] with a real
//! sub-app, driven through `finish_plugin_build` and several `update()` calls.
//!
//! This is the closest thing to running the engine that works without a GPU,
//! and it pins the behaviour that the render sub-app depends on: the extract
//! step runs once per frame, in order, and the main world survives being moved
//! into the sub-app and back every frame.

use app::{
    plugins::{Plugin, TimePlugin},
    sub_app::{SubApp, SubAppLabel},
    App,
};
use ecs::{
    component::Component,
    extract::Extract,
    query::Query,
    resource::{Res, ResMut, Resource},
    system::schedule::UpdateGroup,
};

const TEST_APP: SubAppLabel = SubAppLabel("TestApp");

/// Lives in the main world.
#[derive(Component)]
struct Marker(u32);

/// Lives in the main world.
#[derive(Resource)]
struct FrameCounter(u32);

/// Lives in the sub-app's world: a running log of what ran, and when.
#[derive(Resource, Default)]
struct Trace {
    startups: u32,
    extracts: u32,
    renders: u32,
    /// Values seen in the main world by the most recent extract.
    last_seen_markers: Vec<u32>,
    /// `FrameCounter` as observed by the most recent extract.
    last_seen_frame: u32,
}

fn bump_frame_counter(mut counter: ResMut<FrameCounter>) {
    counter.0 += 1;
}

fn record_startup(mut trace: ResMut<Trace>) {
    trace.startups += 1;
}

fn extract_main_world(
    markers: Extract<Query<&Marker>>,
    counter: Extract<Res<FrameCounter>>,
    mut trace: ResMut<Trace>,
) {
    let counter: &FrameCounter = &counter;

    trace.extracts += 1;
    trace.last_seen_frame = counter.0;
    trace.last_seen_markers = markers.iter().map(|marker| marker.0).collect();
    trace.last_seen_markers.sort_unstable();
}

fn record_render(mut trace: ResMut<Trace>) {
    trace.renders += 1;
}

struct TestSubAppPlugin;

impl Plugin for TestSubAppPlugin {
    fn build(&self, app: &mut App) {
        app.insert_resource(FrameCounter(0));
        app.add_system(UpdateGroup::Update, bump_frame_counter);

        let mut sub_app = SubApp::new();
        sub_app
            .init_resource::<Trace>()
            .add_system(UpdateGroup::Startup, record_startup)
            .add_system(UpdateGroup::Extract, extract_main_world)
            .add_system(UpdateGroup::Render, record_render);

        app.insert_sub_app(TEST_APP, sub_app);
    }
}

fn trace(app: &App) -> &Trace {
    app.get_sub_app(TEST_APP)
        .expect("test sub-app")
        .get_resource::<Trace>()
        .expect("Trace resource")
}

fn built_app() -> App {
    let mut app = App::new();
    app.register_plugin(TimePlugin)
        .register_plugin(TestSubAppPlugin);
    app.finish_plugin_build();
    app
}

#[test]
fn sub_app_startup_runs_once() {
    let mut app = built_app();
    assert_eq!(trace(&app).startups, 1);

    for _ in 0..3 {
        app.update();
    }

    assert_eq!(trace(&app).startups, 1);
}

#[test]
fn extract_and_render_run_once_per_frame() {
    let mut app = built_app();

    // Startup must not have run the per-frame schedules.
    assert_eq!(trace(&app).extracts, 0);
    assert_eq!(trace(&app).renders, 0);

    for frame in 1..=5 {
        app.update();

        let trace = trace(&app);
        assert_eq!(trace.extracts, frame);
        assert_eq!(trace.renders, frame);
    }
}

#[test]
fn extract_sees_main_world_state_from_the_same_frame() {
    let mut app = built_app();
    app.world_mut().spawn((Marker(1),));
    app.world_mut().spawn((Marker(2),));

    app.update();

    assert_eq!(trace(&app).last_seen_markers, vec![1, 2]);
    // `bump_frame_counter` runs in Update, before the sub-app extracts, so the
    // extract must observe this frame's value rather than last frame's.
    assert_eq!(trace(&app).last_seen_frame, 1);

    app.world_mut().spawn((Marker(3),));
    app.update();

    assert_eq!(trace(&app).last_seen_markers, vec![1, 2, 3]);
    assert_eq!(trace(&app).last_seen_frame, 2);
}

#[test]
fn main_world_is_intact_after_many_frames() {
    let mut app = built_app();
    let entity = app.world_mut().spawn((Marker(7),));

    for _ in 0..10 {
        app.update();
    }

    // The main world is moved into the sub-app and back on every frame; both
    // its entities and its resources must come through unchanged.
    assert_eq!(
        app.world()
            .get_component_for_entity::<Marker>(entity)
            .expect("marker component")
            .0,
        7
    );
    assert_eq!(app.get_resource::<FrameCounter>().unwrap().0, 10);
    assert_eq!(trace(&app).last_seen_frame, 10);
}

#[test]
fn sub_app_world_is_isolated_from_the_main_world() {
    let mut app = built_app();
    app.world_mut().spawn((Marker(1),));
    app.update();

    // The sub-app's resources must not appear in the main world.
    assert!(app.get_resource::<Trace>().is_none());
    // Nor the main world's in the sub-app's.
    let sub_app = app.get_sub_app(TEST_APP).unwrap();
    assert!(sub_app.get_resource::<FrameCounter>().is_none());
    // The main world's entities stay in the main world; extract copies values
    // across rather than moving entities.
    let sub_markers = Query::<&Marker>::new(sub_app.world().as_unsafe_world_cell());
    assert_eq!(sub_markers.iter().count(), 0);
}

#[test]
fn app_with_no_sub_apps_still_updates() {
    let mut app = App::new();
    app.register_plugin(TimePlugin);
    app.insert_resource(FrameCounter(0));
    app.add_system(UpdateGroup::Update, bump_frame_counter);
    app.finish_plugin_build();

    for _ in 0..3 {
        app.update();
    }

    assert_eq!(app.get_resource::<FrameCounter>().unwrap().0, 3);
}
