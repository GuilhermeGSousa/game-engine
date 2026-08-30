use std::collections::HashMap;

use ecs::{
    define_label,
    intern::Interned,
    system::schedule::{InternedScheduleLabel, ScheduleLabel, Schedules},
    Component, IntoSystemConfig, Resource, World,
};
use facet::Facet;

use crate::{compile, extractor::ExtractFn, schedule_groups::Startup};

define_label!(
    /// A strongly-typed identifier for a [`SubApp`], interned like a
    /// [`ScheduleLabel`](ecs::system::schedule::ScheduleLabel).
    SubAppLabel
);

/// An [`Interned`] [`SubAppLabel`], usable as a map key.
pub type InternedSubAppLabel = Interned<dyn SubAppLabel>;

macro_rules! define_sub_app_label {
    ($(#[$attr:meta])* $name:ident) => {
        $(#[$attr])*
        #[derive(Clone, PartialEq, Eq, Hash, Debug)]
        pub struct $name;

        impl SubAppLabel for $name {
            fn dyn_clone(&self) -> Box<dyn SubAppLabel> {
                Box::new(self.clone())
            }
        }
    };
}

define_sub_app_label!(
    /// Label of the built-in render [`SubApp`] created for every [`App`](crate::App).
    RenderApp
);

/// The set of worlds an [`App`](crate::App) drives each frame: the `main`
/// (simulation) world plus any number of secondary [`SubApp`]s keyed by
/// [`SubAppLabel`] (e.g. [`RenderApp`]).
///
/// Each frame [`update`](Self::update) runs the main world, then for every
/// sub-app runs its extract step (copying from the main world) followed by its
/// own update schedule.
pub struct SubApps {
    main: SubApp,
    sub_apps: HashMap<InternedSubAppLabel, SubApp>,
}

impl Default for SubApps {
    fn default() -> Self {
        let mut sub_apps = HashMap::new();
        sub_apps.insert(RenderApp.intern(), SubApp::default());
        Self {
            main: SubApp::default(),
            sub_apps,
        }
    }
}

impl SubApps {
    pub fn main(&self) -> &SubApp {
        &self.main
    }

    pub fn main_mut(&mut self) -> &mut SubApp {
        &mut self.main
    }

    /// Returns the sub-app registered under `label`, panicking if there is none.
    pub fn sub_app(&self, label: impl SubAppLabel) -> &SubApp {
        let label = label.intern();
        self.sub_apps
            .get(&label)
            .unwrap_or_else(|| panic!("sub-app {label:?} is not registered"))
    }

    /// Returns the sub-app registered under `label`, panicking if there is none.
    pub fn sub_app_mut(&mut self, label: impl SubAppLabel) -> &mut SubApp {
        let label = label.intern();
        self.sub_apps
            .get_mut(&label)
            .unwrap_or_else(|| panic!("sub-app {label:?} is not registered"))
    }

    pub fn get_sub_app(&self, label: impl SubAppLabel) -> Option<&SubApp> {
        self.sub_apps.get(&label.intern())
    }

    pub fn get_sub_app_mut(&mut self, label: impl SubAppLabel) -> Option<&mut SubApp> {
        self.sub_apps.get_mut(&label.intern())
    }

    pub fn insert_sub_app(&mut self, label: impl SubAppLabel, sub_app: SubApp) {
        self.sub_apps.insert(label.intern(), sub_app);
    }

    pub fn remove_sub_app(&mut self, label: impl SubAppLabel) -> Option<SubApp> {
        self.sub_apps.remove(&label.intern())
    }

    pub fn iter_sub_apps_mut(&mut self) -> impl Iterator<Item = &mut SubApp> {
        self.sub_apps.values_mut()
    }

    pub fn startup(&mut self) {
        self.main.world.run_schedule(Startup);
        for sub_app in self.sub_apps.values_mut() {
            sub_app.world.run_schedule(Startup);
        }
    }

    pub fn update(&mut self) {
        self.main.update();

        for sub_app in self.sub_apps.values_mut() {
            sub_app.run_extract(&mut self.main.world);
            sub_app.update();
        }
    }
}

pub struct SubApp {
    world: World,
    update_schedule: Option<InternedScheduleLabel>,
    /// Copies data out of the main world before this sub-app's schedules run.
    /// Called as `extract(main_world, &mut self.world)`.
    extract: Option<ExtractFn>,
}

impl Default for SubApp {
    fn default() -> Self {
        let mut world = World::new();
        world.init_resource::<Schedules>();
        Self {
            world,
            update_schedule: None,
            extract: None,
        }
    }
}

impl SubApp {
    pub fn insert_resource<R: Resource>(&mut self, value: R) -> &mut Self {
        self.world.insert_resource(value);
        self
    }
    pub fn remove_resource<R: Resource>(&mut self) -> Option<R> {
        self.world.remove_resource::<R>()
    }

    pub fn get_resource<R: Resource>(&self) -> Option<&R> {
        self.world.get_resource::<R>()
    }

    pub fn get_resource_mut<R: Resource>(&mut self) -> Option<&mut R> {
        self.world.get_resource_mut::<R>()
    }

    pub fn register_component_lifetimes<T: Component>(&mut self) -> &mut Self {
        self.world.register_component_lifetimes::<T>();
        self
    }

    pub fn register_reflection<T: Component + for<'a> Facet<'a>>(&mut self) -> &mut Self {
        self.world.register_reflection::<T>();
        self
    }

    pub fn add_system<M>(
        &mut self,
        update_group: impl ScheduleLabel,
        system: impl IntoSystemConfig<M> + 'static,
    ) -> &mut Self {
        self.get_resource_mut::<Schedules>()
            .expect("Schedules resource not found!")
            .add_system(update_group, system);
        self
    }

    pub fn update(&mut self) {
        if let Some(label) = self.update_schedule {
            self.world.run_schedule(label);
        }
    }

    pub fn set_update_schedule(&mut self, label: impl ScheduleLabel) -> &mut Self {
        self.update_schedule = Some(label.intern());
        self
    }

    /// Sets the closure that extracts data from the main world into this
    /// sub-app's world each frame, before its schedules run.
    pub fn set_extract(&mut self, extract: ExtractFn) -> &mut Self {
        self.extract = Some(extract);
        self
    }

    /// Runs this sub-app's extract closure (if any) against `main_world`.
    pub fn run_extract(&mut self, main_world: &mut World) {
        let Some(mut extract) = self.extract.take() else {
            return;
        };
        extract(main_world, &mut self.world);
        self.extract = Some(extract);
    }

    /// Compiles this sub-app's [`Schedules`] into the world's [`CompiledSchedules`].
    ///
    /// [`CompiledSchedules`]: ecs::system::schedule::CompiledSchedules
    pub fn compile_schedules(&mut self) {
        let schedules = self
            .world
            .remove_resource::<Schedules>()
            .expect("Schedules resource not found on sub-app!");
        let compiled = compile(schedules, &mut self.world);
        self.world.insert_resource(compiled);
    }

    pub fn world(&self) -> &World {
        &self.world
    }

    pub fn world_mut(&mut self) -> &mut World {
        &mut self.world
    }
}
