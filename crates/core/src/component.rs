use std::any::TypeId;

pub use core_macros::Component;

pub struct ComponentId(TypeId);

pub trait Component: 'static + Sized {
    fn name() -> String;
}
