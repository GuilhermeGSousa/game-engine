use std::{any::TypeId, collections::HashMap};

use any_vec::{any_value::AnyValueTypelessRaw, AnyVec};

use crate::component::Component;

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
}

pub struct Table {
    columns: HashMap<TypeId, Column>,
}

impl Table {
    pub fn new() -> Self {
        Self {
            columns: HashMap::new(),
        }
    }

    pub fn add_column<T: Component>(&mut self) {
        self.columns.insert(TypeId::of::<T>(), Column::new::<T>());
    }

    pub fn get_column_mut(&mut self, type_id: TypeId) -> Option<&mut Column> {
        self.columns.get_mut(&type_id)
    }

    pub fn get_row_count(&self) -> usize {
        self.columns
            .values()
            .next()
            .map(|column| column.len())
            .unwrap_or(0)
    }
}
