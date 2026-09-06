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

/// A short-name alias slot: the applier plus the `TypeId` that currently owns
/// the alias. Re-registering the *same* type overwrites the applier silently;
/// only a second, *different* type claiming the alias is a real collision.
struct AliasEntry {
    type_id: TypeId,
    apply: ErasedApply,
}

#[derive(Default)]
pub(crate) struct ComponentRegistry {
    component_map: HashMap<ComponentId, ComponentIndex>,
    /// Canonical key: the full type path (`T::name()`). Unique per type, so an
    /// entry here is only ever re-inserted by the same type.
    scene_component_map: HashMap<&'static str, ErasedApply>,
    /// Convenience key: the final `::` segment, for hand-authored glTF `extras`.
    scene_alias_map: HashMap<&'static str, AliasEntry>,
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
    /// resolves.
    ///
    /// Re-registering the same `T` (two plugins that both register it, or one
    /// plugin added twice) is idempotent and silent. Only a *different* type
    /// claiming an already-taken short name warns; the last registration wins
    /// the alias, matching the previous plain-`insert` behaviour.
    pub(crate) fn register_scene_component<T: SceneComponent>(&mut self) {
        self.register_component::<T>();

        let full = T::name();
        self.scene_component_map.insert(full, apply_typed::<T>);

        // Short alias so Blender-authored `extras` keys resolve. `full` is
        // 'static, so the suffix borrows from it without allocating.
        let short = full.rsplit("::").next().unwrap_or(full);
        if short != full {
            let type_id = TypeId::of::<T>();
            match self.scene_alias_map.entry(short) {
                Occupied(mut entry) => {
                    if entry.get().type_id != type_id {
                        log::warn!(
                            "Two component types share the short name '{short}'; scenes and \
                             glTF extras that use it now resolve to '{full}'. Author the full \
                             path to disambiguate."
                        );
                    }
                    *entry.get_mut() = AliasEntry {
                        type_id,
                        apply: apply_typed::<T>,
                    };
                }
                Vacant(entry) => {
                    entry.insert(AliasEntry {
                        type_id,
                        apply: apply_typed::<T>,
                    });
                }
            }
        }
    }

    pub(crate) fn get_scene_component(&self, name: &str) -> Option<ErasedApply> {
        self.scene_component_map
            .get(name)
            .copied()
            .or_else(|| self.scene_alias_map.get(name).map(|entry| entry.apply))
    }
}
