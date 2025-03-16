use std::any::TypeId;

pub use core_macros::Component;

use crate::archetype::Archetype;

pub type ComponentId = TypeId;

pub trait Component: 'static + Sized {
    fn name() -> String;
}
