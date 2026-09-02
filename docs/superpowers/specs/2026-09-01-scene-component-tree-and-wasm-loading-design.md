# Generic Scene Component Tree & Cross-Platform Cooked Loading

## Problem

Two regressions block merging `asset-store-rework` into `master`.

1. **`Scene` cannot represent skeletons or animation.** The asset-import pipeline replaced the runtime glTF loader with an offline `GltfImporter` emitting a `Scene` sub-asset, but `SceneNode` only carries `name`/`transform`/`children`/`mesh`/`material`. Skeleton, animation, camera, light, and the Blender-`extras`-driven component injection from PR #56 were dropped. `examples/tech-demo` and `examples/animation-test` are parked in `[workspace].exclude` because they cannot be migrated.

2. **wasm asset loading is broken.** The four cooked-format loaders (`TextureLoader`, `MeshLoader`, `StandardMaterialLoader`, `SceneLoader`) read cooked bytes with `std::fs::read(Path::new("res")/…)` — CWD-relative and native-only. The previous `essential::assets::utils::{load_binary, load_to_string}` had an executable-relative native branch and a `reqwest` HTTP branch for wasm; both are now dead code with zero callers. wasm still *compiles* (`std::fs` exists on `wasm32`), so every cooked asset load fails silently at runtime and the documented `trunk serve` workflow is broken.

## Goals

- `Scene` represents **an arbitrary tree of entities and their serializable components** — not a fixed set of typed fields. Any registered component round-trips through a cooked scene with no change to `Scene` itself.
- Skeletal animation works end-to-end from a cooked glTF: skeleton binding, bone hierarchy, and loadable `AnimationClip` assets.
- The Blender-`extras` component injection from PR #56 continues to work (Goal #4 of the original pipeline design).
- Cooked assets load on both native and wasm through one shared code path.
- `examples/tech-demo` and `examples/animation-test` return to the workspace and animate.

## Non-Goals

- **Serializing the animation graph.** `AnimationPlayer`'s graph/blackboard/pose-pool is runtime state. The scene cooks *clips as assets* and binds the skeleton; applications keep building their own blend graphs (`setup_animations`, `setup_state_machine`), exactly as today.
- **A binary format for component payloads.** Component data is serde-JSON embedded in the bincode `Scene`. Scene component data is small (a `Transform` is ten floats; a mesh reference is one UUID), JSON keeps cooked scenes debuggable, and glTF `extras` arrive as JSON already.
- Runtime fallback to parsing DCC sources. Unchanged: the runtime reads only cooked output.
- Physics-shape generation from scenes (`GLTFSpawnerComponent::with_physics_shapes` has no replacement yet).

## Architecture Overview

Two phases, landing in order.

**Phase 1 — cross-platform cooked loading.** `AssetLoadContext` carries a `CookedAssetRoot`; one shared `async` helper resolves cooked bytes per platform; the four loaders call it.

**Phase 2 — generic scene component tree.** `SceneNode` holds `Vec<SerializedComponent>` instead of typed fields. A serde-backed component registry deserializes each payload into its concrete type and inserts it. Each payload's type implements one trait, `SceneComponent`, whose `apply` turns it into runtime components — resolving asset handles and entity references, and expanding into several components where needed. `GltfImporter` emits real component values; `spawn_scene_components` inserts them generically.

### Why serde, not facet

PR #56 introduced a `facet`-based reflection registry (`register_reflection`, `insert_from_json`) for injecting components from Blender `extras`. A spike established that facet cannot carry this design: **`glam::Vec3`/`Quat` do not implement `Facet`**, glam ships no `facet` feature, and the orphan rule prevents adding one — so `Transform`, the most fundamental component, could never round-trip. Working around it would require an at-rest DTO for every glam-bearing component, defeating the "any component" goal.

serde has none of these problems, and the asset-import pipeline already made the codebase serde-native:

- `glam`'s `serde` feature is already enabled in `essential`; `Transform` already derives `Serialize`/`Deserialize`.
- `uuid`'s serde feature is already enabled.
- `AssetHandle<A>`'s manual `Serialize`/`Deserialize` impls already exist and already behave correctly — they write the bare `AssetId` and read back a `Weak` handle. The shipped `SceneNode` already round-trips `Option<AssetHandle<Mesh>>` through them.
- Every asset and component type the pipeline touched is already serde-derived.

