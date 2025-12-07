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

    pub fn read_world(&mut self) {
        self.reads_all = true;
    }

    pub fn write_world(&mut self) {
        self.writes_all = true;
    }

    pub fn is_compatible(&self, other: &Self) -> bool {
        if self.writes_all || other.writes_all {
            return false;
        }

        // Self reads all and other writes any
        if self.reads_all
            && (!other.component_writes.is_empty() || !other.resource_writes.is_empty())
        {
            return false;
        }

        // Other reads all and self writes any
        if other.reads_all
            && (!self.component_writes.is_empty() || !self.resource_writes.is_empty())
        {
            return false;
        }

        // Self reads component and other writes
        if !self.component_reads.is_disjoint(&other.component_writes) {
            return false;
        }

        // Self writes component and other reads
        if !self.component_writes.is_disjoint(&other.component_reads) {
            return false;
        }

        // Both write component
        if !self.component_writes.is_disjoint(&other.component_writes) {
            return false;
        }

        // Self reads resource and other writes
        if !self.resource_reads.is_disjoint(&other.resource_writes) {
            return false;
        }

        // Self writes resource and other reads
        if !self.resource_writes.is_disjoint(&other.resource_reads) {
            return false;
        }

        // Both write resource
        if !self.resource_writes.is_disjoint(&other.resource_writes) {
            return false;
        }

        return true;
    }

    pub fn combine(&mut self, other: Self) {
        self.component_reads.extend(other.component_reads);
        self.component_writes.extend(other.component_writes);
        self.resource_reads.extend(other.resource_reads);
        self.resource_writes.extend(other.resource_writes);
        self.reads_all |= other.reads_all;
        self.writes_all |= other.writes_all;
    }
}
