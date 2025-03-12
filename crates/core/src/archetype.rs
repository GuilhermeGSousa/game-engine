use std::any::TypeId;

use any_vec::{any_value::AnyValueTypelessRaw, mem::Heap, AnyVecMut, AnyVecRef};

use crate::{
    bundle::ComponentBundle,
    component::{Component, ComponentId},
    table::Table,
};

pub struct Archetype {
    data_table: Table,
}

impl Archetype {
    pub fn new(data_table: Table) -> Archetype {
        Archetype { data_table }
    }

    pub fn add_component(&mut self, type_id: TypeId, raw_value: AnyValueTypelessRaw) {
        if let Some(column) = self.data_table.get_column_mut(type_id) {
            column.push(raw_value);
        }
    }

    pub fn get_row_count(&self) -> usize {
        self.data_table.get_row_count()
    }

    pub fn has_component<'s, T: Component + 'static>(&self) -> bool {
        self.data_table.has_column(ComponentId::of::<T>())
    }

    pub fn len(&self) -> usize {
        self.data_table.get_row_count()
    }

    pub fn is_compatible_with_bundle<T: ComponentBundle>(&self) -> bool {
        true
    }

    pub unsafe fn get_component_vector_unsafe<T: Component + 'static>(&self) -> AnyVecRef<T, Heap> {
        self.data_table
            .get_column(ComponentId::of::<T>())
            .unwrap()
            .as_vec_unchecked()
    }

    pub unsafe fn get_component_vector_mut_unsafe<T: Component + 'static>(
        &mut self,
    ) -> AnyVecMut<T, Heap> {
        self.data_table
            .get_column_mut(ComponentId::of::<T>())
            .unwrap()
            .as_vec_mut_unchecked()
    }

    pub unsafe fn get_component_unsafe<T: 'static>(&self, index: usize) -> &T {
        self.data_table
            .get_column(ComponentId::of::<T>())
            .unwrap()
            .get_unsafe(index)
    }

    pub unsafe fn get_component_mut_unsafe<T: 'static>(&mut self, index: usize) -> &mut T {
        self.data_table
            .get_column_mut(ComponentId::of::<T>())
            .unwrap()
            .get_mut_unsafe(index)
    }
}
