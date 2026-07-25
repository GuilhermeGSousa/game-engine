# Blend loader + generic Scene / SceneSpawner

## Goal

Add a `.blend` asset loader that mirrors the existing glTF path, and along the
way pull the "asset that can be spawned as a scene" concept out of glTF into a
reusable abstraction shared by both formats:

- A **`Scene` asset trait** describing any asset that spawns an entity hierarchy
  (implemented by `GLTFScene` and the new `BlendScene`).
- A **generic `SceneSpawner<S>` component** replacing the concrete
  `GLTFSpawnerComponent`. It holds an `AssetHandle<S>` for any `S: Scene` and a
  generic spawn system turns it into entities.
- A **`.blend` loader + `BlendScene` asset** built on the [`blend`] crate
  (pure-Rust runtime parse of Blender's DNA blocks), living in a new
  `crates/blend-loader` that parallels `crates/gltf-loader`.

[`blend`]: https://crates.io/crates/blend

## Current state (what we're generalizing)

`crates/gltf-loader/src/loader.rs` today contains three things that are really
format-agnostic:

- `GLTFSpawnerComponent { handle: AssetHandle<GLTFScene>, generate_physics_shapes: bool }`
  — a handle plus spawn options.
- `GLTFInstance { roots, nodes_by_name, animation_players }` — back-references
  written onto the spawner entity once spawning completes.
- `spawn_gltf_components` — a `Query<(Entity, &GLTFSpawnerComponent, Option<&Transform>)>`
  + `Res<AssetStore<GLTFScene>>` system that: ensures the entity has a
  `Transform`, spawns a node entity per node, wires parent/child edges, parents
  roots under the spawner entity, then inserts mesh/material/skeleton/
  animation-player/camera/light components, removes the spawner component, and
  inserts the instance.

Only the *body* of the spawn loop is glTF-specific (it reads `asset.nodes`,
`asset.meshes`, `asset.skeletons`, …). The outer scaffolding (transform
fixup, remove-spawner / insert-instance) is generic. That split is what the
`Scene` trait formalizes.

The loader itself (`GLTFLoader: AssetLoader`, `GLTFScene: LoadableAsset` with
its `GLTFUsageSettings { root_bone }`) stays glTF-specific and is **not**
touched by the refactor — `UsageSettings` are load-time options, distinct from
the spawn-time options that move into `SceneSpawner`.

## Design

### 1. New crate: `crates/scene`

Lightweight home for the shared abstraction. Depends only on `ecs`,
`essential`, and `app` (for the plugin) — **not** on `render`/`mesh`/`physics`,
because all format-specific spawning stays in the concrete impls.

```rust
// crates/scene/src/lib.rs
use ecs::{command::CommandQueue, component::Component, entity::Entity, query::Query, resource::Res};
use essential::assets::{Asset, asset_store::AssetStore, handle::AssetHandle};
use essential::transform::Transform;

/// Options applied when spawning any scene, shared across formats.
#[derive(Clone, Default)]
pub struct SceneSpawnSettings {
    pub generate_physics_shapes: bool,
}

/// An asset that can be instantiated into an entity hierarchy under a root entity.
pub trait Scene: Asset {
    /// Spawn this scene's contents. `root` is the spawner entity (already
    /// guaranteed to carry a `Transform`); scene roots are parented under it.
    /// Returns the instance back-references to record on `root`.
    fn spawn(
        &self,
        cmd: &mut CommandQueue,
        root: Entity,
        settings: &SceneSpawnSettings,
    ) -> SceneInstance;
}

/// Generic spawner component: replaces `GLTFSpawnerComponent`.
#[derive(Component)]
pub struct SceneSpawner<S: Scene> {
    pub handle: AssetHandle<S>,
    pub settings: SceneSpawnSettings,
}

impl<S: Scene> SceneSpawner<S> {
    pub fn from_handle(handle: AssetHandle<S>) -> Self {
        Self { handle, settings: SceneSpawnSettings::default() }
    }
    pub fn with_physics_shapes(mut self) -> Self {
        self.settings.generate_physics_shapes = true;
        self
    }
}

/// Back-references recorded on the spawner entity once spawning finishes.
/// (Generalized `GLTFInstance`.)
#[derive(Component, Default)]
pub struct SceneInstance {
    roots: Vec<Entity>,
    nodes_by_name: std::collections::HashMap<String, Entity>,
    animation_players: Vec<Entity>,
}
// + the existing accessors: roots(), get_node(), animation_players(), animation_player(),
//   plus builder/setters the concrete `spawn` impls use to populate it.
```

The generic system — registered once per concrete `S`:

```rust
pub fn spawn_scene_components<S: Scene>(
    mut cmd: CommandQueue,
    spawners: Query<(Entity, &SceneSpawner<S>, Option<&Transform>)>,
    assets: Res<AssetStore<S>>,
) {
    for (entity, spawner, transform) in spawners.iter() {
        if let Some(asset) = assets.get(&spawner.handle) {
            if transform.is_none() {
                cmd.insert(Transform::IDENTITY, entity);
            }
            let instance = asset.spawn(&mut cmd, entity, &spawner.settings);
            cmd.remove::<SceneSpawner<S>>(entity);
            cmd.insert(instance, entity);
        }
    }
}
```

Registration helper as a generic plugin (keeps `app` free of a `scene`
dependency):

```rust
pub struct ScenePlugin<S>(PhantomData<S>);
impl<S: Scene + LoadableAsset> Plugin for ScenePlugin<S> {
    fn build(&self, app: &mut App) {
        app.register_asset::<S>();
        app.add_system(UpdateGroup::Update, spawn_scene_components::<S>);
    }
}
```

> **Feasibility note:** `spawn_scene_components::<S>` is a generic fn
> instantiated per concrete type; each instantiation is an ordinary system with
> `Query`/`Res` params, so `add_system` accepts it exactly like the current
> concrete `spawn_gltf_components`. This is the same monomorphize-per-type
> pattern the asset store already relies on.

### 2. Refactor `crates/gltf-loader`

- Delete `GLTFSpawnerComponent`; expose `pub type GLTFSpawner = SceneSpawner<GLTFScene>;`
  (keeps a friendly name and the `from_handle`/`with_physics_shapes` builders via the generic impl).
- Delete `GLTFInstance`; use `scene::SceneInstance`. glTF-specific `GLTFInstance`
  accessors (`get_node`, `animation_players`, …) are already generic enough to
  live on `SceneInstance`.
- Move the body of `spawn_gltf_components` into `impl Scene for GLTFScene::spawn`,
  taking `(cmd, root, settings)` and returning a populated `SceneInstance`.
  `component.generate_physics_shapes` becomes `settings.generate_physics_shapes`.
  Everything below the transform-fixup / remove / insert scaffolding transplants
  verbatim.
- `GLTFPlugin::build` becomes `app.add_plugin(ScenePlugin::<GLTFScene>::default())`
  (replacing the manual `register_asset` + `add_system`).
- Add `scene = { path = "../scene" }` to `gltf-loader/Cargo.toml`.
- Update call sites in `examples/*` that reference `GLTFSpawnerComponent` /
  `GLTFInstance` to the new names (grep: `examples/render-test`,
  `examples/animation-test`, `examples/tech-demo`).

This refactor is behavior-preserving and can land (and be reviewed/tested) on
its own, before any `.blend` code exists.

### 3. New crate: `crates/blend-loader`

Parallels `gltf-loader`: `loader.rs`, `plugin.rs`, `lib.rs`, `Cargo.toml`.

**Asset + loader.** `BlendScene` mirrors the subset of `GLTFScene` we can
extract from a `.blend`, and `BlendLoader: AssetLoader` parses via the `blend`
crate:

```rust
use blend::Blend;

#[derive(Asset)]
pub struct BlendScene {
    nodes: Vec<BlendNode>,                 // one per Blender Object (OB)
    meshes: Vec<BlendMesh>,                // Mesh assets + material indices
    materials: Vec<AssetHandle<StandardMaterial>>,
    lights: Vec<BlendLight>,
    cameras: Vec<BlendCamera>,
}
```

Loader outline (native path; see wasm note below):

```rust
let bytes = essential::assets::utils::load_binary(path.clone()).await?;
let blend = Blend::new(std::io::Cursor::new(bytes))?;   // parse DNA blocks

// Objects → nodes (transform from `obmat`/`loc`+`rot`+`size`, parent pointer → hierarchy).
for obj in blend.instances_with_code(*b"OB") {
    let name  = obj.get("id").get_string("name");
    let matrix = obj.get_f32_matrix("obmat");           // 4x4 world matrix
    let data   = obj.get("data");                        // → ME / LA / CA
    // classify by `obj.get_i16("type")` or the pointed-to block's code
}

// Meshes (ME) → render::assets::mesh::Mesh (Vertex { pos, uv, normal, tangent, … }).
// Materials (MA) → render StandardMaterial (base color / metallic / roughness).
// Lamps (LA) → BlendLight, Cameras (CA) → BlendCamera.
```

**`impl Scene for BlendScene::spawn`** does the same shape of work the glTF impl
does: spawn a node entity per object, wire parent/child from the object parent
pointers, parent roots under `root`, insert `MeshComponent` +
`MaterialComponent` (+ `MeshCollider` when `settings.generate_physics_shapes`),
and `Light`/`Camera` components, returning a `SceneInstance`.

**Plugin:**

```rust
impl Plugin for BlendPlugin {
    fn build(&self, app: &mut App) {
        // register any sub-assets the loader `add`s (textures via render, etc.)
        app.add_plugin(ScenePlugin::<BlendScene>::default());
    }
}
```

**Cargo.toml** deps mirror `gltf-loader` (app, ecs, essential, render, mesh,
physics, color, glam, anyhow, async-trait, log, uuid) plus
`blend = "0.7"`, minus `gltf`. Workspace `members = ["crates/*", …]` picks it up
automatically; also add it to the root `[dependencies]` list and register
`BlendPlugin` wherever `GLTFPlugin` is added.

## Scope of the first `.blend` version

**In:** static scenes — object hierarchy + transforms, mesh geometry
(positions / normals / UVs), basic PBR material factors, lights, cameras.
Enough to load a `.blend` the way we load a static-mesh glTF.

**Deferred (call out, don't attempt in v1):** armatures/skinning and animation
(Blender stores actions/F-curves and armature bones in a representation far less
direct than glTF's; extracting them from raw DNA is a separate, large effort),
textures/image-packing, modifiers (mirror/subsurf — bake in Blender first),
shader-node materials beyond the Principled BSDF base factors.

## Key risks / open questions

1. **Blender-version sensitivity (main risk).** The `blend` crate exposes raw
   DNA structs, and Blender's *mesh storage changed*: legacy `MVert`/`MPoly`/
   `MLoop`/`MLoopUV` arrays (≤ 2.7x-style) vs. modern generic **attribute /
   CustomData** layers (`position`, `corner_vert`, …) in 3.x/4.x. The field
   names the loader reads must match the version that authored the `.blend`.
   **Action:** pin the target Blender export version early, dump the DNA of a
   sample file, and write the extraction against those exact fields. Consider a
   small `xtask`/test that asserts the expected struct fields exist so a version
   bump fails loudly rather than silently producing empty meshes.
2. **wasm.** The glTF loader special-cases wasm (no filesystem → fetch bytes,
   `gltf::import_slice`). The `blend` loader should read bytes via
   `essential::assets::utils::load_binary` and `Blend::new(Cursor)` on **both**
   targets, so wasm needs no special path — but confirm the `blend` crate builds
   for `wasm32` (it's `std` + byte-slice parsing, so it should).
3. **Coordinate system.** Blender is Z-up; the engine (matching glTF) is Y-up.
   Decide whether to bake a Z-up→Y-up correction into the root transforms or
   require "+Y up" on export. glTF export from Blender already does this
   conversion, so matching that convention is the safe default.
4. **`warnings = "deny"`** at the workspace root — the new crates must be
   warning-clean (no unused `PhantomData` imports, etc.).

## Suggested landing order

1. **`crates/scene`** — trait, `SceneSpawner`, `SceneInstance`, generic system,
   `ScenePlugin`. Compiles standalone with no users yet.
2. **Refactor `gltf-loader`** onto `scene` + fix example call sites. Pure
   refactor, behavior-preserving — verify existing glTF examples still run.
3. **`crates/blend-loader`** — `BlendScene` + `BlendLoader` + `Scene` impl +
   `BlendPlugin`, static-scene scope. Validate against a known sample `.blend`
   exported from the pinned Blender version.

Steps 1–2 are independent of the `blend`-crate risk and can be reviewed first.
```
