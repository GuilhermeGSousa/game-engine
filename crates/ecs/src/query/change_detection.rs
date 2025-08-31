use std::ops::{Deref, DerefMut};

use crate::component::{Component, Tick};

pub struct Mut<'w, T: Component> {
    data: &'w mut T,
    changed_tick: &'w mut Tick,
    current_tick: Tick,
    was_changed: bool,
}

impl<'w, T> Mut<'w, T>
where
    T: Component,
{
    pub fn new(data: &'w mut T, changed_tick: &'w mut Tick, current_tick: Tick) -> Self {
        Self {
            data,
            changed_tick,
            current_tick,
            was_changed: false,
        }
    }
}

impl<'w, T> Deref for Mut<'w, T>
where
    T: Component,
{
    type Target = &'w mut T;

    fn deref(&self) -> &Self::Target {
        &self.data
    }
}

impl<'w, T> DerefMut for Mut<'w, T>
where
    T: Component,
{
    fn deref_mut(&mut self) -> &mut Self::Target {
        if !self.was_changed {
            self.was_changed = true;
            *self.changed_tick = self.current_tick;
        }
        &mut self.data
    }
}

pub trait DetectChanges {
    fn has_changed(&self) -> bool;
}
