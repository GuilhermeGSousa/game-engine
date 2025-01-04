use std::any::TypeId;

pub use core_macros::Component;

pub struct ComponentId(TypeId);

pub trait Component: 'static {
    fn name() -> String;
}

pub trait ComponentStorage {
    fn as_any(&self) -> &dyn std::any::Any;
    fn as_any_mut(&mut self) -> &mut dyn std::any::Any;

    fn remove(&mut self, index: usize);
}

impl<T: Component> ComponentStorage for Vec<T> {
    fn remove(&mut self, index: usize) {
        self.remove(index);
    }

    fn as_any(&self) -> &dyn std::any::Any {
        self as &dyn std::any::Any
    }

    fn as_any_mut(&mut self) -> &mut dyn std::any::Any {
        self as &mut dyn std::any::Any
    }
}
