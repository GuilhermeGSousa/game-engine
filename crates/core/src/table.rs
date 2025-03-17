use std::collections::HashMap;

use any_vec::{any_value::AnyValueTypelessRaw, mem::Heap, AnyVec, AnyVecMut, AnyVecRef};

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

    pub fn push(&mut self, raw_value: AnyValueTypelessRaw) {
        unsafe { self.data.push_unchecked(raw_value) };
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

    pub unsafe fn get_unsafe<T: 'static>(&self, index: usize) -> &T {
        self.data.get_unchecked(index).downcast_ref().unwrap()
    }

    pub unsafe fn get_mut_unsafe<T: 'static>(&mut self, index: usize) -> &mut T {
        self.data.get_unchecked_mut(index).downcast_mut().unwrap()
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
