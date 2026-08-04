use std::fmt;

#[cfg(all(feature = "multithreaded", not(target_arch = "wasm32")))]
use ecs::system::executor::multi_thread::MultiThreadedExecutor;
#[cfg(not(all(feature = "multithreaded", not(target_arch = "wasm32"))))]
use ecs::system::executor::single_thread::SingleThreadedExecutor;
use ecs::{
    component::Component,
    extract::{MainWorld, ScratchMainWorld},
    resource::Resource,
    system::schedule::{CompiledSchedules, Schedules, UpdateGroup},
    world::World,
    IntoSystemConfig,
};
use facet::Facet;

/// Identifies a [`SubApp`] within an [`App`](crate::App).
///
/// Labels are declared by whichever crate owns the sub-app — for example the
/// render crate exports `RENDER_APP` — so the `app` crate stays unaware of what
/// sub-apps exist.
#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug)]
pub struct SubAppLabel(pub &'static str);

impl fmt::Display for SubAppLabel {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.0)
    }
}

/// Runs a sub-app's extract step, given the main world and the sub-app.
pub type ExtractFn = Box<dyn Fn(&mut World, &mut SubApp) + Send + Sync>;

/// A [`World`] together with the schedules that run against it.
///
/// An [`App`](crate::App) always has a main sub-app and may have any number of
/// additional ones (the render world being the motivating case). Each sub-app
/// owns its world exclusively; the only sanctioned way to move data between
/// them is the `Extract` schedule, driven by [`SubApp::extract`].
///
/// Sub-apps run sequentially, after the main sub-app, in the order they were
/// inserted.
pub struct SubApp {
    world: World,
    /// Present until [`compile_schedules`](SubApp::compile_schedules) consumes
    /// it; systems can only be added before that point.
    schedules: Option<Schedules>,
    compiled_schedules: Option<CompiledSchedules>,
    extract: Option<ExtractFn>,
}

impl SubApp {
    /// Creates an empty sub-app with the default extract behaviour.
    pub fn new() -> Self {
        let mut world = World::new();
        // Parked here so the extract swap never has to allocate a world.
        world.init_resource::<ScratchMainWorld>();

        Self {
            world,
            schedules: Some(Schedules::default()),
            compiled_schedules: None,
            extract: Some(Box::new(default_extract)),
        }
    }

    pub fn world(&self) -> &World {
        &self.world
    }

    pub fn world_mut(&mut self) -> &mut World {
        &mut self.world
    }

    /// Registers a system in the given [`UpdateGroup`].
    ///
    /// # Panics
    /// Panics if the schedules have already been compiled.
    pub fn add_system<M>(
        &mut self,
        update_group: UpdateGroup,
        system: impl IntoSystemConfig<M> + 'static,
    ) -> &mut Self {
        self.schedules
            .as_mut()
            .expect("Cannot add systems after the schedules have been compiled")
            .add_system(update_group, system);

        self
    }

    pub fn insert_resource<R: Resource>(&mut self, value: R) -> &mut Self {
        self.world.insert_resource(value);
        self
    }

    pub fn init_resource<R: Resource + Default>(&mut self) -> &mut Self {
        self.world.init_resource::<R>();
        self
    }

    pub fn get_resource<R: Resource>(&self) -> Option<&R> {
        self.world.get_resource()
    }

    pub fn get_resource_mut<R: Resource>(&mut self) -> Option<&mut R> {
        self.world.get_resource_mut()
    }

    pub fn remove_resource<R: Resource>(&mut self) -> Option<R> {
        self.world.remove_resource()
    }

    pub fn register_component_lifecycle<T: Component>(&mut self) -> &mut Self {
        self.world.register_component_lifetimes::<T>();
        self
    }

    pub fn register_reflection<T: Component + for<'a> Facet<'a>>(&mut self) -> &mut Self {
        self.world.register_reflection::<T>();
        self
    }

    /// Replaces the extract step for this sub-app.
    ///
    /// The default ([`default_extract`]) moves the main world in as a
    /// [`MainWorld`] resource and runs the `Extract` schedule; override it only
    /// when a sub-app needs a different bridge.
    pub fn set_extract(&mut self, extract: ExtractFn) -> &mut Self {
        self.extract = Some(extract);
        self
    }