**Decision: the facet registry is replaced by a serde registry.** There is one registration function. `facet`/`facet-json` become removable from `ecs`.

---

## Phase 1: Cross-Platform Cooked Loading

### `CookedAssetRoot`

```rust
// crates/essential/src/assets/mod.rs
pub enum CookedAssetRoot {
    /// Native: absolute path to the directory containing `.cooked/`.
    Directory(PathBuf),
    /// wasm: URL base, e.g. "http://host/res".
    UrlBase(String),
}
```

Defaults: `Directory(<exe-dir>/res)` on native — restoring the executable-relative convention the deleted `load_binary` used — and `UrlBase("<window.location.origin>/res")` on wasm. Configurable through the `AssetServer`.

### The shared helper

```rust
// crates/essential/src/assets/utils.rs
pub async fn load_cooked_asset_bytes(
    root: &CookedAssetRoot,
    id: AssetId,
) -> anyhow::Result<Vec<u8>>;
```

Native: `std::fs::read(dir.join(".cooked").join(format!("{}.bin", id.simple_hex())))`.
wasm: `reqwest::get(format!("{url_base}/.cooked/{}.bin", id.simple_hex()))`, reusing the `cfg_if` structure already in `utils.rs`.

This is the runtime mirror of `asset_cook::cooked_file_path_for_id`, which stays the cook-side function. A test asserts the two agree on the `.cooked/<simple_hex>.bin` layout.

### Loader changes

`AssetLoadContext` gains a `cooked_root: CookedAssetRoot` field alongside `asset_server` and `asset_id`, populated at its single construction site in `request_load`. Each of the four loaders replaces

```rust
let cooked_path = asset_cook::cooked_file_path_for_id(Path::new("res"), load_context.asset_id());
let bytes = std::fs::read(&cooked_path)?;
```

with

```rust
let bytes = load_cooked_asset_bytes(load_context.cooked_root(), load_context.asset_id()).await?;
```

removing the hard-coded `"res"` and the four `TODO(follow-up)` comments about it.

`examples/render-test` drops its `set_current_dir(CARGO_MANIFEST_DIR)` workaround and regains a one-line `build.rs` copying `res/` next to the binary — the convention the other examples use and what the executable-relative default expects.

The now doubly-dead `load_binary`/`load_to_string` are deleted.

---

## Phase 2: Generic Scene Component Tree

### Core types

```rust
// crates/scene/src/scene.rs
#[derive(Serialize, Deserialize)]
pub struct SerializedComponent {
    /// Registry key — the name `Component::name()` returns.
    pub type_name: String,
    /// serde-JSON encoding of the component value.
    pub data: String,
}

#[derive(Serialize, Deserialize)]
pub struct SceneNode {
    pub name: String,
    /// Indices into `Scene::nodes`.
    pub children: Vec<usize>,
    pub components: Vec<SerializedComponent>,
}

#[derive(Asset, Serialize, Deserialize)]
pub struct Scene {
    pub nodes: Vec<SceneNode>,
    /// Every AssetId reachable from the nodes' components.
    pub referenced_assets: Vec<AssetId>,
}
```

`Scene` stays a `CookedAsset` serialized with bincode; component payloads are JSON strings inside it.

`referenced_assets` exists because component payloads are opaque to `Scene`: `CookedAsset::referenced_sub_assets` can no longer introspect typed handle fields, so the importer accumulates the ids as it emits and the cook-time reference-integrity pass keeps working.

The tree is explicit in `children`. Parenting is deliberately *not* a component — the hierarchy is structural, and `spawn_scene_components` already wires it with `cmd.add_child`.

### The `SceneComponent` interface

Scene-authored data knows how to become runtime components. That is the whole interface — one trait, one method:

