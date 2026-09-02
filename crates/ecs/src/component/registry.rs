use std::{
    any::TypeId,
    collections::{
        HashMap,
        hash_map::Entry::{Occupied, Vacant},
    },
};

use derive_more::{Deref, DerefMut};

use crate::{
    Component, Entity,
    component::{
        ComponentId,
        scene::{SceneComponent, SceneSpawnContext},
    },
};

pub(crate) struct ComponentInfo {}

#[derive(Deref, DerefMut)]
pub(crate) struct ComponentIndex(usize);

/// Deserializes a JSON payload into `T` and applies it. Returns `Err` with a
/// human-readable reason when the payload does not parse.
type ErasedApply = fn(&str, Entity, &mut SceneSpawnContext<'_>) -> Result<(), serde_json::Error>;

fn apply_typed<T: SceneComponent>(
    json: &str,
    entity: Entity,
    ctx: &mut SceneSpawnContext<'_>,
) -> Result<(), serde_json::Error> {
    let value: T = serde_json::from_str(json)?;
    value.apply(entity, ctx);
    Ok(())
}

#[derive(Default)]
pub(crate) struct ComponentRegistry {
    component_map: HashMap<ComponentId, ComponentIndex>,
    scene_component_map: HashMap<&'static str, ErasedApply>,
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

    /// Registers `T` under two keys: its canonical full type path and its final
    /// `::` segment. Importers emit the full path; the short alias exists so a
    /// hand-authored glTF `extras` key like `{"MeshCollider": {}}` still
    /// resolves. Full-path keys are inserted first and never aliased, so only
    /// the short alias can collide — and a collision warns rather than failing
    /// silently.
    pub(crate) fn register_scene_component<T: SceneComponent>(&mut self) {
        self.register_component::<T>();

        let full = T::name();
        self.scene_component_map.insert(full, apply_typed::<T>);

        // Short alias so Blender-authored `extras` keys resolve. `full` is
        // 'static, so the suffix borrows from it without allocating.
        let short = full.rsplit("::").next().unwrap_or(full);
        if short != full {
            let displaced = self.scene_component_map.insert(short, apply_typed::<T>);
            if displaced.is_some() {
                log::warn!(
                    "Two component types share the short name '{short}'; scenes and glTF \
                     extras that use it now resolve to '{full}'. Author the full path to \
                     disambiguate."
                );
            }
        }
    }

    pub(crate) fn get_scene_component(&self, name: &str) -> Option<ErasedApply> {
        self.scene_component_map.get(name).copied()
    }
}
