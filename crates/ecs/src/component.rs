use std::{
    any::TypeId,
    ops::{Deref, DerefMut},
};

pub use ecs_macros::Component;

pub type ComponentId = TypeId;

pub trait Component: 'static + Sized {
    fn name() -> String;
}

pub(crate) struct Tick(u32);

impl Tick {
    pub fn new(tick: u32) -> Self {
        Self(tick)
    }

    pub fn set(&mut self, tick: u32) {
        self.0 = tick;
    }
}

impl Deref for Tick {
    type Target = u32;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl DerefMut for Tick {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.0
    }
}
