use std::mem;

use derive_more::Deref;
use ecs::{Resource, World};

#[derive(Deref, Default, Resource)]
pub(crate) struct ScratchMainWorld(pub(crate) World);

#[derive(Deref, Resource)]
pub(crate) struct MainWorld(pub(crate) World);

impl MainWorld {
    pub(crate) fn new(world: World) -> Self {
        Self(world)
    }
}

pub(crate) fn extract(main: &mut World, other: &mut World) {
    let scratch = main
        .remove_resource::<ScratchMainWorld>()
        .unwrap_or_default();

    let moved_main = mem::replace(main, scratch.0);

    other.insert_resource(MainWorld::new(moved_main));

    // Do some extracting

    let moved_main = other.remove_resource::<MainWorld>().unwrap();
    let scratch_world = mem::replace(main, moved_main.0);
    main.insert_resource(ScratchMainWorld(scratch_world));
}
