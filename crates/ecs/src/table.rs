use std::{
    any::TypeId,
    collections::HashMap,
    ops::{Deref, DerefMut},
};

use any_vec::{any_value::AnyValueWrapper, AnyVec};

use crate::{
    component::{Component, ComponentId, Tick},
    entity::Entity,
};

pub struct Column {
    data: AnyVec,
    added_ticks: Vec<Tick>,
    changed_ticks: Vec<Tick>,
}

impl Column {
    pub fn new<T: Component>() -> Self {
        Self {
            data: AnyVec::new::<T>(),
            added_ticks: Vec::new(),
            changed_ticks: Vec::new(),
        }
    }

    pub fn push<T>(&mut self, raw_value: AnyValueWrapper<T>, tick: u32) {
        self.data.push(raw_value);
        self.added_ticks.push(Tick::new(tick));
        self.changed_ticks.push(Tick::new(0));
    }

    pub fn insert<T>(&mut self, raw_value: AnyValueWrapper<T>, tick: u32, row: TableRowIndex) {
        let index = *row;
        self.data.insert(index, raw_value);
        self.added_ticks.insert(index, Tick::new(tick));
        self.changed_ticks.insert(index, Tick::new(0));
    }

    pub fn len(&self) -> usize {
        self.data.len()
    }

    pub fn is_empty(&self) -> bool {
        self.data.is_empty()
    }

    pub(crate) unsafe fn get_unsafe<T: 'static>(&self, row: TableRowIndex) -> Option<&T> {
        self.data.get_unchecked(*row).downcast_ref()
    }

    pub(crate) unsafe fn get_unsafe_mut<T: 'static>(
        &mut self,
        row: TableRowIndex,
    ) -> Option<MutableCellAccessor<'_, T>> {
        self.data
            .get_unchecked_mut(*row)
            .downcast_mut()
            .map(|data| MutableCellAccessor {
                data,
                changed_tick: &mut self.changed_ticks[*row],
            })
    }

    pub fn set_changed(&mut self, row: TableRowIndex, current_tick: u32) {
        self.changed_ticks[*row].set(current_tick);
    }

    pub fn clone_empty_data(&self) -> AnyVec {
        self.data.clone_empty()
    }
}

pub struct Table {
    columns: HashMap<ComponentId, Column>,
    entities: Vec<Entity>,
}

impl Table {
    pub fn new() -> Self {
        Self {
            columns: HashMap::new(),
            entities: Vec::new(),
        }
    }

    pub(crate) fn from_row(mut removed_row: TableRow) -> Self {
        let mut columns = HashMap::new();

        removed_row.data.drain().for_each(|(key, value)| {
            columns.insert(
                key,
                Column {
                    data: value.component_data,
                    added_ticks: vec![value.added_tick],
                    changed_ticks: vec![value.changed_tick],
                },
            );
        });

        Self {
            columns,
            entities: vec![removed_row.entity],
        }
    }

    pub fn add_column<T: Component>(&mut self) {
        self.columns
            .insert(ComponentId::of::<T>(), Column::new::<T>());
    }

    pub fn add_row(&mut self, mut row: TableRow) {
        self.columns.iter_mut().for_each(|(key, value)| {
            if let Some(added_col) = row.data.get_mut(key) {
                value.data.push(added_col.component_data.pop().unwrap());
                value.added_ticks.push(added_col.added_tick);
                value.changed_ticks.push(added_col.changed_tick);
            }
        });

        self.entities.push(row.entity);
    }

    pub fn add_entity(&mut self, entity: Entity) {
        self.entities.push(entity);
    }

    pub fn insert_entity(&mut self, row: TableRowIndex, entity: Entity) {
        self.entities.insert(*row, entity);
    }

    pub fn get_row_count(&self) -> usize {
        self.columns
            .values()
            .next()
            .map(|column| column.len())
            .unwrap_or(0)
    }

    pub fn has_column(&self, type_id: ComponentId) -> bool {
        self.columns.contains_key(&type_id)
    }

    pub fn get_column(&self, type_id: ComponentId) -> Option<&Column> {
        self.columns.get(&type_id)
    }

    pub fn get_column_mut(&mut self, type_id: ComponentId) -> Option<&mut Column> {
        self.columns.get_mut(&type_id)
    }

    pub fn entities(&self) -> &[Entity] {
        &self.entities
    }

    pub fn was_added(
        &self,
        row: TableRowIndex,
        component_id: ComponentId,
        current_tick: u32,
    ) -> bool {
        if let Some(column) = self.columns.get(&component_id) {
            *column.added_ticks[*row] == current_tick
        } else {
            false
        }
    }

    pub fn was_changed(
        &self,
        row: TableRowIndex,
        component_id: ComponentId,
        current_tick: u32,
    ) -> bool {
        if let Some(column) = self.columns.get(&component_id) {
            *column.changed_ticks[*row] == current_tick || *column.added_ticks[*row] == current_tick
        } else {
            false
        }
    }

    pub fn remove_swap(&mut self, row: TableRowIndex) -> TableRow {
        let removed_row_data = self
            .columns
            .iter_mut()
            .map(|(id, col)| {
                let index: usize = *row;
                let removed_added_tick = col.added_ticks.swap_remove(index);
                let removed_changed_tick = col.changed_ticks.swap_remove(index);

                let mut new_col_vec = col.clone_empty_data();
                new_col_vec.push(col.data.swap_remove(index));

                (
                    *id,
                    TableRowData {
                        component_data: new_col_vec,
                        added_tick: removed_added_tick,
                        changed_tick: removed_changed_tick,
                    },
                )
            })
            .collect();

        TableRow {
            data: removed_row_data,
            entity: self.entities.swap_remove(*row),
        }
    }
}

impl Default for Table {
    fn default() -> Self {
        Self::new()
    }
}

pub(crate) struct TableRowData {
    component_data: AnyVec,
    added_tick: Tick,
    changed_tick: Tick,
}

impl TableRowData {
    fn make_empty<T: Component>(tick: Tick) -> TableRowData {
        TableRowData {
            component_data: AnyVec::new::<T>(),
            added_tick: tick,
            changed_tick: tick,
        }
    }
}

pub struct TableRow {
    pub(crate) data: HashMap<ComponentId, TableRowData>,
    pub(crate) entity: Entity,
}

impl TableRow {
    pub fn insert<T: Component>(&mut self, raw_value: AnyValueWrapper<T>, current_tick: Tick) {
        let table_row_data = self
            .data
            .entry(TypeId::of::<T>())
            .or_insert(TableRowData::make_empty::<T>(current_tick));

        table_row_data.component_data.push(raw_value);
        table_row_data.added_tick.set(*current_tick);
        table_row_data.changed_tick.set(*current_tick);
    }

    pub fn remove<T: Component>(&mut self) {
        self.data.remove(&TypeId::of::<T>());
    }
}

#[derive(Clone, Copy, PartialEq)]
pub struct TableRowIndex(usize);

impl TableRowIndex {
    pub const fn new(index: usize) -> TableRowIndex {
        TableRowIndex(index)
    }
}

impl Deref for TableRowIndex {
    type Target = usize;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl DerefMut for TableRowIndex {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.0
    }
}

pub(crate) struct MutableCellAccessor<'w, T> {
    pub(crate) data: &'w mut T,
    pub(crate) changed_tick: &'w mut Tick,
}
