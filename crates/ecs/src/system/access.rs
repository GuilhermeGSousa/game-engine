use std::{any::TypeId, collections::HashSet};

use crate::{component::Component, resource::Resource};

#[derive(Default)]
pub struct SystemAccess {
    component_reads: HashSet<TypeId>,
    component_writes: HashSet<TypeId>,
    resource_reads: HashSet<TypeId>,
    resource_writes: HashSet<TypeId>,
    reads_all: bool,
    writes_all: bool,
}

impl SystemAccess {
    pub fn read_component<T: Component>(&mut self) {
        self.component_reads.insert(TypeId::of::<T>());
    }

    pub fn write_component<T: Component>(&mut self) {
        self.component_writes.insert(TypeId::of::<T>());
    }

    pub fn read_resource<T: Resource>(&mut self) {
        self.resource_reads.insert(TypeId::of::<T>());
    }

    pub fn write_resource<T: Resource>(&mut self) {
        self.resource_writes.insert(TypeId::of::<T>());
    }

    pub fn read_world(&mut self)
    {
        self.reads_all = true;
    }

    pub fn write_world(&mut self)
    {
        self.writes_all = true;
    }

    pub fn is_compatible(other: &Self) -> bool
    {
        todo!()
    }
}
