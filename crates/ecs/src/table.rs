use std::{
    any::Any,
    collections::HashMap,
    ops::{Deref, DerefMut},
};

use any_vec::{
    any_value::AnyValueWrapper,
    mem::Heap,
    ops::{SwapRemove, TempValue},
    traits::None,
    AnyVec, AnyVecMut, AnyVecRef,
};

use crate::{
    component::{Component, ComponentId, Tick},
    entity::{self, Entity},
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
        let index = *row as usize;
        self.data.insert(index, raw_value);
        self.added_ticks.insert(index, Tick::new(tick));
        self.changed_ticks.insert(index, Tick::new(0));
    }

    pub fn len(&self) -> usize {
        self.data.len()
    }

    pub unsafe fn as_vec_unchecked<T: 'static>(&self) -> AnyVecRef<T, Heap> {
        self.data.downcast_ref_unchecked::<T>()
    }

    pub unsafe fn as_vec_mut_unchecked<T: 'static>(&mut self) -> AnyVecMut<T, Heap> {
        self.data.downcast_mut_unchecked::<T>()
    }

    pub unsafe fn get_unsafe<T: 'static>(&self, row: TableRowIndex) -> Option<&T> {
        self.data.get_unchecked(*row as usize).downcast_ref()
    }

    pub unsafe fn get_mut_unsafe<T: 'static>(&mut self, row: TableRowIndex) -> Option<&mut T> {
        self.data.get_unchecked_mut(*row as usize).downcast_mut()
    }

    pub fn set_changed(&mut self, row: TableRowIndex, current_tick: u32) {
        self.changed_ticks[*row as usize].set(current_tick);
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

    pub fn add_column<T: Component>(&mut self) {
        self.columns
            .insert(ComponentId::of::<T>(), Column::new::<T>());
    }

    pub fn add_entity(&mut self, entity: Entity) {
        self.entities.push(entity);
    }

    pub fn insert_entity(&mut self, row: TableRowIndex, entity: Entity) {
        self.entities.insert(*row as usize, entity);
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
            *column.added_ticks[*row as usize] == current_tick
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
            *column.changed_ticks[*row as usize] == current_tick
        } else {
            false
        }
    }

    pub fn remove_swap(&mut self, row: TableRowIndex) -> RemovedRow {
        let removed_row_data = self
            .columns
            .iter_mut()
            .map(|(_, col)| {
                let index = *row as usize;
                col.added_ticks.swap_remove(index);
                col.changed_ticks.swap_remove(index);
                col.data.swap_remove(index)
            })
            .collect();

        RemovedRow {
            data: removed_row_data,
            entity: self.entities.swap_remove(*row as usize),
        }
    }
}

pub(crate) struct RemovedRow<'a> {
    pub(crate) data: Vec<SwapRemove<'a, dyn None, Heap>>,
    pub(crate) entity: Entity,
}

#[derive(Clone, Copy, PartialEq)]
pub struct TableRowIndex(u32);

impl TableRowIndex {
    pub const fn new(index: u32) -> TableRowIndex {
        TableRowIndex(index)
    }
}

impl Deref for TableRowIndex {
    type Target = u32;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl DerefMut for TableRowIndex {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.0
    }
}
