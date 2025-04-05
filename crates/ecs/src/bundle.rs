use crate::archetype::Archetype;
use crate::component::Component;
use crate::table::Table;
use any_vec::any_value::AnyValueWrapper;
use std::any::TypeId;

use typle::typle;

pub trait ComponentBundle {
    fn get_type_ids() -> Vec<TypeId>;

    fn add_to_archetype(self, archetype: &mut Archetype);

    fn generate_empty_table() -> Table;
}

#[allow(unused_mut)]
#[allow(unused_variables)]
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

    fn generate_empty_table() -> Table {
        let mut table: Table = Table::new();
        for typle_index!(i) in 0..T::LEN {
            table.add_column::<T<{ i }>>();
        }
        table
    }

    fn add_to_archetype(self, archetype: &mut Archetype) {
        for typle_index!(i) in 0..T::LEN {
            archetype.add_component(AnyValueWrapper::<T<{ i }>>::new(self[[i]]));
        }
    }
}
