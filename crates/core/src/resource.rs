use std::any::{Any, TypeId};

pub type ResourceId = TypeId;

pub trait ToAny: 'static {
    fn as_any(&self) -> &dyn Any;
    fn as_any_mut(&mut self) -> &mut dyn Any;
}

impl<T: 'static> ToAny for T {
    fn as_any(&self) -> &dyn Any {
        self
    }

    fn as_any_mut(&mut self) -> &mut dyn Any {
        self
    }
}

pub trait Resource: 'static {
    fn name() -> String
    where
        Self: Sized;
}
