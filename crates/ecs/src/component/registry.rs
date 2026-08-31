use std::{
    any::TypeId,
    collections::{
        HashMap,
        hash_map::Entry::{Occupied, Vacant},
    },
};

use derive_more::{Deref, DerefMut};
use facet::Facet;

use crate::{
    Component,
    component::{ComponentId, reflection::ComponentReflection},
};

pub(crate) struct ComponentInfo {}

#[derive(Deref, DerefMut)]
pub(crate) struct ComponentIndex(usize);

#[derive(Default)]
pub(crate) struct ComponentRegistry {
    component_map: HashMap<ComponentId, ComponentIndex>,
    reflection_map: HashMap<&'static str, ComponentReflection>,
    component_info: Vec<ComponentInfo>,
}

impl ComponentRegistry {
    pub(crate) fn register_component<T: Component>(&mut self) {
        match self.component_map.entry(TypeId::of::<T>()) {
            Occupied(occupied_entry) => {
                let _ = self
                    .component_info
                    .get(occupied_entry.get().0)
                    .unwrap_or_else(|| {
                        panic!(
                            "Registered component {} has no matching component info",
                            T::name()
                        )
                    });
            }
            Vacant(vacant_entry) => {
                vacant_entry.insert(ComponentIndex(self.component_info.len()));
                self.component_info.push(ComponentInfo {});
            }
        };
    }

    pub(crate) fn register_refection<T: Component + for<'a> Facet<'a>>(&mut self) {
        self.register_component::<T>();

        self.reflection_map
            .insert(T::name(), ComponentReflection::from_type::<T>());
    }

    pub(crate) fn get_reflection(&self, name: &str) -> Option<&ComponentReflection> {
        self.reflection_map.get(name)
    }
}
