use std::any::TypeId;

pub use core_macros::Component;

use crate::system_input::SystemInput;

pub type ComponentId = TypeId;

pub trait Component: 'static + Sized {
    fn name() -> String;
}