    /// Compiles this sub-app's schedules, making it ready to run.
    pub fn compile_schedules(&mut self) {
        let schedules = self
            .schedules
            .take()
            .expect("Schedules have already been compiled");

        #[cfg(all(feature = "multithreaded", not(target_arch = "wasm32")))]
        let compiled_schedules = schedules.compile::<MultiThreadedExecutor>();
        #[cfg(not(all(feature = "multithreaded", not(target_arch = "wasm32"))))]
        let compiled_schedules = schedules.compile::<SingleThreadedExecutor>();

        self.compiled_schedules = Some(compiled_schedules);
    }

    /// Pulls data out of `main_world` and into this sub-app's world.
    pub fn extract(&mut self, main_world: &mut World) {
        // Taken out so the extract fn can borrow `self` mutably, then put back.
        let Some(extract) = self.extract.take() else {
            return;
        };
        (extract)(main_world, self);
        self.extract = Some(extract);
    }

    pub fn run_startup(&mut self) {
        let (schedules, world) = self.compiled_and_world();
        schedules.startup(world);
    }

    pub fn run_fixed_update(&mut self) {
        let (schedules, world) = self.compiled_and_world();
        schedules.fixed_update(world);
    }

    pub fn run_update(&mut self) {
        let (schedules, world) = self.compiled_and_world();
        schedules.update(world);
    }

    pub fn run_extract_schedule(&mut self) {
        let (schedules, world) = self.compiled_and_world();
        schedules.extract(world);
    }

    pub fn run_render(&mut self) {
        let (schedules, world) = self.compiled_and_world();
        schedules.render(world);
    }

    pub fn tick(&mut self) {
        self.world.tick();
    }

    /// Borrows the compiled schedules and the world at the same time.
    ///
    /// The schedules live beside the world rather than inside it precisely so
    /// this split borrow is possible — running a schedule needs `&mut World`,
    /// which schedules stored *as* a world resource could not provide without
    /// being removed and reinserted around every call.
    fn compiled_and_world(&mut self) -> (&mut CompiledSchedules, &mut World) {
        let schedules = self
            .compiled_schedules
            .as_mut()
            .expect("Schedules have not been compiled yet");

        (schedules, &mut self.world)
    }
}

impl Default for SubApp {
    fn default() -> Self {
        Self::new()
    }
}

/// Moves the main world into `sub_app` as a [`MainWorld`] resource, runs the
/// `Extract` schedule, and moves it back out.
///
/// The main world has to be *moved* rather than borrowed so that extract
/// systems can hold ordinary parameters on both worlds simultaneously; the
/// parked [`ScratchMainWorld`] fills the hole while the swap is in effect.
pub fn default_extract(main_world: &mut World, sub_app: &mut SubApp) {
    profiling::scope!("sub_app::extract");

    let scratch = sub_app
        .world
        .remove_resource::<ScratchMainWorld>()
        .unwrap_or_default();
    let moved_world = std::mem::replace(main_world, scratch.0);
    sub_app.world.insert_resource(MainWorld(moved_world));

    sub_app.run_extract_schedule();

    let MainWorld(moved_world) = sub_app
        .world
        .remove_resource::<MainWorld>()
        .expect("MainWorld was removed during the Extract schedule");
    let scratch = std::mem::replace(main_world, moved_world);
    sub_app.world.insert_resource(ScratchMainWorld(scratch));
}

/// The main sub-app plus every additional sub-app, in insertion order.
#[derive(Default)]
pub struct SubApps {
    pub main: SubApp,
    // A Vec rather than a HashMap: sub-apps run in a fixed order every frame,
    // and hash iteration order is not stable.
    sub_apps: Vec<(SubAppLabel, SubApp)>,
}

impl SubApps {
    pub fn new() -> Self {
        Self {
            main: SubApp::new(),
            sub_apps: Vec::new(),
        }
    }

    /// Inserts a sub-app, replacing any existing one with the same label.
    pub fn insert(&mut self, label: SubAppLabel, sub_app: SubApp) {
        match self.sub_apps.iter_mut().find(|(l, _)| *l == label) {
            Some(slot) => slot.1 = sub_app,
            None => self.sub_apps.push((label, sub_app)),
        }
    }

    pub fn get(&self, label: SubAppLabel) -> Option<&SubApp> {
        self.sub_apps
            .iter()
            .find(|(l, _)| *l == label)
            .map(|(_, sub_app)| sub_app)
    }

    pub fn get_mut(&mut self, label: SubAppLabel) -> Option<&mut SubApp> {
        self.sub_apps
            .iter_mut()
            .find(|(l, _)| *l == label)
            .map(|(_, sub_app)| sub_app)
    }

