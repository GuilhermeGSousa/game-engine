use derive_more::{Deref, DerefMut};
use ecs::World;

pub(crate) type ExtractorFn = Box<dyn FnOnce(&mut World, &mut World)>;

#[derive(Deref, DerefMut)]
pub(crate) struct MainWorld(pub(crate) World);

impl MainWorld {
    pub(crate) fn new(world: World) -> Self {
        Self(world)
    }
}
