use any_vec::{any_value::AnyValueWrapper, mem::Heap, AnyVecMut, AnyVecRef};

use crate::{
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

    pub fn add_component<T: Component>(&mut self, raw_value: AnyValueWrapper<T>) {
        if let Some(column) = self.data_table.get_column_mut(ComponentId::of::<T>()) {
            column.push(raw_value);
        }
    }

    pub fn get_row_count(&self) -> usize {
        self.data_table.get_row_count()
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
