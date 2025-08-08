use any_vec::any_value::AnyValueWrapper;

use crate::{
    component::{Component, ComponentId},
    entity::Entity,
    table::{RemovedRow, Table, TableRowIndex},
};

pub struct Archetype {
    data_table: Table,
    component_ids: Vec<ComponentId>,
}

impl Archetype {
    pub fn new(data_table: Table, component_ids: Vec<ComponentId>) -> Archetype {
        Archetype {
            data_table,
            component_ids,
        }
    }

    pub fn add_component<T: Component>(
        &mut self,
        raw_value: AnyValueWrapper<T>,
        current_tick: u32,
    ) {
        if let Some(column) = self.data_table.get_column_mut(ComponentId::of::<T>()) {
            column.push(raw_value, current_tick);
        }
    }

    pub fn insert_component<T: Component>(
        &mut self,
        raw_value: AnyValueWrapper<T>,
        current_tick: u32,
        row: TableRowIndex,
    ) {
        if let Some(column) = self.data_table.get_column_mut(ComponentId::of::<T>()) {
            column.insert(raw_value, current_tick, row);
        }
    }

    pub fn add_entity(&mut self, entity: Entity) {
        self.data_table.add_entity(entity);
    }

    pub fn insert_entity(&mut self, entity: Entity, row: TableRowIndex) {
        self.data_table.insert_entity(row, entity);
    }

    pub fn contains(&self, component_id: ComponentId) -> bool {
        self.data_table.has_column(component_id)
    }

    pub fn contains_all(&self, component_ids: Vec<ComponentId>) -> bool {
        component_ids.iter().all(|id| self.contains(*id))
    }

    pub fn len(&self) -> usize {
        self.data_table.get_row_count()
    }

    pub unsafe fn get_component_unsafe<T: 'static>(&self, row: TableRowIndex) -> Option<&T> {
        self.data_table
            .get_column(ComponentId::of::<T>())?
            .get_unsafe(row)
    }

    pub fn was_entity_added(
        &self,
        component_id: ComponentId,
        row: TableRowIndex,
        current_tick: u32,
    ) -> bool {
        self.data_table.was_added(row, component_id, current_tick)
    }

    pub fn was_entity_changed(
        &self,
        component_id: ComponentId,
        row: TableRowIndex,
        current_tick: u32,
    ) -> bool {
        self.data_table.was_changed(row, component_id, current_tick)
    }

    pub unsafe fn get_component_unsafe_mut<T: 'static>(
        &mut self,
        row: TableRowIndex,
        current_tick: u32,
    ) -> Option<&mut T> {
        let column = self.data_table.get_column_mut(ComponentId::of::<T>())?;
        column.set_changed(row, current_tick);
        column.get_mut_unsafe(row)
    }

    pub fn entities(&self) -> &[Entity] {
        self.data_table.entities()
    }

    pub fn remove_swap(&mut self, row: TableRowIndex) -> RemovedRow {
        self.data_table.remove_swap(row)
    }

    pub fn component_ids(&self) -> &[ComponentId] {
        &self.component_ids
    }
}
