use crate::{
    archetype::Archetype,
    component::{Component, ComponentId},
    entity::Entity,
    table::{Table, TableRowIndex},
};
use any_vec::any_value::AnyValueWrapper;
use std::any::TypeId;

use typle::typle;

pub trait ComponentBundle {
    fn get_component_ids() -> Vec<ComponentId>;

    fn add_row_to_archetype(
        self,
        archetype: &mut Archetype,
        entity: Entity,
        current_tick: u32,
    ) -> TableRowIndex;

    fn insert_to_archetype(
        self,
        archetype: &mut Archetype,
        current_tick: u32,
        entity: Entity,
        row: TableRowIndex,
    );

    fn generate_empty_table() -> Table;
}

impl<T> ComponentBundle for T
where
    T: Component,
{
    fn get_component_ids() -> Vec<ComponentId> {
        let mut type_ids = Vec::new();
        type_ids.push(TypeId::of::<T>());
        type_ids
    }

    fn add_row_to_archetype(
        self,
        archetype: &mut Archetype,
        entity: Entity,
        current_tick: u32,
    ) -> TableRowIndex {
        let table_row = TableRowIndex::new(archetype.len() as u32);
        archetype.add_entity(entity);
        archetype.add_component(AnyValueWrapper::<T>::new(self), current_tick);
        table_row
    }

    fn insert_to_archetype(
        self,
        archetype: &mut Archetype,
        current_tick: u32,
        entity: Entity,
        row: TableRowIndex,
    ) {
        archetype.insert_entity(entity, row);
        archetype.insert_component(AnyValueWrapper::<T>::new(self), current_tick, row);
    }

    fn generate_empty_table() -> Table {
        let mut table: Table = Table::new();
        table.add_column::<T>();
        table
    }
}

#[allow(unused_mut)]
#[allow(unused_variables)]
#[typle(Tuple for 0..=12)]
impl<T> ComponentBundle for T
where
    T: Tuple,
    T<_>: Component,
{
    fn get_component_ids() -> Vec<TypeId> {
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

    fn add_row_to_archetype(
        self,
        archetype: &mut Archetype,
        entity: Entity,
        current_tick: u32,
    ) -> TableRowIndex {
        let table_row = TableRowIndex::new(archetype.len() as u32);
        archetype.add_entity(entity);
        for typle_index!(i) in 0..T::LEN {
            archetype.add_component(AnyValueWrapper::<T<{ i }>>::new(self[[i]]), current_tick);
        }
        table_row
    }

    fn insert_to_archetype(
        self,
        archetype: &mut Archetype,
        current_tick: u32,
        entity: Entity,
        row: TableRowIndex,
    ) {
        archetype.insert_entity(entity, row);
        for typle_index!(i) in 0..T::LEN {
            archetype.insert_component(
                AnyValueWrapper::<T<{ i }>>::new(self[[i]]),
                current_tick,
                row,
            );
        }
    }
}
