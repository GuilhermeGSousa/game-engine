use crate::{
    archetype::Archetype,
    component::{Component, ComponentId, Tick},
    entity::Entity,
    table::{Table, TableRow, TableRowIndex},
};
use any_vec::any_value::AnyValueWrapper;
use std::any::TypeId;

use typle::typle;

/// A destination a [`ComponentBundle`] can write its components onto, one component at a time.
///
/// A bundle never knows in advance whether it's landing on a freshly-pushed archetype row, an
/// existing archetype row, or an in-flight [`TableRow`] being merged into another archetype —
/// this is the seam that lets bundle writing stay agnostic to all three.
#[doc(hidden)]
pub trait ComponentSink {
    fn write<T: Component>(&mut self, value: AnyValueWrapper<T>, current_tick: u32);
}

impl ComponentSink for TableRow {
    fn write<T: Component>(&mut self, value: AnyValueWrapper<T>, current_tick: u32) {
        self.insert(value, Tick::new(current_tick));
    }
}

/// Writes onto a freshly-pushed row at the end of an archetype's columns.
struct AppendRow<'a>(&'a mut Archetype);

impl ComponentSink for AppendRow<'_> {
    fn write<T: Component>(&mut self, value: AnyValueWrapper<T>, current_tick: u32) {
        self.0.add_component(value, current_tick);
    }
}

pub trait ComponentBundle: Send + Sync + Sized {
    fn get_component_ids() -> Vec<ComponentId>;

    fn generate_empty_table() -> Table;

    /// Writes every component in the bundle onto `sink`, one at a time.
    #[doc(hidden)]
    fn write_into<S: ComponentSink>(self, sink: &mut S, current_tick: u32);
}

/// Appends a new row to `archetype` for `entity` and writes `bundle` onto it.
pub(crate) fn add_row_to_archetype<T: ComponentBundle>(
    bundle: T,
    archetype: &mut Archetype,
    entity: Entity,
    current_tick: u32,
) -> TableRowIndex {
    let table_row = TableRowIndex::new(archetype.len());
    archetype.add_entity(entity);
    bundle.write_into(&mut AppendRow(archetype), current_tick);
    table_row
}

impl<T> ComponentBundle for T
where
    T: Component,
{
    fn get_component_ids() -> Vec<ComponentId> {
        vec![TypeId::of::<T>()]
    }

    fn generate_empty_table() -> Table {
        let mut table: Table = Table::new();
        table.add_column::<T>();
        table
    }

    fn write_into<S: ComponentSink>(self, sink: &mut S, current_tick: u32) {
        sink.write(AnyValueWrapper::<T>::new(self), current_tick);
    }
}

#[allow(unused_mut)]
#[allow(unused_variables)]
#[typle(Tuple for 0..=12)]
impl<T> ComponentBundle for T
where
    T: Tuple,
    T<_>: ComponentBundle,
{
    fn get_component_ids() -> Vec<ComponentId> {
        let mut type_ids = Vec::new();
        for typle_index!(i) in 0..T::LEN {
            type_ids.extend(<T<{ i }>>::get_component_ids());
        }
        type_ids.sort();
        type_ids
    }

    fn write_into<S: ComponentSink>(self, sink: &mut S, current_tick: u32) {
        for typle_index!(i) in 0..T::LEN {
            self[[i]].write_into(sink, current_tick);
        }
    }

    #[allow(clippy::let_and_return)]
    fn generate_empty_table() -> Table {
        let mut table = Table::new();
        for typle_index!(i) in 0..T::LEN {
            table.merge(<T<{ i }>>::generate_empty_table());
        }
        table
    }
}
