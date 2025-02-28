use crate::component::{Component, ComponentId};
use std::collections::HashSet;

pub struct SystemMetadata {
    read_components: HashSet<ComponentId>,
    write_components: HashSet<ComponentId>,
}

impl SystemMetadata {
    pub fn new() -> Self {
        Self {
            read_components: HashSet::new(),
            write_components: HashSet::new(),
        }
    }

    pub fn with_read<T: Component + 'static>(mut self) -> Self {
        self.read_components.insert(ComponentId::of::<T>());
        self
    }

    pub fn with_write<T: Component + 'static>(mut self) -> Self {
        self.write_components.insert(ComponentId::of::<T>());
        self
    }

    pub fn extend(&mut self, other: SystemMetadata) {
        self.read_components.extend(other.read_components);
        self.write_components.extend(other.write_components);
    }

    pub fn can_read<T: Component + 'static>(&self) -> bool {
        self.read_components.contains(&ComponentId::of::<T>())
    }

    pub fn can_write<T: Component + 'static>(&self) -> bool {
        self.write_components.contains(&ComponentId::of::<T>())
    }
}
