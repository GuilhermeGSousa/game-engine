use std::any::TypeId;

pub use core_macros::Component;

#[derive(Eq, Hash, PartialEq)]
pub struct ComponentId(TypeId);

pub trait Component: 'static + Sized {
    fn name() -> String;
}
