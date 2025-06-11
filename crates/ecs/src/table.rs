use std::{
    collections::HashMap,
    ops::{Deref, DerefMut},
};

use any_vec::{any_value::AnyValueWrapper, mem::Heap, AnyVec, AnyVecMut, AnyVecRef};

use crate::component::{Component, ComponentId};

pub struct Column {
    data: AnyVec,
}

impl Column {
    pub fn new<T: Component>() -> Self {
        Self {
            data: AnyVec::new::<T>(),
        }
    }

    pub fn push<T>(&mut self, raw_value: AnyValueWrapper<T>) {
        self.data.push(raw_value);
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

    pub unsafe fn get_unsafe<T: 'static>(&self, row: TableRow) -> Option<&T> {
        self.data.get_unchecked(*row as usize).downcast_ref()
    }

    pub unsafe fn get_mut_unsafe<T: 'static>(&mut self, row: TableRow) -> Option<&mut T> {
        self.data.get_unchecked_mut(*row as usize).downcast_mut()
    }
}

pub struct Table {
    columns: HashMap<ComponentId, Column>,
}

impl Table {
    pub fn new() -> Self {
        Self {
            columns: HashMap::new(),
        }
    }

    pub fn add_column<T: Component>(&mut self) {
        self.columns
            .insert(ComponentId::of::<T>(), Column::new::<T>());
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
}

#[derive(Clone, Copy, PartialEq)]
pub struct TableRow(u32);

impl TableRow {
    pub const fn new(index: u32) -> TableRow {
        TableRow(index)
    }
}

impl Deref for TableRow {
    type Target = u32;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl DerefMut for TableRow {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.0
    }
}
