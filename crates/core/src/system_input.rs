use crate::world::UnsafeWorldCell;
use typle::{typle, typle_for};

pub unsafe trait SystemInput {
    type Data<'world>;
    unsafe fn get_data<'world>(world: UnsafeWorldCell<'world>) -> Self::Data<'world>;
}
