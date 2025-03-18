use crate::world::UnsafeWorldCell;
use typle::typle;

pub unsafe trait SystemInput {
    type Data<'world>;
    unsafe fn get_data<'world>(world: UnsafeWorldCell<'world>) -> Self::Data<'world>;
}

#[allow(unused_variables)]
#[typle(Tuple for 0..=12)]
unsafe impl<T> SystemInput for T
where
    T: Tuple,
    T<_>: SystemInput + 'static,
{
    type Data<'world> = typle_for!(i in .. => T<{i}>::Data<'world>);

    unsafe fn get_data<'world>(world: UnsafeWorldCell<'world>) -> Self::Data<'world> {
        typle_for!(i in .. => <T<{i}>>::get_data(world))
    }
}
