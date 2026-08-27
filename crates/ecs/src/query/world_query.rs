use crate::{Component, Entity, World};

use typle::typle;

pub trait WorldQuery {
    type State: Send + Sync + Sized;

    fn init_state(world: &mut World) -> Self::State;
}

impl<T: Component> WorldQuery for &T {
    type State = ();

    fn init_state(world: &mut World) -> Self::State {
        world.register_component::<T>();
    }
}

impl<T: Component> WorldQuery for &mut T {
    type State = ();

    fn init_state(world: &mut World) -> Self::State {
        world.register_component::<T>();
    }
}

impl<T: Component> WorldQuery for Option<&T> {
    type State = ();

    fn init_state(world: &mut World) -> Self::State {
        world.register_component::<T>();
    }
}

impl<T: Component> WorldQuery for Option<&mut T> {
    type State = ();

    fn init_state(world: &mut World) -> Self::State {
        world.register_component::<T>();
    }
}

impl WorldQuery for Entity {
    type State = ();

    fn init_state(_world: &mut World) -> Self::State {}
}

#[allow(unused_mut)]
#[allow(unused_variables)]
#[typle(Tuple for 0..=12)]
impl<T> WorldQuery for T
where
    T: Tuple,
    T<_>: WorldQuery,
{
    type State = typle_for!(i in .. => T<{i}>::State);

    fn init_state(world: &mut World) -> Self::State {
        typle_for!(i in .. => <T<{i}>>::init_state(world))
    }
}
