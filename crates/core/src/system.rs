use std::env::Args;

use crate::{bundle::ComponentBundle, world::World};

pub trait System {
    fn run(&mut self, world: &mut World);
}

pub struct FnSystem<F, Inputs> {
    func: F,
    _marker: std::marker::PhantomData<Inputs>,
}

impl<F, Inputs> FnSystem<F, Inputs> {
    pub fn new(func: F) -> Self {
        Self {
            func,
            _marker: std::marker::PhantomData,
        }
    }
}

impl<F, Inputs> System for FnSystem<F, Inputs>
where
    Inputs: ComponentBundle,
    F: FnMut(Inputs),
{
    fn run(&mut self, world: &mut World) {
        Inputs::get_type_ids();
    }
}

pub trait IntoSystem {
    fn into_system(self) -> BoxedSystem;
}

pub type BoxedSystem = Box<dyn System>;