```rust
// crates/ecs/src/component/scene.rs
pub struct SceneSpawnContext<'w> {
    world: RestrictedWorld<'w>,
    /// Indexed by SceneEntityRef; one entry per Scene node, in node order.
    node_entities: &'w [Entity],
}

impl<'w> SceneSpawnContext<'w> {
    pub fn insert<T: Component>(&mut self, component: T, entity: Entity);
    pub fn entity_for(&self, r: SceneEntityRef) -> Option<Entity>;
    /// Escape hatch for resources — notably `AssetServer`, which `ecs`
    /// cannot name.
    pub fn world(&mut self) -> &mut RestrictedWorld<'w>;
}

/// Data authored into a cooked Scene that knows how to apply itself to a
/// spawned entity.
pub trait SceneComponent: Component + DeserializeOwned + Sized + 'static {
    fn apply(self, entity: Entity, ctx: &mut SceneSpawnContext<'_>);
}

#[derive(Serialize, Deserialize, Clone, Copy)]
pub struct SceneEntityRef(pub usize);
```

**Whether a type inserts itself is decided by its `apply` body, not by the trait bound.** A type that is a runtime component inserts itself; a type that is really authoring data (`SceneSkeleton`) expands into several runtime components — possibly on *other* entities — and simply never inserts one of itself. Both are the same interface.

`SceneComponent` requires `Component` for now. That keeps every registered type a component type, lets the registry key come uniformly from `Component::name()`, and makes the common insert-self body type-check without further bounds. The cost is that authoring-only data like `SceneSkeleton` must still derive `Component` even though nothing ever reads one off an entity. Relaxing this bound later is a non-breaking change; tightening it would not be, so it starts tight (see Follow-Ups).

The `Component` trait itself is unchanged: no new method.

### The component registry

One registration function replaces the facet one:

```rust
// crates/ecs/src/component/registry.rs
type ErasedApply = fn(&str, Entity, &mut SceneSpawnContext<'_>) -> anyhow::Result<()>;

fn apply_typed<T: SceneComponent>(
    json: &str, entity: Entity, ctx: &mut SceneSpawnContext<'_>,
) -> anyhow::Result<()> {
    let value: T = serde_json::from_str(json)?;
    value.apply(entity, ctx);
    Ok(())
}

// crates/app/src/lib.rs
impl App {
    pub fn register_component<T: SceneComponent>(&mut self) -> &mut Self;
}
```

The registry is keyed by `T::name()`, available on every registered type via the `Component` bound.

There is **no `#[derive(SceneComponent)]`**. Every registered type writes an explicit `impl SceneComponent`; for a plain component that is a two-line insert-self body. This keeps the machinery small and makes each type's spawn behaviour visible at its definition.

`register_reflection` is removed; its two call sites move to `register_component`. `CommandQueue::insert_from_json` is re-backed by this registry and keeps its signature, so PR #56's Blender-extras path is unchanged at the data level — glTF `extras` are already JSON. Extras component types replace their `Facet` bound with an `impl SceneComponent`.

`crates/ecs` gains `serde` (with `derive`, for `SceneEntityRef`) and `serde_json` dependencies, and loses its use of `facet`/`facet-json`. `crates/mesh`, `crates/render`, `crates/animation`, and `crates/scene` gain `serde` where they lack it, plus the `serde` feature on `glam`/`uuid` as needed — `essential` already has all of these from the pipeline work.

### Registered components

`ecs` gains no dependency on `essential`: `AssetServer` is a `Resource`, reached through `ctx.world().get_resource()` by the component's own crate.

| Type | Crate | `apply` does |
|---|---|---|
| `Transform` | essential | insert self (already serde-derived) |
| `Camera`, `Light`, `SyncWithRenderWorld` | render | insert self |
| `MeshComponent` | mesh | upgrade handle, insert self |
| `MaterialComponent` | render | upgrade handle, insert self |
| `SceneSkeleton` | scene | expand — derives `Component` to satisfy the bound, but never inserts itself |
| Blender-extras components | user | insert self |

Each gains `Serialize`/`Deserialize` derives where it lacks them (`Transform` already has them).

A runtime component that carries a reference resolves it and inserts itself:

```rust
impl SceneComponent for MeshComponent {
    fn apply(mut self, entity: Entity, ctx: &mut SceneSpawnContext<'_>) {
        if let Some(server) = ctx.world().get_resource::<AssetServer>() {
            self.handle = server.load_by_id(self.handle.id());   // Weak → Strong, kicks off the load
        }
        ctx.insert(self, entity);
    }
}
```

