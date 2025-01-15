use std::any::TypeId;

use any_vec::any_value::AnyValueTypelessRaw;

use crate::table::Table;

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
}
