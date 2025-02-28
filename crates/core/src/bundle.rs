use crate::component::Component;
use crate::table::Table;
use any_vec::any_value::AnyValueTypelessRaw;
use std::any::TypeId;
use std::mem;
use std::ptr::NonNull;
use typle::typle;

pub trait ComponentBundle {
    fn get_type_ids() -> Vec<TypeId>;

    fn get_components<F: FnMut(TypeId, AnyValueTypelessRaw)>(self, f: F);

    fn generate_empty_table() -> Table;
}

#[typle(Tuple for 0..=12)]
impl<T> ComponentBundle for T
where
    T: Tuple,
    T<_>: Component,
{
    fn get_type_ids() -> Vec<TypeId> {
        let mut type_ids = Vec::new();
        for typle_index!(i) in 0..T::LEN {
            type_ids.push(TypeId::of::<T<{ i }>>());
        }
        type_ids.sort();
        type_ids
    }

    fn get_components<F: FnMut(TypeId, AnyValueTypelessRaw)>(self, mut f: F) {
        for typle_index!(i) in 0..T::LEN {
            let a = self[[i]];

            let raw_val = unsafe {
                AnyValueTypelessRaw::new(NonNull::from(&a).cast::<u8>(), mem::size_of::<T<{ i }>>())
            };

            f(TypeId::of::<T<{ i }>>(), raw_val);
        }
    }

    fn generate_empty_table() -> Table {
        let mut table: Table = Table::new();
        for typle_index!(i) in 0..T::LEN {
            table.add_column::<T<{ i }>>();
        }
        table
    }
}
