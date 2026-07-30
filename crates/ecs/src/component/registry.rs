use std::{
    any::TypeId,
    collections::{
        hash_map::Entry::{Occupied, Vacant},
        HashMap,
    },
};

use derive_more::{Deref, DerefMut};

use crate::{component::ComponentId, Component};

pub(crate) struct ComponentInfo {}

#[derive(Deref, DerefMut)]
pub(crate) struct ComponentIndex(usize);

#[derive(Default)]
pub(crate) struct ComponentRegistry {
    component_map: HashMap<ComponentId, ComponentIndex>,
    component_info: Vec<ComponentInfo>,
}

impl ComponentRegistry {
    pub(crate) fn register_component<T: Component>(&mut self) {
        match self.component_map.entry(TypeId::of::<T>()) {
            Occupied(occupied_entry) => {
                let info = self
                    .component_info
                    .get(occupied_entry.get().0)
                    .expect(&format!(
                        "Registered component {} has no matching component info",
                        T::name()
                    ));
            }
            Vacant(vacant_entry) => {
                vacant_entry.insert(ComponentIndex(self.component_info.len()));
                self.component_info.push(ComponentInfo {});
            }
        };
    }
}