`AssetHandle` is **unchanged** — it stays a `Strong`/`Weak` enum with its existing serde impls, so the cooked format is untouched and no re-cook is forced by this.

`SceneSkeleton` is authoring data: its `apply` expands into runtime components and **never inserts one of itself**, so no entity carries it. It exists because `SkeletonComponent` holds `Vec<Entity>`, which cannot exist at rest:

```rust
/// Derives `Component` only to satisfy the `SceneComponent: Component`
/// bound — `apply` never inserts one, so no entity ever carries it.
#[derive(Component, Serialize, Deserialize)]
pub struct SceneSkeleton {
    pub skeleton: AssetHandle<Skeleton>,
    pub bones: Vec<SceneEntityRef>,
    pub bone_ids: Vec<Uuid>,
    pub root: Option<SceneEntityRef>,
}

impl SceneComponent for SceneSkeleton {
    fn apply(self, entity: Entity, ctx: &mut SceneSpawnContext<'_>) {
        let bones: Vec<Entity> = self.bones.iter().filter_map(|r| ctx.entity_for(*r)).collect();
        let skeleton = match ctx.world().get_resource::<AssetServer>() {
            Some(server) => server.load_by_id(self.skeleton.id()),
            None => self.skeleton,
        };
        if let Some(root) = self.root.and_then(|r| ctx.entity_for(r)) {
            ctx.insert(AnimationRootBone::default(), root);      // a different entity
        }
        ctx.insert(AnimationPlayer::new(bones.len()), entity);
        ctx.insert(SkeletonComponent::new(skeleton, bones, self.bone_ids), entity);
    }
}
```

Expansion onto other entities is an advertised capability of the interface, not a side effect — which is why `apply` receives the whole `SceneSpawnContext` rather than just `&mut self`.

### New cooked asset types

`Skeleton { inverse_bindposes: Box<[Mat4]> }` and `AnimationClip { channels: HashMap<Uuid, Vec<AnimationChannel>> }` gain `Serialize`/`Deserialize` + `CookedAsset` + `LoadableAsset`. Both are plain data — `AnimationChannel { time_samples: Vec<f32>, outputs: AnimationChanelOutput }`, and `AnimationChanelOutput` is an enum over `Vec<Vec3>`/`Vec<Quat>`/`Vec<Vec3>`. `crates/animation` and `crates/mesh` enable `glam`'s and `uuid`'s serde features.

`GltfImporter` emits `skeleton/N` and `animation/N` sub-assets, so a clip is loadable as `asset_server.load::<AnimationClip>("idle.gltf#animation/0")`.

### `GltfImporter`

Re-gains the parsing deleted in the pipeline branch, ported from git `1d0682d^:crates/gltf-loader/src/loader.rs`:

- **Skeletons** — `document.skins()`, inverse bind matrices → a `skeleton/N` sub-asset; joint node indices → `SceneSkeleton.bones`; `collect_paths`/`paths_to_uuid` restored for the stable per-bone UUIDs that `AnimationClip` channels are keyed by.
- **Animations** — `document.animations()` → an `AnimationClip` per animation → `animation/N` sub-assets, channels keyed by the same bone UUIDs.
- **Cameras / lights** — `document.cameras()`/`document.lights()` → `Camera`/`Light` components on the owning nodes; lights also get `SyncWithRenderWorld`, matching the old spawner.
- **Blender extras** — `parse_extras` restored; each entry becomes a `SerializedComponent { type_name, data }` directly, since the payload is already JSON.

Per node the importer emits a `components` list rather than typed fields, and accumulates every referenced `AssetId` into `Scene::referenced_assets`. Multi-primitive mesh flattening (parent node plus one child node per primitive) is unchanged.

### `spawn_scene_components`

```
for each spawner entity whose Scene is loaded:
    spawn one entity per node, collect into node_entities
    for each node, for each SerializedComponent:
        look up type_name in the component registry
        deserialize into its concrete type and call SceneComponent::apply
    wire children with cmd.add_child
    parent root nodes to the spawner entity
    remove SceneSpawnerComponent
```

The current typed mesh/material/`SyncWithRenderWorld` special-casing disappears — those are ordinary registered components. The one-shot guard and the out-of-range `children` bounds check are retained. Because resolution needs `node_entities`, the queued insert command carries the node→entity map; `insert_from_json` passes an empty map for non-scene callers.

