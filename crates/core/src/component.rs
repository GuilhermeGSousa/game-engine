use std::any::TypeId;

pub use core_macros::Component;

pub type ComponentId = TypeId;

pub trait Component: 'static + Sized {
    fn name() -> String;
}

pub trait ComponentBorrow {
    fn is_mutable() -> bool;
}

impl<T: Component> ComponentBorrow for &T {
    fn is_mutable() -> bool {
        false
    }
}

impl<T: Component> ComponentBorrow for &mut T {
    fn is_mutable() -> bool {
        false
    }
}
