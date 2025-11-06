use std::{
    any::TypeId,
    ops::{Deref, DerefMut},
};

pub mod bundle;

pub use ecs_macros::Component;

use crate::{entity::Entity, world::RestrictedWorld};

pub type ComponentId = TypeId;

pub type ComponentLifecycleCallback = for<'w> fn(RestrictedWorld<'w>, ComponentLifecycleContext);

pub struct ComponentLifecycleContext {
    pub entity: Entity,
}

pub trait Component: 'static {
    fn name() -> &'static str;

    fn on_add() -> Option<ComponentLifecycleCallback> {
        None
    }

    fn on_remove() -> Option<ComponentLifecycleCallback> {
        None
    }
}

// TODO: Implement add lifecycle
#[allow(dead_code)]
pub(crate) struct ComponentLifecycleCallbacks {
    pub(crate) on_add: Option<ComponentLifecycleCallback>,
    pub(crate) on_remove: Option<ComponentLifecycleCallback>,
}

impl ComponentLifecycleCallbacks {
    pub(crate) fn from_component<T: Component>() -> Self {
        Self {
            on_add: T::on_add(),
            on_remove: T::on_remove(),
        }
    }
}

#[derive(Clone, Copy, PartialEq, Eq)]
pub struct Tick(u32);

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