---

## Data Flow

**Cook time.** `GltfImporter` parses the glTF once and emits `mesh/N`, `material/N`, `texture/N[_linear]`, `skeleton/N`, `animation/N` sub-assets; builds one `SceneNode` per glTF node with its components serialized to JSON; records every referenced `AssetId`; emits the `scene` sub-asset. The cook-time reference-integrity pass validates `referenced_assets` against everything produced.

**Runtime.** `asset_server.load::<Scene>("model.gltf#scene")` → `SceneLoader` reads cooked bytes via `load_cooked_asset_bytes` → bincode-deserializes `Scene` → `spawn_scene_components` spawns the tree, and each payload's `SceneComponent::apply` inserts its runtime components — upgrading asset handles (which triggers the nested loads) and mapping entity references.

## Error Handling

- **Unregistered `type_name`** — logged as a warning, that component skipped, the rest of the node still spawns. This preserves the existing `insert_from_json` behavior and lets a scene carry components an application has not registered.
- **Malformed component JSON** — logged with the type name and node index; component skipped.
- **Out-of-range `SceneEntityRef` or `children` index** — skipped, never a panic (generalizing the bounds guard already in the spawner).
- **Missing cooked file** — `load_cooked_asset_bytes` returns an `anyhow` error carrying the resolved path or URL.
- **Cook-time dangling reference** — unchanged: the global reference-integrity pass fails the cook run.

## Testing Strategy

- **Unit** — `Scene` bincode round-trip with mixed components; `Skeleton` and `AnimationClip` cook round-trips; `load_cooked_asset_bytes` against a `Directory` root with a temp dir; a `Transform` (glam fields) serde-JSON round-trip through the registry, pinning the reason serde was chosen.
- **Integration** — a real `spawn_scene_components` test against a `World`: a scene with a parent, a `SceneSkeleton`, and two mesh children; assert entity count, parenting, that resolved `MeshComponent` handles are `Strong`, that `SkeletonComponent::bones()` are the right entities, and that `SceneSpawnerComponent` is gone. This closes the coverage gap the pipeline branch's final review flagged.
- **Importer** — a skinned, animated glTF fixture; assert `skeleton/0` and `animation/0` sub-assets and a `SceneSkeleton` component on the expected node.
- **Registry** — a `SceneComponent` that resolves a handle produces a `Strong` one; a `SceneComponent` that expands (`SceneSkeleton`) inserts its runtime components and leaves nothing of itself on the entity; an unregistered `type_name` is skipped with a warning rather than failing the spawn.
- **Manual** — un-park `tech-demo` and `animation-test`, cook their assets, confirm they animate; `trunk serve` `render-test` to confirm the wasm path.

## Migration

Cooked output from the current format will not load: `SceneNode`'s layout changes and new sub-asset kinds appear. `COOK_FORMAT_VERSION` bumps to `2`, which already forces a full re-cook and marks stale `.index` entries dirty. Cooked output is git-ignored, so nothing on disk needs migrating.

`register_reflection` is removed in favour of `register_component`; its call sites and any Blender-extras component types replace their `Facet` bound with an `impl SceneComponent` (a two-line insert-self body). `facet` and `facet-json` become removable from `ecs`.

`AssetHandle` is unchanged, so no consumer of it needs to change.

## Follow-Ups (out of scope)

- Physics-shape generation from scenes, replacing `GLTFSpawnerComponent::with_physics_shapes`.
- A `#[derive(SceneComponent)]` emitting the trivial insert-self body, once the number of plain registered components makes the boilerplate worth removing.
- Dropping the `SceneComponent: Component` bound, so authoring-only types such as `SceneSkeleton` need not be components at all. Non-breaking when done.
- Dropping the `facet`/`facet-json` dependencies from `ecs` once nothing references them.
- Per-usage colour space for standalone cooked textures (`ImageImporter` hard-codes sRGB).
- MTL texture addressing in `ObjImporter` (assumes manifest-root-relative, untested).
- Pruning orphaned `.cooked/` files when a source leaves the manifest.
- Per-importer `Importer::validate()` implementations.
