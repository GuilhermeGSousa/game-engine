use any_vec::any_value::AnyValueWrapper;

use crate::{
    component::{Component, ComponentId},
    entity::Entity,
    table::{MutableCellAccessor, Table, TableRowIndex},
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

    pub fn add_component<T: Component>(&mut self, value: T, current_tick: u32) {
        if let Some(column) = self.data_table.get_column_mut(ComponentId::of::<T>()) {
            column.push(AnyValueWrapper::<T>::new(value), current_tick);
        }
    }

    /// Overwrites the `T` already stored at `row`, dropping the previous value.
    pub fn replace_component<T: Component>(
        &mut self,
        value: T,
        current_tick: u32,
        row: TableRowIndex,
    ) {
        if let Some(column) = self.data_table.get_column_mut(ComponentId::of::<T>()) {
            column.replace(value, current_tick, row);
        }
    }

    pub fn add_entity(&mut self, entity: Entity) {
        self.data_table.add_entity(entity);
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

    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }

    pub(crate) unsafe fn get_component_unsafe<T: 'static>(&self, row: TableRowIndex) -> Option<&T> {
        unsafe {
            self.data_table
                .get_column(ComponentId::of::<T>())?
                .get_unsafe(row)
        }
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

    pub(crate) unsafe fn get_component_unsafe_mut<T: 'static>(
        &mut self,
        row: TableRowIndex,
    ) -> Option<MutableCellAccessor<'_, T>> {
        let column = self.data_table.get_column_mut(ComponentId::of::<T>())?;
        unsafe { column.get_unsafe_mut(row) }
    }

    pub fn entities(&self) -> &[Entity] {
        self.data_table.entities()
    }

    /// An empty table shaped like this archetype's, ready to be extended into a new archetype.
    pub(crate) fn clone_empty_table(&self) -> Table {
        self.data_table.clone_empty()
    }

    /// Moves the row at `row` into `dst`, returning the entity that occupied it.
    ///
    /// See [`Table::move_row_to`] for how `replaced` and missing columns are handled.
    pub(crate) fn move_row_to(
        &mut self,
        row: TableRowIndex,
        dst: &mut Archetype,
        replaced: &[ComponentId],
    ) -> Entity {
        self.data_table
            .move_row_to(row, &mut dst.data_table, replaced)
    }

    /// Swap-removes the row at `row`, dropping every component in it.
    pub(crate) fn drop_row(&mut self, row: TableRowIndex) {
        self.data_table.drop_row(row);
    }

    pub fn component_ids(&self) -> &[ComponentId] {
        &self.component_ids
    }
}