    /// Iterates the non-main sub-apps in insertion order.
    pub fn iter_mut(&mut self) -> impl Iterator<Item = &mut SubApp> {
        self.sub_apps.iter_mut().map(|(_, sub_app)| sub_app)
    }

    /// Runs `f` on the main sub-app and then on every other sub-app.
    pub fn for_each(&mut self, mut f: impl FnMut(&mut SubApp)) {
        f(&mut self.main);
        for (_, sub_app) in self.sub_apps.iter_mut() {
            f(sub_app);
        }
    }

    /// Extracts into every non-main sub-app and runs its per-frame schedules.
    pub fn update_sub_apps(&mut self) {
        if self.sub_apps.is_empty() {
            return;
        }

        profiling::scope!("sub_apps::update");

        for (_, sub_app) in self.sub_apps.iter_mut() {
            sub_app.extract(&mut self.main.world);
            sub_app.run_update();
            sub_app.run_render();
            sub_app.tick();
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use ecs::{
        extract::Extract,
        resource::{Res, ResMut},
    };

    #[derive(Resource)]
    struct Score(u32);

    #[derive(Resource)]
    struct ExtractedScore(u32);

    #[derive(Resource, Default)]
    struct RenderFrames(u32);

    const TEST_APP: SubAppLabel = SubAppLabel("TestApp");

    fn extract_score(score: Extract<Res<Score>>, mut out: ResMut<ExtractedScore>) {
        let score: &Score = &score;
        out.0 = score.0;
    }

    fn count_render_frames(mut frames: ResMut<RenderFrames>) {
        frames.0 += 1;
    }

    fn built_sub_apps() -> SubApps {
        let mut sub_apps = SubApps::new();
        sub_apps.main.insert_resource(Score(11));

        let mut sub_app = SubApp::new();
        sub_app
            .insert_resource(ExtractedScore(0))
            .init_resource::<RenderFrames>()
            .add_system(UpdateGroup::Extract, extract_score)
            .add_system(UpdateGroup::Render, count_render_frames);

        sub_apps.insert(TEST_APP, sub_app);
        sub_apps.for_each(|sub_app| sub_app.compile_schedules());
        sub_apps
    }

    #[test]
    fn extract_moves_data_into_the_sub_app() {
        let mut sub_apps = built_sub_apps();

        sub_apps.update_sub_apps();

        let sub_app = sub_apps.get(TEST_APP).unwrap();
        assert_eq!(sub_app.get_resource::<ExtractedScore>().unwrap().0, 11);
        assert_eq!(sub_app.get_resource::<RenderFrames>().unwrap().0, 1);
    }

    #[test]
    fn main_world_survives_repeated_extracts() {
        let mut sub_apps = built_sub_apps();

        for frame in 1..=5 {
            // A change in the main world must be visible to the next extract,
            // which only holds if the same world is swapped back each time.
            sub_apps.main.insert_resource(Score(frame));
            sub_apps.update_sub_apps();

            let sub_app = sub_apps.get(TEST_APP).unwrap();
            assert_eq!(sub_app.get_resource::<ExtractedScore>().unwrap().0, frame);
            assert_eq!(sub_app.get_resource::<RenderFrames>().unwrap().0, frame);
        }

        assert_eq!(sub_apps.main.get_resource::<Score>().unwrap().0, 5);
    }

    #[test]
    fn sub_app_world_is_separate_from_the_main_world() {
        let mut sub_apps = built_sub_apps();
        sub_apps.update_sub_apps();

        // Resources inserted into the sub-app must not leak into the main
        // world, and vice versa.
        assert!(sub_apps.main.get_resource::<ExtractedScore>().is_none());
        assert!(sub_apps
            .get(TEST_APP)
            .unwrap()
            .get_resource::<Score>()
            .is_none());
    }

    #[test]
    fn no_sub_apps_is_a_no_op() {
        let mut sub_apps = SubApps::new();
        sub_apps.for_each(|sub_app| sub_app.compile_schedules());

        sub_apps.update_sub_apps();

        assert!(sub_apps.get(TEST_APP).is_none());
    }

    #[test]
    fn inserting_the_same_label_replaces_the_sub_app() {
        let mut sub_apps = SubApps::new();
        sub_apps.insert(TEST_APP, SubApp::new());
        sub_apps.insert(TEST_APP, SubApp::new());

        assert_eq!(sub_apps.iter_mut().count(), 1);
    }
}
