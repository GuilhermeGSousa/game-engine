use std::any::TypeId;

pub use ecs_macros::Component;

pub type ComponentId = TypeId;

pub trait Component: 'static + Sized {
    fn name() -> String;
}
