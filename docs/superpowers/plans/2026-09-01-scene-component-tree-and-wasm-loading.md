# Generic Scene Component Tree & Cross-Platform Cooked Loading — Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Make `Scene` an arbitrary tree of entities and their serializable components — so skeletal animation, cameras, lights, and Blender-`extras` components all round-trip through a cooked glTF — and make cooked assets load on wasm as well as native.

**Architecture:** `SceneNode` stores `Vec<SerializedComponent { type_name, data }>` (serde-JSON) instead of typed fields. A serde-backed registry in `ecs` maps a type name to an erased "deserialize and apply" function. Each registered type implements `SceneComponent::apply(self, entity, ctx)`, which turns authoring data into runtime components — resolving asset handles, mapping node-index entity references, and expanding into several components where needed. Separately, a `CookedAssetRoot` on `AssetLoadContext` plus one shared `async` helper give the four cooked-format loaders a native/wasm-agnostic byte source.

**Tech Stack:** Rust, `serde` + `serde_json` (component payloads) and `bincode` (the `Scene` itself), `glam`/`uuid` serde features, `reqwest` (wasm HTTP), existing `gltf`/`tobj` crates in the offline importers.

**Spec:** `docs/superpowers/specs/2026-09-01-scene-component-tree-and-wasm-loading-design.md`

## Global Constraints

- No unnamed tuples as data types anywhere in new/modified code — named structs with named fields, including small internal helper types. Newtype wrappers with a single field (`SceneEntityRef(usize)`, `SceneSpawnerComponent(AssetHandle<Scene>)`) are exempt; they are the established pattern in this repo.
- No runtime fallback to parsing DCC source files. The runtime reads only cooked output.
- `SceneComponent: Component + DeserializeOwned + Sized + 'static`. The `Component` bound stays for now; whether a type inserts itself is decided by its `apply` body, never by the bound.
- There is exactly one component-registration function, `App::register_component::<T: SceneComponent>()`. `register_reflection` and the `facet` registry are removed.
- The `Component` trait gains no new method.
- `AssetHandle` is **not** modified. It stays a `Strong`/`Weak` enum with its existing manual serde impls.
- Component payloads are serde-JSON strings; the `Scene` asset itself stays bincode.
- CI gates that must stay green after every task: `cargo build --workspace` (zero warnings), `cargo test --workspace`, `cargo fmt --all -- --check`, `cargo clippy -- -A clippy::type_complexity -A clippy::too_many_arguments -D warnings`.
- Follow the existing test convention: plain `#[test]` functions (no async test runner — this codebase uses `pollster`/its own `TaskPool`, never `tokio`), integration tests in a crate-level `tests/` directory, `assert_eq!`/`assert!` with descriptive messages.
- Run `cargo fmt -p <crate>` on every crate touched before committing.
- Commit messages end with:
  `Co-Authored-By: Claude Opus 5 <noreply@anthropic.com>`

---

## Phase 1: Cross-Platform Cooked Loading

### Task 1: `CookedAssetRoot` and the shared byte-loading helper

**Files:**
- Modify: `crates/essential/src/assets/mod.rs`
- Modify: `crates/essential/src/assets/utils.rs`
- Test: `crates/essential/tests/cooked_asset_root.rs` (new)

**Interfaces:**
- Produces: `enum CookedAssetRoot { Directory(PathBuf), UrlBase(String) }` with `CookedAssetRoot::default_for_platform() -> Self`; `async fn load_cooked_asset_bytes(root: &CookedAssetRoot, id: AssetId) -> anyhow::Result<Vec<u8>>` in `essential::assets::utils`.
- Consumes: `AssetId::simple_hex()` (already present).

- [ ] **Step 1: Write the failing test**

Create `crates/essential/tests/cooked_asset_root.rs`:

```rust
//! Covers the runtime cooked-byte loader against a Directory root. The
//! `.cooked/<simple_hex>.bin` layout here must match what
//! `asset_cook::cooked_file_path_for_id` writes at cook time — if these two
//! ever disagree, every cooked asset fails to load at runtime.
use essential::assets::utils::load_cooked_asset_bytes;
use essential::assets::{AssetId, CookedAssetRoot};

#[test]
fn reads_cooked_bytes_from_a_directory_root() {
    let temp_dir = std::env::temp_dir().join(format!("cooked-root-{}-read", std::process::id()));
    let _ = std::fs::remove_dir_all(&temp_dir);
    std::fs::create_dir_all(temp_dir.join(".cooked")).unwrap();

    let id = AssetId::from_path("models/character.gltf#mesh/0");
    std::fs::write(
        temp_dir.join(".cooked").join(format!("{}.bin", id.simple_hex())),
        b"cooked-payload",
    )
    .unwrap();

    let root = CookedAssetRoot::Directory(temp_dir.clone());
    let bytes = pollster::block_on(load_cooked_asset_bytes(&root, id))
        .expect("a cooked file written at the ID-keyed path must be readable");

    assert_eq!(bytes, b"cooked-payload", "loader must return the cooked file's exact bytes");

    std::fs::remove_dir_all(&temp_dir).ok();
}

#[test]
fn missing_cooked_file_errors_with_the_resolved_path() {
    let temp_dir = std::env::temp_dir().join(format!("cooked-root-{}-missing", std::process::id()));
    let _ = std::fs::remove_dir_all(&temp_dir);
    std::fs::create_dir_all(&temp_dir).unwrap();

    let id = AssetId::from_path("models/absent.gltf#mesh/0");
    let root = CookedAssetRoot::Directory(temp_dir.clone());
    let err = pollster::block_on(load_cooked_asset_bytes(&root, id))
        .expect_err("a missing cooked file must be an error, not empty bytes");

    let message = format!("{err:#}");
    assert!(
        message.contains(&id.simple_hex()),
        "the error must name the resolved cooked path so a missing asset is traceable; got: {message}"
    );

    std::fs::remove_dir_all(&temp_dir).ok();
}
```

- [ ] **Step 2: Run test to verify it fails**

Run: `cargo test -p essential --test cooked_asset_root`
Expected: FAIL to compile — `CookedAssetRoot` and `load_cooked_asset_bytes` do not exist.

- [ ] **Step 3: Add `pollster` as a dev-dependency**

The test drives an `async fn` from a plain `#[test]`. `render` already depends on `pollster = "0.4.0"`; add the same version to `crates/essential/Cargo.toml`:

```toml
[dev-dependencies]
pollster = "0.4.0"
```

(If a `[dev-dependencies]` section already exists, add the line to it.)

- [ ] **Step 4: Add `CookedAssetRoot`**

In `crates/essential/src/assets/mod.rs`, add near `AssetPath`:

```rust
/// Where the runtime finds cooked asset files. The cooked layout is always
/// `<root>/.cooked/<asset-id-hex>.bin`; only the root differs per platform.
#[derive(Debug, Clone)]
pub enum CookedAssetRoot {
    /// Native: a directory containing `.cooked/`.
    Directory(PathBuf),
    /// wasm: a URL base, e.g. `"http://host/res"`.
    UrlBase(String),
}

impl CookedAssetRoot {
    /// Native: `<directory containing the executable>/res`, matching the
    /// convention the pre-cook `load_binary` used and what the examples'
    /// `build.rs` copies into place. wasm: `<page origin>/res`.
    pub fn default_for_platform() -> Self {
        cfg_if::cfg_if! {
            if #[cfg(target_arch = "wasm32")] {
                let origin = web_sys::window()
                    .and_then(|window| window.location().origin().ok())
                    .unwrap_or_default();
                CookedAssetRoot::UrlBase(format!("{origin}/res"))
            } else {
                let exe_dir = std::env::current_exe()
                    .ok()
                    .and_then(|exe| exe.parent().map(Path::to_path_buf))
                    .unwrap_or_else(|| PathBuf::from("."));
                CookedAssetRoot::Directory(exe_dir.join("res"))
            }
        }
    }
}

impl Default for CookedAssetRoot {
    fn default() -> Self {
        Self::default_for_platform()
    }
}
```

`cfg_if`, `Path`, and `PathBuf` are already imported in this file; add any that are missing.

- [ ] **Step 5: Replace the dead helpers with `load_cooked_asset_bytes`**

Rewrite `crates/essential/src/assets/utils.rs` entirely. `load_binary` and `load_to_string` have had zero callers since the cooked-format loaders landed; this replaces them with the one helper that does have callers, keeping their native/wasm branching structure:

```rust
use anyhow::Context;
use cfg_if::cfg_if;

use super::{AssetId, CookedAssetRoot};

/// Reads the cooked bytes for `id` from `root`. This is the runtime mirror of
/// `asset_cook::cooked_file_path_for_id` — both resolve an AssetId to
/// `.cooked/<simple-hex>.bin`, and they must not drift apart.
pub async fn load_cooked_asset_bytes(
    root: &CookedAssetRoot,
    id: AssetId,
) -> anyhow::Result<Vec<u8>> {
    let file_name = format!("{}.bin", id.simple_hex());

    match root {
        CookedAssetRoot::Directory(dir) => {
            let path = dir.join(".cooked").join(&file_name);
            std::fs::read(&path)
                .with_context(|| format!("failed to read cooked asset '{}'", path.display()))
        }
        CookedAssetRoot::UrlBase(base) => {
            cfg_if! {
                if #[cfg(target_arch = "wasm32")] {
                    let url = format!("{base}/.cooked/{file_name}");
                    let bytes = reqwest::get(&url)
                        .await
                        .with_context(|| format!("HTTP request for cooked asset '{url}' failed"))?
                        .bytes()
                        .await
                        .with_context(|| format!("failed to read response body for '{url}'"))?
                        .to_vec();
                    Ok(bytes)
                } else {
                    anyhow::bail!(
                        "CookedAssetRoot::UrlBase is only supported on wasm32 (base '{base}')"
                    )
                }
            }
        }
    }
}
```

- [ ] **Step 6: Run tests to verify they pass**

Run: `cargo test -p essential --test cooked_asset_root`
Expected: PASS (2 tests)

- [ ] **Step 7: Build and check the whole workspace**

Run: `cargo build --workspace && cargo test -p essential`
Expected: builds clean with zero warnings; all `essential` tests pass. Deleting `load_binary`/`load_to_string` must break nothing — grep to confirm before building: `grep -rn "load_binary\|load_to_string" --include=*.rs .` should return no hits outside the parked examples.

- [ ] **Step 8: Format and commit**

```bash
cargo fmt -p essential
cargo fmt --all -- --check
git add crates/essential
git commit -m "$(cat <<'EOF'
feat(essential): add CookedAssetRoot and a platform-agnostic cooked byte loader

Replaces the dead load_binary/load_to_string helpers with
load_cooked_asset_bytes, keeping their native/wasm branching but resolving
an AssetId to .cooked/<hex>.bin instead of a human asset path.

Co-Authored-By: Claude Opus 5 <noreply@anthropic.com>
EOF
)"
```

---

### Task 2: Route every cooked-format loader through the shared helper

**Files:**
- Modify: `crates/essential/src/assets/asset_server.rs`
- Modify: `crates/render/src/loaders/texture_loader.rs`
- Modify: `crates/render/src/assets/material.rs`
- Modify: `crates/mesh/src/mesh.rs`
- Modify: `crates/scene/src/scene.rs`
- Modify: `examples/render-test/src/main.rs`
- Create: `examples/render-test/build.rs`
- Modify: `examples/render-test/Cargo.toml`

**Interfaces:**
- Consumes: `CookedAssetRoot`, `load_cooked_asset_bytes` (Task 1).
- Produces: `AssetLoadContext::cooked_root(&self) -> &CookedAssetRoot`; `AssetServer` holds a `CookedAssetRoot` used for every load.

- [ ] **Step 1: Give `AssetServer` and `AssetLoadContext` a cooked root**

In `crates/essential/src/assets/asset_server.rs`:

Add the field to `AssetLoadContext` and an accessor beside the existing `asset_server()`/`asset_id()`:

```rust
pub struct AssetLoadContext {
    asset_server: AssetServer,
    asset_id: AssetId,
    cooked_root: CookedAssetRoot,
}

impl AssetLoadContext {
    pub fn cooked_root(&self) -> &CookedAssetRoot {
        &self.cooked_root
    }
}
```

Update `AssetLoadContext::new` to take and store the root, and update its single call site inside `request_load` to pass `server.cooked_root().clone()`.

Give `AssetServer` a `cooked_root: CookedAssetRoot` (stored in its existing shared `AssetServerData`, defaulting to `CookedAssetRoot::default_for_platform()`), plus:

```rust
impl AssetServer {
    pub fn cooked_root(&self) -> CookedAssetRoot { /* clone out of the shared data */ }
    pub fn set_cooked_root(&self, root: CookedAssetRoot) { /* store into the shared data */ }
}
```

**Verify during implementation:** read `AssetServer`'s current construction and interior-mutability pattern (`AssetServerData` behind an `Arc`, fields wrapped in `RwLock`) and follow it exactly rather than the sketch above — this is an additive field on existing working code.

- [ ] **Step 2: Rewrite the four loaders' byte access**

In each of the four loaders, replace the body's path-building and `std::fs::read` with the shared helper. `TextureLoader` (`crates/render/src/loaders/texture_loader.rs`) becomes:

```rust
let bytes =
    essential::assets::utils::load_cooked_asset_bytes(load_context.cooked_root(), load_context.asset_id())
        .await
        .with_context(|| "failed to read cooked texture")?;
let cooked: CookedTexture = bincode::deserialize(&bytes)
    .with_context(|| "failed to deserialize cooked texture")?;
Ok(Texture::from_cooked(cooked))
```

Apply the same shape to:
- `MeshLoader` in `crates/mesh/src/mesh.rs` ("cooked mesh")
- `StandardMaterialLoader` in `crates/render/src/assets/material.rs` ("cooked material") — keep its `material.resolve_asset_handles(load_context.asset_server());` call
- `SceneLoader` in `crates/scene/src/scene.rs` ("cooked scene") — keep its `scene.resolve_asset_handles(load_context.asset_server());` call

Delete from all four: the `asset_cook::cooked_file_path_for_id(std::path::Path::new("res"), …)` line and the `// TODO(follow-up): output root hard-coded as "res"` comment above it. Remove now-unused imports (`std::path::Path`, and `asset_cook` where nothing else in the file uses it — the compiler will flag them under `-D warnings`).

- [ ] **Step 3: Restore `render-test`'s asset copy and drop the CWD workaround**

Create `examples/render-test/build.rs`:

```rust
use std::env;
use std::path::{Path, PathBuf};

use fs_extra::copy_items;
use fs_extra::dir::CopyOptions;

/// Copies the cooked `res/` directory next to the built binary, so the
/// executable-relative `CookedAssetRoot::Directory` default finds it.
/// `res/` is produced by `cook`; if it does not exist yet, do nothing.
fn main() -> anyhow::Result<()> {
    let manifest_dir = PathBuf::from(env::var("CARGO_MANIFEST_DIR")?).canonicalize()?;
    let res_path = manifest_dir.join("res");
    println!("cargo:rerun-if-changed={}", res_path.display());

    if !res_path.exists() {
        return Ok(());
    }

    // OUT_DIR is target/<profile>/build/<pkg>-<hash>/out, so the profile
    // directory — where the binary lands — is three levels up.
    let out_dir = PathBuf::from(env::var("OUT_DIR")?);
    let output_path = out_dir.ancestors().nth(3).unwrap().to_path_buf();

    copy_items(
        &[res_path],
        Path::new(&output_path),
        &CopyOptions { overwrite: true, ..Default::default() },
    )?;

    Ok(())
}
```

Restore the build-dependencies in `examples/render-test/Cargo.toml`:

```toml
[build-dependencies]
anyhow = "1.0.97"
fs_extra = "1.3.0"
```

In `examples/render-test/src/main.rs`, delete the `std::env::set_current_dir(std::path::Path::new(env!("CARGO_MANIFEST_DIR")))` block (around line 88) and its `#[cfg(not(target_arch = "wasm32"))]` attribute — asset resolution is executable-relative again.

- [ ] **Step 4: Build and test**

Run: `cargo build --workspace && cargo test --workspace`
Expected: builds clean with zero warnings; all tests pass.

- [ ] **Step 5: Re-cook and run the example end-to-end**

```bash
cargo run -p cook -- examples/render-test/assets.toml examples/render-test/assets examples/render-test/res
cargo run -p render-test
```
Expected: cook reports `errors: 0`; the window opens and Sponza renders (dim at the committed point-light intensity of 10 — that is expected, not a regression). This proves the executable-relative root plus `build.rs` copy resolves cooked assets.

- [ ] **Step 6: Verify the wasm path**

This is the regression Phase 1 exists to fix, so it must be checked on wasm, not just native. Per the project's wasm workflow, examples run through `trunk serve`, not `cargo run`:

```bash
cd examples/render-test
trunk serve --open
```

The cooked `res/` directory must be served alongside the wasm bundle so that `CookedAssetRoot::UrlBase("<origin>/res")` resolves — add it to the example's `Trunk.toml`/`index.html` asset copy directives if it is not already served (check how the example previously served `res/`).

Expected: Sponza renders in the browser. Open the devtools network tab and confirm `GET /res/.cooked/<hex>.bin` requests return 200 — that is the wasm branch of `load_cooked_asset_bytes` doing its job. Before this task those requests did not exist and every load failed silently.

If `trunk` is not installed or the example has no wasm harness, record that the wasm path is unverified and say so explicitly in the task report rather than claiming it works.

- [ ] **Step 7: Format and commit**

```bash
cargo fmt -p essential -p render -p mesh -p scene
cargo fmt --all -- --check
git add crates/essential crates/render crates/mesh crates/scene examples/render-test
git commit -m "$(cat <<'EOF'
feat: load cooked assets through a platform-agnostic root

Threads CookedAssetRoot through AssetLoadContext and routes all four
cooked-format loaders through load_cooked_asset_bytes, removing the
CWD-relative hard-coded "res" path. render-test regains the build.rs
asset copy and drops its set_current_dir workaround.

Co-Authored-By: Claude Opus 5 <noreply@anthropic.com>
EOF
)"
```

---

## Phase 2: Generic Scene Component Tree

### Task 3: The `SceneComponent` interface in `ecs`

**Files:**
- Create: `crates/ecs/src/component/scene.rs`
- Modify: `crates/ecs/src/component/mod.rs`
- Modify: `crates/ecs/Cargo.toml`
- Test: `crates/ecs/tests/scene_component.rs` (new)

**Interfaces:**
- Produces: `SceneSpawnContext<'w>` with `insert<T: Component>(&mut self, T, Entity)`, `entity_for(&self, SceneEntityRef) -> Option<Entity>`, `world(&mut self) -> &mut RestrictedWorld<'w>`, and `SceneSpawnContext::new(world: RestrictedWorld<'w>, node_entities: &'w [Entity])`; `trait SceneComponent: Component + DeserializeOwned + Sized + 'static { fn apply(self, entity: Entity, ctx: &mut SceneSpawnContext<'_>); }`; `struct SceneEntityRef(pub usize)`.

- [ ] **Step 1: Write the failing test**

Create `crates/ecs/tests/scene_component.rs`:

```rust
//! Covers the two shapes SceneComponent must support: a type that inserts
//! itself, and a type that expands into other components (including onto a
//! different entity) without ever inserting one of itself.
use ecs::component::scene::{SceneComponent, SceneEntityRef, SceneSpawnContext};
use ecs::{Component, Entity, World};
use serde::{Deserialize, Serialize};

#[derive(Component, Serialize, Deserialize, PartialEq, Debug)]
struct PlainMarker {
    value: u32,
}

impl SceneComponent for PlainMarker {
    fn apply(self, entity: Entity, ctx: &mut SceneSpawnContext<'_>) {
        ctx.insert(self, entity);
    }
}

#[derive(Component, Serialize, Deserialize)]
struct ExpandingAuthoringData {
    target: SceneEntityRef,
}

impl SceneComponent for ExpandingAuthoringData {
    fn apply(self, _entity: Entity, ctx: &mut SceneSpawnContext<'_>) {
        // Deliberately inserts onto a *different* entity and never inserts
        // one of itself.
        if let Some(other) = ctx.entity_for(self.target) {
            ctx.insert(PlainMarker { value: 99 }, other);
        }
    }
}

#[test]
fn apply_can_insert_self() {
    let mut world = World::default();
    let entity = world.spawn(());
    let nodes = [entity];

    {
        let mut ctx = SceneSpawnContext::new((&mut world).into(), &nodes);
        PlainMarker { value: 7 }.apply(entity, &mut ctx);
    }

    assert_eq!(
        world.get_component_for_entity::<PlainMarker>(entity),
        Some(&PlainMarker { value: 7 }),
        "a SceneComponent that inserts itself must land on the entity"
    );
}

#[test]
fn apply_can_expand_onto_another_entity_without_inserting_itself() {
    let mut world = World::default();
    let owner = world.spawn(());
    let other = world.spawn(());
    let nodes = [owner, other];

    {
        let mut ctx = SceneSpawnContext::new((&mut world).into(), &nodes);
        ExpandingAuthoringData { target: SceneEntityRef(1) }.apply(owner, &mut ctx);
    }

    assert_eq!(
        world.get_component_for_entity::<PlainMarker>(other),
        Some(&PlainMarker { value: 99 }),
        "expansion must be able to write to an entity other than its own"
    );
    assert!(
        world.get_component_for_entity::<ExpandingAuthoringData>(owner).is_none(),
        "authoring data that never inserts itself must leave nothing on its own entity"
    );
}

#[test]
fn entity_for_returns_none_for_an_out_of_range_ref() {
    let mut world = World::default();
    let entity = world.spawn(());
    let nodes = [entity];

    let mut ctx = SceneSpawnContext::new((&mut world).into(), &nodes);
    assert!(
        ctx.entity_for(SceneEntityRef(5)).is_none(),
        "a malformed cooked scene must yield None, never a panic"
    );
}
```

**Verify during implementation:** the exact `World` construction and spawn API (`World::default()`, `world.spawn(())`, `get_component_for_entity`) — read `crates/ecs/src/world.rs` and adjust the test to the real signatures before implementing. The assertions are the contract; the setup calls are whatever `World` actually offers.

- [ ] **Step 2: Run test to verify it fails**

Run: `cargo test -p ecs --test scene_component`
Expected: FAIL to compile — `ecs::component::scene` does not exist.

- [ ] **Step 3: Add serde to `ecs`**

In `crates/ecs/Cargo.toml` `[dependencies]`, add:

```toml
serde = { version = "1", features = ["derive"] }
serde_json = "1.0"
```

Leave `facet`/`facet-json` in place for now; Task 4 removes them.

- [ ] **Step 4: Create the `SceneComponent` module**

Create `crates/ecs/src/component/scene.rs`:

```rust
use serde::de::DeserializeOwned;
use serde::{Deserialize, Serialize};

use crate::{component::Component, entity::Entity, world::RestrictedWorld};

/// A reference to another node of the same `Scene`, by node index. Resolved
/// to a real [`Entity`] during spawning via [`SceneSpawnContext::entity_for`].
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct SceneEntityRef(pub usize);

/// What a [`SceneComponent`] gets while a cooked scene is being spawned.
pub struct SceneSpawnContext<'w> {
    world: RestrictedWorld<'w>,
    node_entities: &'w [Entity],
}

impl<'w> SceneSpawnContext<'w> {
    pub fn new(world: RestrictedWorld<'w>, node_entities: &'w [Entity]) -> Self {
        Self { world, node_entities }
    }

    /// Adds a runtime component to any entity in the scene being spawned —
    /// not necessarily the one currently being applied to.
    pub fn insert<T: Component>(&mut self, component: T, entity: Entity) {
        self.world.insert(component, entity, true);
    }

    /// Resolves a node reference to its spawned entity. Returns `None` for an
    /// out-of-range index, so a malformed cooked scene cannot panic.
    pub fn entity_for(&self, reference: SceneEntityRef) -> Option<Entity> {
        self.node_entities.get(reference.0).copied()
    }

    /// Escape hatch for resources — notably `AssetServer`, which `ecs` cannot
    /// name.
    pub fn world(&mut self) -> &mut RestrictedWorld<'w> {
        &mut self.world
    }
}

/// Data authored into a cooked `Scene` that knows how to apply itself to a
/// spawned entity.
///
/// A type that is a runtime component inserts itself. A type that is really
/// authoring data expands into several runtime components — possibly on other
/// entities — and never inserts one of itself. Both are this one interface.
pub trait SceneComponent: Component + DeserializeOwned + Sized + 'static {
    fn apply(self, entity: Entity, ctx: &mut SceneSpawnContext<'_>);
}
```

In `crates/ecs/src/component/mod.rs`, add `pub mod scene;` beside the existing `pub mod bundle;` / `pub mod reflection;` declarations.

- [ ] **Step 5: Run tests to verify they pass**

Run: `cargo test -p ecs --test scene_component`
Expected: PASS (3 tests)

- [ ] **Step 6: Build and format**

Run: `cargo build --workspace && cargo test -p ecs && cargo fmt -p ecs && cargo fmt --all -- --check`
Expected: builds clean, all `ecs` tests pass, formatting clean.

- [ ] **Step 7: Commit**

```bash
git add crates/ecs
git commit -m "$(cat <<'EOF'
feat(ecs): add the SceneComponent interface

SceneComponent::apply turns cooked scene data into runtime components. It can
insert self, resolve node references via SceneSpawnContext::entity_for, and
expand onto other entities -- which is what skeleton data needs.

Co-Authored-By: Claude Opus 5 <noreply@anthropic.com>
EOF
)"
```

---

### Task 4: Replace the facet registry with a serde one

**Files:**
- Modify: `crates/ecs/src/component/registry.rs`
- Delete: `crates/ecs/src/component/reflection.rs`
- Modify: `crates/ecs/src/component/mod.rs`
- Modify: `crates/ecs/src/world.rs`
- Modify: `crates/ecs/src/command.rs`
- Modify: `crates/ecs/Cargo.toml`
- Modify: `crates/app/src/lib.rs`
- Modify: `crates/app/src/subapp.rs`
- Modify: `crates/app/Cargo.toml`
- Modify: `crates/director/src/virtual_camera.rs`
- Modify: `crates/director/src/lib.rs`
- Modify: `crates/physics/src/shape.rs`
- Modify: `crates/physics/src/plugin.rs`
- Test: `crates/ecs/tests/component_registry.rs` (new)

**Interfaces:**
- Consumes: `SceneComponent`, `SceneSpawnContext`, `SceneEntityRef` (Task 3).
- Produces: `App::register_component::<T: SceneComponent>()`, `SubApp::register_component`, `World::register_component_type::<T: SceneComponent>()`; `World::apply_scene_component(&mut self, type_name: &str, json: &str, entity: Entity, node_entities: &[Entity]) -> bool` (returns whether a type was registered under that name); `CommandQueue::insert_from_json(type_name: String, data: String, entity: Entity)` unchanged in signature.

- [ ] **Step 1: Write the failing test**

Create `crates/ecs/tests/component_registry.rs`:

```rust
//! Covers the serde component registry: a registered type deserializes from
//! JSON and is applied; an unregistered type name is reported rather than
//! panicking.
use ecs::component::scene::{SceneComponent, SceneSpawnContext};
use ecs::{Component, Entity, World};
use serde::{Deserialize, Serialize};

#[derive(Component, Serialize, Deserialize, PartialEq, Debug)]
struct Registered {
    value: u32,
}

impl SceneComponent for Registered {
    fn apply(self, entity: Entity, ctx: &mut SceneSpawnContext<'_>) {
        ctx.insert(self, entity);
    }
}

#[test]
fn registered_component_is_deserialized_and_applied() {
    let mut world = World::default();
    world.register_component_type::<Registered>();
    let entity = world.spawn(());

    let applied = world.apply_scene_component("Registered", r#"{"value":42}"#, entity, &[entity]);

    assert!(applied, "a registered type name must be found in the registry");
    assert_eq!(
        world.get_component_for_entity::<Registered>(entity),
        Some(&Registered { value: 42 }),
        "the JSON payload must be deserialized into the real component and inserted"
    );
}

#[test]
fn unregistered_component_name_is_reported_not_fatal() {
    let mut world = World::default();
    let entity = world.spawn(());

    let applied = world.apply_scene_component("NeverRegistered", "{}", entity, &[entity]);

    assert!(!applied, "an unregistered type name must report false rather than panicking");
}

#[test]
fn malformed_json_does_not_panic() {
    let mut world = World::default();
    world.register_component_type::<Registered>();
    let entity = world.spawn(());

    let applied = world.apply_scene_component("Registered", "{ not json", entity, &[entity]);

    assert!(!applied, "malformed payloads must be skipped, not propagated as a panic");
    assert!(
        world.get_component_for_entity::<Registered>(entity).is_none(),
        "a failed deserialize must not leave a partial component behind"
    );
}
```

- [ ] **Step 2: Run test to verify it fails**

Run: `cargo test -p ecs --test component_registry`
Expected: FAIL to compile — `register_component_type`/`apply_scene_component` do not exist.

- [ ] **Step 3: Rewrite the registry**

Replace the facet parts of `crates/ecs/src/component/registry.rs`. Delete the `use facet::Facet;` import, the `reflection_map` field's `ComponentReflection` value type, `register_refection`, and `get_reflection`; add:

```rust
use crate::component::scene::{SceneComponent, SceneSpawnContext};

/// Deserializes a JSON payload into `T` and applies it. Returns `Err` with a
/// human-readable reason when the payload does not parse.
type ErasedApply =
    fn(&str, Entity, &mut SceneSpawnContext<'_>) -> Result<(), serde_json::Error>;

fn apply_typed<T: SceneComponent>(
    json: &str,
    entity: Entity,
    ctx: &mut SceneSpawnContext<'_>,
) -> Result<(), serde_json::Error> {
    let value: T = serde_json::from_str(json)?;
    value.apply(entity, ctx);
    Ok(())
}
```

Change the registry field to `scene_component_map: HashMap<&'static str, ErasedApply>` and replace `register_refection` with:

```rust
pub(crate) fn register_scene_component<T: SceneComponent>(&mut self) {
    self.register_component::<T>();
    self.scene_component_map.insert(T::name(), apply_typed::<T>);
}

pub(crate) fn get_scene_component(&self, name: &str) -> Option<ErasedApply> {
    self.scene_component_map.get(name).copied()
}
```

Delete `crates/ecs/src/component/reflection.rs` and its `pub mod reflection;` declaration in `crates/ecs/src/component/mod.rs`.

- [ ] **Step 4: Wire it into `World`**

In `crates/ecs/src/world.rs`, replace `register_reflection` and `get_reflection` with:

```rust
pub fn register_component_type<T: SceneComponent>(&mut self) {
    self.component_registry.register_scene_component::<T>();
}

/// Deserializes `json` into the component registered under `type_name` and
/// applies it to `entity`. Returns `false` (having logged why) when the name
/// is unregistered or the payload does not parse — a cooked scene may carry
/// components this application does not know about.
pub fn apply_scene_component(
    &mut self,
    type_name: &str,
    json: &str,
    entity: Entity,
    node_entities: &[Entity],
) -> bool {
    let Some(apply) = self.component_registry.get_scene_component(type_name) else {
        log::warn!(
            "Skipping component '{type_name}': no type registered under that name \
             (register it with App::register_component)"
        );
        return false;
    };

    let mut ctx = SceneSpawnContext::new(RestrictedWorld::from(self), node_entities);
    match apply(json, entity, &mut ctx) {
        Ok(()) => true,
        Err(err) => {
            log::warn!("Failed to deserialize component '{type_name}' from `{json}`: {err}");
            false
        }
    }
}
```

**Verify during implementation:** `RestrictedWorld::from(&mut World)` borrows `self` mutably for the duration of `ctx`. Scope the borrow so it ends before `apply_scene_component` returns, and confirm the existing `impl<'w> From<&'w mut World> for RestrictedWorld<'w>` is what the borrow checker accepts here.

- [ ] **Step 5: Re-back `InsertErasedCommand` with the registry**

In `crates/ecs/src/command.rs`, replace the whole facet body of `impl Command for InsertErasedCommand` with:

```rust
impl Command for InsertErasedCommand {
    fn execute(self: Box<Self>, world: &mut World) {
        world.apply_scene_component(&self.component_name, &self.component_data, self.entity, &[]);
    }
}
```

`CommandQueue::insert_from_json` keeps its signature. Non-scene callers get an empty `node_entities` slice, so any `SceneEntityRef` they carry resolves to `None`. Remove the now-unused `facet_json` import.

- [ ] **Step 6: Rename the `App`/`SubApp` entry points**

In `crates/app/src/lib.rs` (line ~185) and `crates/app/src/subapp.rs` (line ~92), replace `register_reflection` with:

```rust
pub fn register_component<T: SceneComponent>(&mut self) -> &mut Self {
    self.main_mut().register_component::<T>();   // SubApp: self.world.register_component_type::<T>();
    self
}
```

Remove `use facet::Facet;` from both files and drop `facet` from `crates/app/Cargo.toml`.

- [ ] **Step 7: Migrate the three live call sites**

`register_reflection` has three live call sites outside the parked examples. Each type swaps its `Facet` derive for serde plus a `SceneComponent` impl.

`crates/physics/src/shape.rs` — `MeshCollider` is a unit struct:

```rust
#[derive(Component, Serialize, Deserialize)]
pub struct MeshCollider;

impl SceneComponent for MeshCollider {
    fn apply(self, entity: Entity, ctx: &mut SceneSpawnContext<'_>) {
        ctx.insert(self, entity);
    }
}
```

Update `crates/physics/src/plugin.rs:24` to `app.register_component::<MeshCollider>();`. Add `serde = { version = "1", features = ["derive"] }` to `crates/physics/Cargo.toml` if absent.

`crates/director/src/virtual_camera.rs` — `VirtualCamera { priority: i32, enabled: bool, lens: Option<Lens> }`. Its `#[facet(default = true)]` on `enabled` becomes a serde default:

```rust
#[derive(Component, Serialize, Deserialize)]
pub struct VirtualCamera {
    priority: i32,
    #[serde(default = "default_enabled")]
    enabled: bool,
    pub lens: Option<Lens>,
}

fn default_enabled() -> bool {
    true
}

impl SceneComponent for VirtualCamera {
    fn apply(self, entity: Entity, ctx: &mut SceneSpawnContext<'_>) {
        ctx.insert(self, entity);
    }
}
```

`Lens` and anything it contains need `Serialize`/`Deserialize` derives too — the compiler names each one; add derives until it is satisfied. Update `crates/director/src/lib.rs:56` and `crates/director/src/virtual_camera.rs:243` to `register_component`. Add `serde` to `crates/director/Cargo.toml` if absent.

`examples/tech-demo/src/main.rs:41` also calls `register_reflection`, but tech-demo is excluded from the workspace; Task 11 migrates it.

- [ ] **Step 8: Drop the facet dependencies**

Remove `facet` and `facet-json` from `crates/ecs/Cargo.toml`. Grep to confirm nothing references them: `grep -rn "facet" --include=*.rs --include=Cargo.toml crates/ | grep -v "^crates/.*examples"` should be empty.

- [ ] **Step 9: Run tests and build**

Run: `cargo test -p ecs --test component_registry && cargo build --workspace && cargo test --workspace`
Expected: 3 registry tests pass; builds clean with zero warnings; all workspace tests pass.

- [ ] **Step 10: Format and commit**

```bash
cargo fmt -p ecs -p app -p director -p physics
cargo fmt --all -- --check
git add crates/ecs crates/app crates/director crates/physics
git commit -m "$(cat <<'EOF'
refactor(ecs)!: replace the facet reflection registry with a serde one

facet cannot carry the generic Scene design -- glam types are not Facet and
the orphan rule blocks adding it -- so Transform could never round-trip.
register_reflection becomes register_component::<T: SceneComponent>, backed
by serde_json. VirtualCamera and MeshCollider migrate; facet/facet-json drop
out of ecs and app.

Co-Authored-By: Claude Opus 5 <noreply@anthropic.com>
EOF
)"
```

---

### Task 5: `SceneComponent` impls for the engine's own components

**Files:**
- Modify: `crates/color/src/lib.rs`, `crates/color/src/srgba.rs`, `crates/color/src/hsl.rs`
- Modify: `crates/essential/src/transform/mod.rs`
- Modify: `crates/mesh/src/mesh.rs`
- Modify: `crates/render/src/components/material.rs`
- Modify: `crates/render/src/components/camera.rs`
- Modify: `crates/render/src/components/light.rs`
- Modify: `crates/render/src/components/render_entity.rs`
- Modify: `crates/mesh/Cargo.toml`, `crates/render/Cargo.toml`
- Test: `crates/render/tests/scene_component_impls.rs` (new)

**Interfaces:**
- Consumes: `SceneComponent`, `SceneSpawnContext` (Task 3).
- Produces: `impl SceneComponent` for `Transform`, `MeshComponent`, `MaterialComponent`, `Camera`, `Light`, `SyncWithRenderWorld`. `MeshComponent` and `MaterialComponent` upgrade their handle from `Weak` to `Strong` via `AssetServer::load_by_id`; `Camera` upgrades a `RenderTarget::Texture` handle.

- [ ] **Step 1: Write the failing test**

Create `crates/render/tests/scene_component_impls.rs`:

```rust
//! Covers that a handle-bearing component's `apply` upgrades its Weak handle
//! against the AssetServer rather than inserting an unresolved one — the
//! property that makes cooked meshes actually load.
use ecs::component::scene::{SceneComponent, SceneSpawnContext};
use ecs::{Entity, World};
use essential::assets::{asset_server::AssetServer, handle::AssetHandle, AssetId};
use mesh::mesh::{Mesh, MeshComponent};

#[test]
fn mesh_component_apply_inserts_a_resolved_handle() {
    let mut world = World::default();
    world.insert_resource(AssetServer::default());
    world.register_component_type::<MeshComponent>();
    let entity = world.spawn(());

    let id = AssetId::from_path("models/character.gltf#mesh/0");
    let component = MeshComponent { handle: AssetHandle::<Mesh>::weak(id) };

    {
        let nodes = [entity];
        let mut ctx = SceneSpawnContext::new((&mut world).into(), &nodes);
        component.apply(entity, &mut ctx);
    }

    let inserted = world
        .get_component_for_entity::<MeshComponent>(entity)
        .expect("MeshComponent::apply must insert the component");
    assert_eq!(
        inserted.handle.id(),
        id,
        "resolving must preserve the AssetId the cooked scene referenced"
    );
}

#[test]
fn transform_apply_inserts_itself_unchanged() {
    use essential::transform::Transform;
    use glam::Vec3;

    let mut world = World::default();
    world.register_component_type::<Transform>();
    let entity = world.spawn(());

    let mut transform = Transform::IDENTITY;
    transform.translation = Vec3::new(1.0, 2.0, 3.0);

    {
        let nodes = [entity];
        let mut ctx = SceneSpawnContext::new((&mut world).into(), &nodes);
        transform.clone().apply(entity, &mut ctx);
    }

    let inserted = world
        .get_component_for_entity::<Transform>(entity)
        .expect("Transform::apply must insert the component");
    assert_eq!(
        inserted.translation,
        Vec3::new(1.0, 2.0, 3.0),
        "a plain component must be inserted with its data intact"
    );
}
```

**Verify during implementation:** `AssetServer::default()` and `World::insert_resource` — read the real constructors; `AssetServer` may need building through its plugin. If constructing a bare `AssetServer` in a unit test is impractical, assert instead that `apply` inserts a component whose `handle.id()` matches, with no `AssetServer` resource present (the impls fall through to leaving the handle weak when the resource is absent). Keep the id assertion either way.

- [ ] **Step 2: Run test to verify it fails**

Run: `cargo test -p render --test scene_component_impls`
Expected: FAIL to compile — no `SceneComponent` impls exist.

- [ ] **Step 3: Give `Color` full serde coverage**

`Camera` and `Light` hold `Color`, an enum over `LinearRgba`/`Srgba`/`Hsla`. Only `LinearRgba` gained serde (in the pipeline work). Add `Serialize, Deserialize` to the derive lists of `Srgba` (`crates/color/src/srgba.rs`), `Hsla` (`crates/color/src/hsl.rs`), and `Color` (`crates/color/src/lib.rs`). `crates/color` already has the `serde` dependency.

- [ ] **Step 4: Add derives and impls**

Add `Serialize, Deserialize` derives where missing, then one `SceneComponent` impl each.

`crates/essential/src/transform/mod.rs` — `Transform` already derives serde:

```rust
impl SceneComponent for Transform {
    fn apply(self, entity: Entity, ctx: &mut SceneSpawnContext<'_>) {
        ctx.insert(self, entity);
    }
}
```

`crates/mesh/src/mesh.rs` — add `#[derive(Serialize, Deserialize)]` to `MeshComponent`, then:

```rust
impl SceneComponent for MeshComponent {
    fn apply(mut self, entity: Entity, ctx: &mut SceneSpawnContext<'_>) {
        if let Some(server) = ctx.world().get_resource::<AssetServer>() {
            self.handle = server.load_by_id(self.handle.id());
        }
        ctx.insert(self, entity);
    }
}
```

`crates/render/src/components/material.rs` — add serde derives to `MaterialComponent<M>` and the identical `apply` (its field is also named `handle`). The generic parameter needs `M: Material + Send + Sync + 'static`; keep the existing bounds and add whatever serde requires.

`crates/render/src/components/render_entity.rs` — `SyncWithRenderWorld` is a unit struct: add serde derives and an insert-self `apply`.

`crates/render/src/components/light.rs` — add serde derives to `Light` and `LightType`, and an insert-self `apply`.

`crates/render/src/components/camera.rs` — add serde derives to `Camera` and `RenderTarget`, then:

```rust
impl SceneComponent for Camera {
    fn apply(mut self, entity: Entity, ctx: &mut SceneSpawnContext<'_>) {
        if let RenderTarget::Texture(handle) = &self.render_target {
            if let Some(server) = ctx.world().get_resource::<AssetServer>() {
                self.render_target = RenderTarget::Texture(server.load_by_id(handle.id()));
            }
        }
        ctx.insert(self, entity);
    }
}
```

Add `serde = { version = "1", features = ["derive"] }` to `crates/mesh/Cargo.toml` and `crates/render/Cargo.toml` if absent (both already have it from the pipeline work — verify rather than assume).

- [ ] **Step 5: Register them in `ScenePlugin`**

In `crates/scene/src/plugin.rs`, register every component the scene spawner can encounter, keeping the existing comment about not re-registering assets:

```rust
app.register_component::<Transform>();
app.register_component::<MeshComponent>();
app.register_component::<MaterialComponent>();
app.register_component::<Camera>();
app.register_component::<Light>();
app.register_component::<SyncWithRenderWorld>();
```

Add the corresponding `use` lines and any missing path dependencies to `crates/scene/Cargo.toml`.

- [ ] **Step 6: Run tests and build**

Run: `cargo test -p render --test scene_component_impls && cargo build --workspace && cargo test --workspace`
Expected: 2 tests pass; builds clean; all workspace tests pass.

- [ ] **Step 7: Format and commit**

```bash
cargo fmt -p color -p essential -p mesh -p render -p scene
cargo fmt --all -- --check
git add crates/color crates/essential crates/mesh crates/render crates/scene
git commit -m "$(cat <<'EOF'
feat: implement SceneComponent for the engine's spawnable components

Transform, MeshComponent, MaterialComponent, Camera, Light and
SyncWithRenderWorld gain serde derives and an apply impl; the handle-bearing
ones upgrade Weak handles through the AssetServer. Srgba/Hsla/Color gain the
serde coverage Camera and Light need.

Co-Authored-By: Claude Opus 5 <noreply@anthropic.com>
EOF
)"
```

---

### Task 6: Switch `Scene` to a component list

This is the largest task in the plan: `SceneNode`'s shape changes, so both importers and the spawner must change with it or the workspace will not build. There is no smaller green intermediate. If it proves too large in one sitting, split at the importer boundary — get `Scene` + spawner + `GltfImporter` compiling first, then `ObjImporter`.

**Files:**
- Modify: `crates/scene/src/scene.rs`
- Modify: `crates/scene/src/spawner.rs`
- Modify: `crates/gltf-loader/src/gltf_importer.rs`
- Modify: `crates/obj-loader/src/obj_importer.rs`
- Modify: `crates/asset-cook/src/cook.rs`
- Modify: `crates/scene/tests/scene_serialization.rs`
- Modify: `crates/gltf-loader/tests/gltf_importer.rs`
- Modify: `crates/obj-loader/tests/obj_importer.rs`
- Test: `crates/scene/tests/spawn_scene.rs` (new)

**Interfaces:**
- Consumes: `SceneComponent`, `SceneSpawnContext` (Task 3); `World::apply_scene_component` (Task 4); the component impls (Task 5).
- Produces: `struct SerializedComponent { type_name: String, data: String }`; `struct SceneNode { name: String, children: Vec<usize>, components: Vec<SerializedComponent> }`; `struct Scene { nodes: Vec<SceneNode>, referenced_assets: Vec<AssetId> }`; `SceneNode::push_component<T: Serialize + Component>(&mut self, &T) -> anyhow::Result<()>` for importers.
- `Scene::resolve_asset_handles` is **removed** — handle resolution now happens per component in `apply`.

- [ ] **Step 1: Write the failing integration test**

Create `crates/scene/tests/spawn_scene.rs`. This closes the spawner coverage gap the pipeline branch's final review flagged:

```rust
//! Covers spawn_scene_components end-to-end: one entity per node, components
//! deserialized and applied from their JSON payloads, hierarchy wired, and
//! the spawner component removed so the scene expands exactly once.
use ecs::{Entity, World};
use essential::transform::Transform;
use glam::Vec3;
use mesh::mesh::MeshComponent;
use scene::scene::{Scene, SceneNode, SerializedComponent};

fn node(name: &str, children: Vec<usize>, components: Vec<SerializedComponent>) -> SceneNode {
    SceneNode { name: name.to_string(), children, components }
}

fn transform_component(x: f32) -> SerializedComponent {
    let mut transform = Transform::IDENTITY;
    transform.translation = Vec3::new(x, 0.0, 0.0);
    SerializedComponent {
        type_name: "Transform".to_string(),
        data: serde_json::to_string(&transform).unwrap(),
    }
}

#[test]
fn scene_round_trips_through_bincode_with_component_payloads() {
    let scene = Scene {
        nodes: vec![
            node("root", vec![1], vec![transform_component(0.0)]),
            node("child", vec![], vec![transform_component(5.0)]),
        ],
        referenced_assets: vec![],
    };

    let bytes = bincode::serialize(&scene).expect("Scene must serialize");
    let decoded: Scene = bincode::deserialize(&bytes).expect("Scene must round-trip");

    assert_eq!(decoded.nodes.len(), 2, "both nodes must survive the round-trip");
    assert_eq!(decoded.nodes[0].children, vec![1], "hierarchy indices must survive");
    assert_eq!(
        decoded.nodes[1].components[0].type_name, "Transform",
        "component type names must survive"
    );

    let transform: Transform =
        serde_json::from_str(&decoded.nodes[1].components[0].data).expect("payload must parse");
    assert_eq!(
        transform.translation,
        Vec3::new(5.0, 0.0, 0.0),
        "the component payload must carry its real data"
    );
}

#[test]
fn unregistered_component_is_skipped_without_failing_the_node() {
    let mut world = World::default();
    world.register_component_type::<Transform>();
    let entity = world.spawn(());

    let applied_unknown =
        world.apply_scene_component("NotARegisteredType", "{}", entity, &[entity]);
    let applied_known = world.apply_scene_component(
        "Transform",
        &serde_json::to_string(&Transform::IDENTITY).unwrap(),
        entity,
        &[entity],
    );

    assert!(!applied_unknown, "an unknown component must be skipped");
    assert!(applied_known, "a known component on the same node must still apply");
    assert!(
        world.get_component_for_entity::<Transform>(entity).is_some(),
        "skipping one component must not abort the rest of the node"
    );
}
```

**Verify during implementation:** driving the full `spawn_scene_components` system needs a `World` with an `AssetStore<Scene>` resource and a scheduled system run. If wiring the scheduler in a test proves impractical, keep the two tests above (which cover serialization and the registry path) and additionally extract the spawner's node-walking into a testable free function `expand_scene(scene: &Scene, spawner: Entity, cmd: &mut CommandQueue) -> Vec<Entity>` and test that directly. Do not skip spawner coverage — it is a required deliverable of this task.

- [ ] **Step 2: Run test to verify it fails**

Run: `cargo test -p scene --test spawn_scene`
Expected: FAIL to compile — `SerializedComponent` does not exist and `SceneNode` has different fields.

- [ ] **Step 3: Change the `Scene` types**

In `crates/scene/src/scene.rs`, replace `SceneNode` and `Scene`:

```rust
/// One component's cooked payload: the registry key it was registered under
/// plus its serde-JSON encoding.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SerializedComponent {
    pub type_name: String,
    pub data: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SceneNode {
    pub name: String,
    /// Indices into `Scene::nodes`.
    pub children: Vec<usize>,
    pub components: Vec<SerializedComponent>,
}

impl SceneNode {
    /// Serializes `component` and appends it to this node. Importers use this
    /// rather than building `SerializedComponent` by hand, so the registry key
    /// always comes from `Component::name()`.
    pub fn push_component<T: Serialize + Component>(
        &mut self,
        component: &T,
    ) -> anyhow::Result<()> {
        self.components.push(SerializedComponent {
            type_name: T::name().to_string(),
            data: serde_json::to_string(component)?,
        });
        Ok(())
    }
}

#[derive(Asset, Debug, Clone, Serialize, Deserialize)]
pub struct Scene {
    pub nodes: Vec<SceneNode>,
    /// Every AssetId reachable from the nodes' components. Component payloads
    /// are opaque strings, so `referenced_sub_assets` cannot introspect them —
    /// the importer records the ids here as it emits.
    pub referenced_assets: Vec<AssetId>,
}
```

Update `impl CookedAsset for Scene`:

```rust
impl CookedAsset for Scene {
    const TYPE_NAME: &'static str = "Scene";

    fn referenced_sub_assets(&self) -> Vec<AssetId> {
        self.referenced_assets.clone()
    }
}
```

Delete `Scene::resolve_asset_handles` and its call in `SceneLoader::load` — resolution is per component now. Add `serde_json` to `crates/scene/Cargo.toml`.

- [ ] **Step 4: Rewrite the spawner**

Replace the body of `spawn_scene_components` in `crates/scene/src/spawner.rs`. The typed mesh/material/`SyncWithRenderWorld` special-casing goes away; components are applied generically:

```rust
pub fn spawn_scene_components(
    mut cmd: CommandQueue,
    spawners: Query<(Entity, &SceneSpawnerComponent, Option<&Transform>)>,
    scenes: Res<AssetStore<Scene>>,
) {
    for (spawner_entity, spawner, spawner_transform) in spawners.iter() {
        let Some(scene) = scenes.get(&spawner.0) else {
            continue;
        };

        if spawner_transform.is_none() {
            cmd.insert(Transform::IDENTITY, spawner_entity);
        }

        let mut node_entities = Vec::with_capacity(scene.nodes.len());
        for _ in &scene.nodes {
            node_entities.push(cmd.spawn(()).entity());
        }

        // Every queued command needs the node map; share one allocation
        // rather than cloning a Vec per component (Sponza queues hundreds).
        let shared_nodes: std::sync::Arc<[Entity]> = node_entities.clone().into();

        for (index, node) in scene.nodes.iter().enumerate() {
            for component in &node.components {
                cmd.apply_scene_component(
                    component.type_name.clone(),
                    component.data.clone(),
                    node_entities[index],
                    shared_nodes.clone(),
                );
            }
        }

        let mut has_parent = vec![false; scene.nodes.len()];
        for (index, node) in scene.nodes.iter().enumerate() {
            for &child in &node.children {
                // A malformed cooked Scene can carry an out-of-range child
                // index; skip it rather than panicking on the main schedule.
                let Some(&child_entity) = node_entities.get(child) else {
                    continue;
                };
                cmd.add_child(node_entities[index], child_entity);
                has_parent[child] = true;
            }
        }
        for (index, node_entity) in node_entities.iter().enumerate() {
            if !has_parent[index] {
                cmd.add_child(spawner_entity, *node_entity);
            }
        }

        cmd.remove::<SceneSpawnerComponent>(spawner_entity);
    }
}
```

Nodes now spawn with no components (`cmd.spawn(())`) because `Transform` arrives as a component payload like everything else.

This needs a new queued command. Add to `crates/ecs/src/command.rs`, beside `insert_from_json`:

```rust
impl CommandQueue {
    /// Queues a cooked scene component for deserialization and application.
    /// `node_entities` lets the component resolve `SceneEntityRef`s.
    pub fn apply_scene_component(
        &mut self,
        type_name: String,
        data: String,
        entity: Entity,
        node_entities: std::sync::Arc<[Entity]>,
    ) {
        self.queue_state.add_command(ApplySceneComponentCommand {
            type_name,
            data,
            entity,
            node_entities,
        });
    }
}

pub(crate) struct ApplySceneComponentCommand {
    type_name: String,
    data: String,
    entity: Entity,
    node_entities: std::sync::Arc<[Entity]>,
}

impl Command for ApplySceneComponentCommand {
    fn execute(self: Box<Self>, world: &mut World) {
        world.apply_scene_component(&self.type_name, &self.data, self.entity, &self.node_entities);
    }
}
```

**Verify during implementation:** confirm `cmd.spawn(())` is valid — `spawn` takes a `ComponentBundle`, and the unit type may or may not implement it. If it does not, spawn with `Transform::IDENTITY` and let the node's own `Transform` payload overwrite it.

- [ ] **Step 5: Update `GltfImporter` to emit components**

In `crates/gltf-loader/src/gltf_importer.rs`, replace the typed `SceneNode` construction. Where the importer previously set `transform`, `mesh`, and `material` fields, it now builds a node and pushes components, recording referenced ids:

```rust
let mut scene_node = SceneNode {
    name: gltf_node.name().map(str::to_string).unwrap_or_default(),
    children: gltf_node.children().map(|child| child.index()).collect(),
    components: Vec::new(),
};

scene_node.push_component(&Transform::from_matrix(&gltf_node.transform().matrix()))?;
```

For a single-primitive mesh node, replacing the old `mesh`/`material` field assignment:

```rust
let mesh_id = ctx.sub_asset_id(&format!("mesh/{}", prim.mesh_sub_asset));
let material_id = ctx.sub_asset_id(&format!("material/{}", prim.material_sub_asset));

scene_node.push_component(&MeshComponent { handle: AssetHandle::weak(mesh_id) })?;
scene_node.push_component(&MaterialComponent { handle: AssetHandle::weak(material_id) })?;
scene_node.push_component(&SyncWithRenderWorld)?;

referenced_assets.push(mesh_id);
referenced_assets.push(material_id);
```

The appended child nodes for multi-primitive meshes get the same three components plus an identity `Transform`. Emit `Scene { nodes, referenced_assets }`.

Map `push_component`'s `anyhow::Error` into `ImportError::SerializationFailed { sub_asset_name, message }`.

- [ ] **Step 6: Update `ObjImporter` to emit components**

Apply the same change in `crates/obj-loader/src/obj_importer.rs`: each model's flat `SceneNode` gets `Transform::default()`, `MeshComponent`, `MaterialComponent` (when an `.mtl` was emitted), and `SyncWithRenderWorld` pushed as components, with the ids recorded into `referenced_assets`.

- [ ] **Step 7: Bump the cook format version**

In `crates/asset-cook/src/cook.rs`, change `pub const COOK_FORMAT_VERSION: u32 = 1;` to `2`. `SceneNode`'s layout changed, so every previously cooked scene must be rebuilt; the existing version check in `run.rs::source_is_unchanged` makes stale indexes dirty automatically.

- [ ] **Step 8: Update the existing importer and scene tests**

`crates/scene/tests/scene_serialization.rs`, `crates/gltf-loader/tests/gltf_importer.rs`, and `crates/obj-loader/tests/obj_importer.rs` assert on the old typed fields. Rewrite those assertions against the component list, e.g. in the glTF test:

```rust
let scene_entry = outputs.sub_assets.iter().find(|s| s.name == "scene").unwrap();
let cooked_scene: Scene = bincode::deserialize(&scene_entry.bytes).unwrap();

assert_eq!(cooked_scene.nodes[0].name, "Triangle");
let mesh_component = cooked_scene.nodes[0]
    .components
    .iter()
    .find(|c| c.type_name == "MeshComponent")
    .expect("the node must carry a MeshComponent payload");
let decoded: MeshComponent = serde_json::from_str(&mesh_component.data).unwrap();
assert_eq!(
    decoded.handle.id(),
    AssetId::from_path("triangle.gltf#mesh/0"),
    "the scene node's mesh handle must carry the exact AssetId a runtime load would compute"
);
```

Keep every existing assertion's intent — only the access path changes. The multi-primitive flattening test keeps asserting parent/child structure and that leaf nodes carry mesh components while the parent does not.

- [ ] **Step 9: Run tests and build**

Run: `cargo test -p scene -p gltf-loader -p obj-loader && cargo build --workspace && cargo test --workspace`
Expected: all pass; builds clean with zero warnings.

- [ ] **Step 10: Re-cook and run the example**

```bash
cargo run -p cook -- examples/render-test/assets.toml examples/render-test/assets examples/render-test/res
cargo run -p render-test
```
Expected: `errors: 0` (a full re-cook, since the format version bumped); Sponza still renders.

- [ ] **Step 11: Format and commit**

```bash
cargo fmt -p scene -p gltf-loader -p obj-loader -p asset-cook -p ecs
cargo fmt --all -- --check
git add crates/scene crates/gltf-loader crates/obj-loader crates/asset-cook crates/ecs
git commit -m "$(cat <<'EOF'
feat(scene)!: represent scenes as trees of serialized components

SceneNode carries Vec<SerializedComponent> instead of typed transform/mesh/
material fields; both importers emit component payloads and record the ids
they reference; the spawner applies them generically through the registry.
Bumps COOK_FORMAT_VERSION to 2.

Co-Authored-By: Claude Opus 5 <noreply@anthropic.com>
EOF
)"
```

---

### Task 7: Cook `Skeleton` and `AnimationClip` as assets

**Files:**
- Modify: `crates/mesh/src/skeleton.rs`
- Modify: `crates/animation/src/clip.rs`
- Modify: `crates/animation/Cargo.toml`
- Test: `crates/animation/tests/clip_serialization.rs` (new)

**Interfaces:**
- Produces: `Skeleton` and `AnimationClip` implement `Serialize`/`Deserialize` + `CookedAsset` (`TYPE_NAME` `"Skeleton"` / `"AnimationClip"`) + `LoadableAsset` with `SkeletonLoader`/`AnimationClipLoader` reading cooked bytes by `AssetLoadContext::asset_id()`.

- [ ] **Step 1: Write the failing test**

Create `crates/animation/tests/clip_serialization.rs`:

```rust
//! Covers AnimationClip round-tripping through bincode so it can be cooked
//! as a standalone sub-asset addressable as "file.gltf#animation/0".
use animation::clip::{AnimationChanelOutput, AnimationChannel, AnimationClip};
use glam::Vec3;
use uuid::Uuid;

#[test]
fn animation_clip_round_trips_through_bincode() {
    let bone_id = Uuid::from_u128(0x1234_5678_9abc_def0_1234_5678_9abc_def0);
    let mut clip = AnimationClip::default();
    clip.add_channel(
        bone_id,
        AnimationChannel::new(
            vec![0.0, 0.5, 1.0],
            AnimationChanelOutput::Translation(vec![
                Vec3::ZERO,
                Vec3::new(1.0, 0.0, 0.0),
                Vec3::new(2.0, 0.0, 0.0),
            ]),
        ),
    );

    let bytes = bincode::serialize(&clip).expect("AnimationClip must serialize");
    let decoded: AnimationClip = bincode::deserialize(&bytes).expect("AnimationClip must round-trip");

    let channels = decoded
        .get_channels(&bone_id)
        .expect("the channel must survive keyed by the same bone id");
    assert_eq!(channels.len(), 1, "one channel was added, one must come back");
}

#[test]
fn skeleton_round_trips_through_bincode() {
    use glam::Mat4;
    use mesh::skeleton::Skeleton;

    let skeleton = Skeleton::from(vec![Mat4::IDENTITY, Mat4::from_translation(Vec3::X)]);
    let bytes = bincode::serialize(&skeleton).expect("Skeleton must serialize");
    let decoded: Skeleton = bincode::deserialize(&bytes).expect("Skeleton must round-trip");

    assert_eq!(
        decoded.inverse_bindposes.len(),
        2,
        "both inverse bind poses must survive the round-trip"
    );
}
```

- [ ] **Step 2: Run test to verify it fails**

Run: `cargo test -p animation --test clip_serialization`
Expected: FAIL to compile — the types are not `Serialize`.

- [ ] **Step 3: Add serde derives**

`crates/mesh/src/skeleton.rs` — add `Serialize, Deserialize` to `Skeleton`'s derive list. `Box<[Mat4]>` and `glam::Mat4` (with glam's serde feature) both serialize.

`crates/animation/src/clip.rs` — add `Serialize, Deserialize` derives to `AnimationChanelOutput`, `AnimationChannel`, and `AnimationClip`. All fields are plain data (`Vec<f32>`, `Vec<Vec3>`, `Vec<Quat>`, `HashMap<Uuid, Vec<AnimationChannel>>`). The private fields are fine — the derive expands in the same module.

Add to `crates/animation/Cargo.toml`: `serde = { version = "1", features = ["derive"] }`, `bincode = "1.3"`, `asset-cook = { path = "../asset-cook" }`, and the `serde` feature on its `glam` and `uuid` dependencies. Add the `serde` feature to `crates/mesh`'s `glam` dependency if absent.

- [ ] **Step 4: Make both types cooked, loadable assets**

In `crates/mesh/src/skeleton.rs`, mirroring `Mesh`'s existing `CookedAsset`/`LoadableAsset`/`MeshLoader` block exactly (same async_trait `cfg_attr` pattern, same `load_cooked_asset_bytes` call from Task 2):

```rust
impl CookedAsset for Skeleton {
    const TYPE_NAME: &'static str = "Skeleton";
}

impl LoadableAsset for Skeleton {
    type UsageSettings = ();
    fn loader() -> Box<dyn AssetLoader<Asset = Self>> { Box::new(SkeletonLoader) }
    fn default_usage_settings() -> Self::UsageSettings {}
}

pub struct SkeletonLoader;

#[cfg_attr(target_arch = "wasm32", async_trait::async_trait(?Send))]
#[cfg_attr(not(target_arch = "wasm32"), async_trait::async_trait)]
impl AssetLoader for SkeletonLoader {
    type Asset = Skeleton;
    async fn load(
        &self,
        _path: AssetPath<'static>,
        load_context: &mut AssetLoadContext,
        _usage_settings: (),
    ) -> anyhow::Result<Self::Asset> {
        let bytes = essential::assets::utils::load_cooked_asset_bytes(
            load_context.cooked_root(),
            load_context.asset_id(),
        )
        .await
        .with_context(|| "failed to read cooked skeleton")?;
        bincode::deserialize(&bytes).with_context(|| "failed to deserialize cooked skeleton")
    }
}
```

Add the identical block for `AnimationClip` in `crates/animation/src/clip.rs` (`TYPE_NAME = "AnimationClip"`, `AnimationClipLoader`, "cooked animation clip" messages).

Register both in `crates/scene/src/plugin.rs`: `app.register_asset::<Skeleton>();` and `app.register_asset::<AnimationClip>();`. **Verify during implementation:** grep for an existing `register_asset::<Skeleton>` first — `RenderPlugin` registers `Skeleton` already, and re-registering would swap a populated `AssetStore` for an empty one. Register only what is missing.

- [ ] **Step 5: Run tests and build**

Run: `cargo test -p animation --test clip_serialization && cargo build --workspace && cargo test --workspace`
Expected: 2 tests pass; builds clean; workspace tests pass.

- [ ] **Step 6: Format and commit**

```bash
cargo fmt -p mesh -p animation -p scene
cargo fmt --all -- --check
git add crates/mesh crates/animation crates/scene
git commit -m "$(cat <<'EOF'
feat(animation): cook Skeleton and AnimationClip as loadable assets

Both are plain data, so they derive serde directly and gain CookedAsset +
LoadableAsset impls reading cooked bytes by AssetId -- making a clip
addressable as "idle.gltf#animation/0".

Co-Authored-By: Claude Opus 5 <noreply@anthropic.com>
EOF
)"
```

---

### Task 8: `SceneSkeleton` — the expanding component

**Files:**
- Create: `crates/scene/src/skeleton.rs`
- Modify: `crates/scene/src/lib.rs`
- Modify: `crates/scene/src/plugin.rs`
- Modify: `crates/scene/Cargo.toml`
- Test: `crates/scene/tests/scene_skeleton.rs` (new)

**Interfaces:**
- Consumes: `SceneComponent`, `SceneSpawnContext`, `SceneEntityRef` (Task 3); `Skeleton` as a loadable asset (Task 7).
- Produces: `struct SceneSkeleton { skeleton: AssetHandle<Skeleton>, bones: Vec<SceneEntityRef>, bone_ids: Vec<Uuid>, root: Option<SceneEntityRef> }` whose `apply` inserts `SkeletonComponent` + `AnimationPlayer` on its own entity and `AnimationRootBone` on the root bone's entity, and never inserts itself.

- [ ] **Step 1: Write the failing test**

Create `crates/scene/tests/scene_skeleton.rs`:

```rust
//! Covers the expansion case of the SceneComponent interface: SceneSkeleton
//! is authoring data that becomes several runtime components -- one of them
//! on a *different* entity -- and never lands on an entity itself.
use animation::player::AnimationPlayer;
use animation::root::AnimationRootBone;
use ecs::component::scene::{SceneComponent, SceneEntityRef, SceneSpawnContext};
use ecs::World;
use essential::assets::{handle::AssetHandle, AssetId};
use mesh::skeleton::{Skeleton, SkeletonComponent};
use scene::skeleton::SceneSkeleton;
use uuid::Uuid;

#[test]
fn scene_skeleton_expands_into_runtime_components() {
    let mut world = World::default();
    let owner = world.spawn(());
    let bone_a = world.spawn(());
    let bone_b = world.spawn(());
    let nodes = [owner, bone_a, bone_b];

    let authoring = SceneSkeleton {
        skeleton: AssetHandle::<Skeleton>::weak(AssetId::from_path("rig.gltf#skeleton/0")),
        bones: vec![SceneEntityRef(1), SceneEntityRef(2)],
        bone_ids: vec![Uuid::from_u128(1), Uuid::from_u128(2)],
        root: Some(SceneEntityRef(1)),
    };

    {
        let mut ctx = SceneSpawnContext::new((&mut world).into(), &nodes);
        authoring.apply(owner, &mut ctx);
    }

    let skeleton_component = world
        .get_component_for_entity::<SkeletonComponent>(owner)
        .expect("apply must insert a SkeletonComponent on its own entity");
    assert_eq!(
        skeleton_component.bones(),
        &[bone_a, bone_b],
        "SceneEntityRef indices must resolve to the spawned bone entities"
    );

    assert!(
        world.get_component_for_entity::<AnimationPlayer>(owner).is_some(),
        "apply must insert an AnimationPlayer sized to the bone count"
    );
    assert!(
        world.get_component_for_entity::<AnimationRootBone>(bone_a).is_some(),
        "the root bone marker must land on the root bone's entity, not the owner"
    );
    assert!(
        world.get_component_for_entity::<SceneSkeleton>(owner).is_none(),
        "authoring data must never insert one of itself"
    );
}

#[test]
fn out_of_range_bone_refs_are_skipped() {
    let mut world = World::default();
    let owner = world.spawn(());
    let nodes = [owner];

    let authoring = SceneSkeleton {
        skeleton: AssetHandle::<Skeleton>::weak(AssetId::from_path("rig.gltf#skeleton/0")),
        bones: vec![SceneEntityRef(9)],
        bone_ids: vec![Uuid::from_u128(1)],
        root: Some(SceneEntityRef(9)),
    };

    {
        let mut ctx = SceneSpawnContext::new((&mut world).into(), &nodes);
        authoring.apply(owner, &mut ctx);
    }

    let skeleton_component = world
        .get_component_for_entity::<SkeletonComponent>(owner)
        .expect("a malformed scene must still produce a component, not a panic");
    assert!(
        skeleton_component.bones().is_empty(),
        "an out-of-range bone reference must be dropped rather than panicking"
    );
}
```

- [ ] **Step 2: Run test to verify it fails**

Run: `cargo test -p scene --test scene_skeleton`
Expected: FAIL to compile — `scene::skeleton` does not exist.

- [ ] **Step 3: Implement `SceneSkeleton`**

Create `crates/scene/src/skeleton.rs`:

```rust
use animation::player::AnimationPlayer;
use animation::root::AnimationRootBone;
use ecs::component::scene::{SceneComponent, SceneEntityRef, SceneSpawnContext};
use ecs::{Component, Entity};
use essential::assets::{asset_server::AssetServer, handle::AssetHandle};
use mesh::skeleton::{Skeleton, SkeletonComponent};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

/// Skeleton binding as authored into a cooked scene. Bones are node indices
/// here because `SkeletonComponent` holds `Entity`, which cannot exist at rest.
///
/// Derives `Component` only to satisfy the `SceneComponent: Component` bound —
/// `apply` never inserts one, so no entity ever carries it.
#[derive(Component, Debug, Clone, Serialize, Deserialize)]
pub struct SceneSkeleton {
    pub skeleton: AssetHandle<Skeleton>,
    pub bones: Vec<SceneEntityRef>,
    pub bone_ids: Vec<Uuid>,
    pub root: Option<SceneEntityRef>,
}

impl SceneComponent for SceneSkeleton {
    fn apply(self, entity: Entity, ctx: &mut SceneSpawnContext<'_>) {
        let bones: Vec<Entity> = self
            .bones
            .iter()
            .filter_map(|reference| ctx.entity_for(*reference))
            .collect();

        let skeleton = match ctx.world().get_resource::<AssetServer>() {
            Some(server) => server.load_by_id(self.skeleton.id()),
            None => self.skeleton.clone(),
        };

        if let Some(root) = self.root.and_then(|reference| ctx.entity_for(reference)) {
            ctx.insert(AnimationRootBone::default(), root);
        }

        ctx.insert(AnimationPlayer::new(bones.len()), entity);
        ctx.insert(
            SkeletonComponent::new(skeleton, bones, self.bone_ids),
            entity,
        );
    }
}
```

Add `pub mod skeleton;` to `crates/scene/src/lib.rs`, `app.register_component::<SceneSkeleton>();` to `ScenePlugin`, and `animation = { path = "../animation" }` plus `uuid` to `crates/scene/Cargo.toml`.

**Verify during implementation:** confirm `AnimationRootBone` derives or implements `Default`; if not, construct it however `spawn_gltf_components` did at git `1d0682d^:crates/gltf-loader/src/loader.rs` line ~844.

- [ ] **Step 4: Run tests and build**

Run: `cargo test -p scene --test scene_skeleton && cargo build --workspace && cargo test --workspace`
Expected: 2 tests pass; builds clean; workspace tests pass.

- [ ] **Step 5: Format and commit**

```bash
cargo fmt -p scene
cargo fmt --all -- --check
git add crates/scene
git commit -m "$(cat <<'EOF'
feat(scene): add SceneSkeleton, the expanding scene component

SceneSkeleton is authoring data: apply resolves its bone node-indices to
entities, inserts SkeletonComponent and AnimationPlayer on its own entity and
AnimationRootBone on the root bone's, and never inserts one of itself.

Co-Authored-By: Claude Opus 5 <noreply@anthropic.com>
EOF
)"
```

---

### Task 9: `GltfImporter` emits skeletons and animations

**Files:**
- Modify: `crates/gltf-loader/src/gltf_importer.rs`
- Create: `crates/gltf-loader/tests/fixtures/skinned.gltf`
- Modify: `crates/gltf-loader/tests/gltf_importer.rs`

**Interfaces:**
- Consumes: `Skeleton`/`AnimationClip` cooked assets (Task 7); `SceneSkeleton` (Task 8).
- Produces: `GltfImporter` emits `skeleton/N` and `animation/N` sub-assets and a `SceneSkeleton` component on each skinned node.

- [ ] **Step 1: Restore the node-path helpers**

Port `collect_paths` and `paths_to_uuid` from git `1d0682d^:crates/gltf-loader/src/loader.rs` (lines 919-946) into `crates/gltf-loader/src/gltf_importer.rs` as private free functions, together with the `GLTFNodePathInfo` struct they use (rename it `NodePathInfo`). These derive the stable per-bone `Uuid` that `AnimationClip` channels are keyed by; the bone ids must match between the skeleton and the clips or animation silently does nothing.

Retrieve them with:
```bash
git show 1d0682d^:crates/gltf-loader/src/loader.rs > /tmp/old-gltf-loader.rs
```

- [ ] **Step 2: Write the failing test with a skinned fixture**

Create `crates/gltf-loader/tests/fixtures/skinned.gltf` by running this generator once and committing its output. Hand-computing the buffer offsets and base64 is error-prone; the script makes the fixture deterministic and self-documenting. Save it as `/tmp/gen_skinned_gltf.py` and run `python3 /tmp/gen_skinned_gltf.py > crates/gltf-loader/tests/fixtures/skinned.gltf`:

```python
import base64, json, struct

# A two-joint rig: one triangle skinned entirely to joint 0, and one
# animation channel translating joint 0 along X over one second.
blobs = []          # (name, bytes, target_is_index_buffer)
def add(name, data):
    while len(blobs) and (sum(len(b[1]) for b in blobs) % 4):   # 4-byte align
        blobs[-1] = (blobs[-1][0], blobs[-1][1] + b"\0", blobs[-1][2])
    blobs.append((name, data, False))
    return len(blobs) - 1

positions = struct.pack("<9f", 0,0,0,  1,0,0,  0,1,0)
indices   = struct.pack("<3H", 0, 1, 2)
joints    = struct.pack("<12B", *([0,0,0,0] * 3))
weights   = struct.pack("<12f", *([1.0,0.0,0.0,0.0] * 3))
ibm       = struct.pack("<32f", *([1,0,0,0, 0,1,0,0, 0,0,1,0, 0,0,0,1] * 2))
times     = struct.pack("<2f", 0.0, 1.0)
trans     = struct.pack("<6f", 0,0,0,  1,0,0)

bv_pos = add("pos", positions); bv_idx = add("idx", indices)
bv_jnt = add("jnt", joints);    bv_wgt = add("wgt", weights)
bv_ibm = add("ibm", ibm);       bv_tim = add("tim", times)
bv_trn = add("trn", trans)

buffer, views, offset = b"", [], 0
for _name, data, _ in blobs:
    views.append({"buffer": 0, "byteOffset": offset, "byteLength": len(data)})
    buffer += data
    offset += len(data)

gltf = {
  "asset": {"version": "2.0"},
  "scene": 0,
  "scenes": [{"nodes": [0, 1]}],
  "nodes": [
    {"name": "SkinnedMesh", "mesh": 0, "skin": 0},
    {"name": "Root",  "children": [2]},
    {"name": "Joint", "translation": [0.0, 1.0, 0.0]}
  ],
  "meshes": [{"primitives": [{
      "attributes": {"POSITION": 0, "JOINTS_0": 2, "WEIGHTS_0": 3},
      "indices": 1}]}],
  "skins": [{"joints": [1, 2], "inverseBindMatrices": 4, "skeleton": 1}],
  "animations": [{"name": "Wiggle",
      "samplers": [{"input": 5, "output": 6, "interpolation": "LINEAR"}],
      "channels": [{"sampler": 0, "target": {"node": 1, "path": "translation"}}]}],
  "accessors": [
    {"bufferView": bv_pos, "componentType": 5126, "count": 3, "type": "VEC3",
     "min": [0,0,0], "max": [1,1,0]},
    {"bufferView": bv_idx, "componentType": 5123, "count": 3, "type": "SCALAR"},
    {"bufferView": bv_jnt, "componentType": 5121, "count": 3, "type": "VEC4"},
    {"bufferView": bv_wgt, "componentType": 5126, "count": 3, "type": "VEC4"},
    {"bufferView": bv_ibm, "componentType": 5126, "count": 2, "type": "MAT4"},
    {"bufferView": bv_tim, "componentType": 5126, "count": 2, "type": "SCALAR",
     "min": [0.0], "max": [1.0]},
    {"bufferView": bv_trn, "componentType": 5126, "count": 2, "type": "VEC3"}
  ],
  "bufferViews": views,
  "buffers": [{"byteLength": len(buffer),
               "uri": "data:application/octet-stream;base64," +
                      base64.b64encode(buffer).decode()}]
}
print(json.dumps(gltf, indent=2))
```

This generator was run and its output validated against `gltf::import` while writing this plan: it yields `joints=[1, 2]`, 2 inverse bind matrices, 1 animation channel, and a `SkinnedMesh` node referencing the skin. If you change it and `gltf::import` starts rejecting the result, the error names the offending accessor or buffer view — fix the generator and regenerate rather than editing the JSON by hand.

Add to `crates/gltf-loader/tests/gltf_importer.rs`:

```rust
#[test]
fn import_emits_skeleton_and_animation_sub_assets() {
    let fixture = Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/skinned.gltf");
    let mut ctx = ImportContext::new(std::path::PathBuf::from("skinned.gltf"));

    GltfImporter
        .import(&fixture, &mut ctx)
        .expect("importing the skinned fixture should succeed");
    let outputs = ctx.into_parts();

    let names: Vec<&str> = outputs.sub_assets.iter().map(|s| s.name.as_str()).collect();
    assert!(names.contains(&"skeleton/0"), "expected a skeleton sub-asset, got: {names:?}");
    assert!(names.contains(&"animation/0"), "expected an animation sub-asset, got: {names:?}");

    let scene_entry = outputs.sub_assets.iter().find(|s| s.name == "scene").unwrap();
    let cooked_scene: Scene = bincode::deserialize(&scene_entry.bytes).unwrap();

    let skinned_node = cooked_scene
        .nodes
        .iter()
        .find(|node| node.components.iter().any(|c| c.type_name == "SceneSkeleton"))
        .expect("a skinned node must carry a SceneSkeleton component");

    let payload = skinned_node
        .components
        .iter()
        .find(|c| c.type_name == "SceneSkeleton")
        .unwrap();
    let scene_skeleton: SceneSkeleton = serde_json::from_str(&payload.data).unwrap();

    assert_eq!(
        scene_skeleton.skeleton.id(),
        AssetId::from_path("skinned.gltf#skeleton/0"),
        "the skeleton handle must address the emitted skeleton sub-asset"
    );
    assert_eq!(
        scene_skeleton.bones.len(),
        scene_skeleton.bone_ids.len(),
        "every bone must have a matching stable id for animation channel lookup"
    );
    assert!(
        !scene_skeleton.bones.is_empty(),
        "the fixture's skin has joints, so bones must not be empty"
    );
}
```

- [ ] **Step 3: Run test to verify it fails**

Run: `cargo test -p gltf-loader --test gltf_importer`
Expected: FAIL — no `skeleton/0` sub-asset is emitted.

- [ ] **Step 4: Emit skeletons**

Port the skin-reading loop from the old loader (lines 318-355 of `/tmp/old-gltf-loader.rs`), adapting it: instead of `load_context.asset_server().add(inverse_bind_matrices)`, emit the `Skeleton` as a sub-asset and build a `SceneSkeleton`:

```rust
let mut node_paths = HashMap::new();
for scene in document.scenes() {
    for root_node in scene.nodes() {
        collect_paths(&root_node, &[], &mut node_paths, &mut HashSet::new());
    }
}

for (skin_index, skin) in document.skins().enumerate() {
    let Some(inverse_bind_matrices) = skin
        .reader(|buffer| Some(&buffers[buffer.index()]))
        .read_inverse_bind_matrices()
        .map(|iter| iter.map(|pose| Mat4::from_cols_array_2d(&pose)).collect::<Vec<_>>())
    else {
        continue;
    };

    let skeleton = Skeleton::from(inverse_bind_matrices);
    ctx.emit(&format!("skeleton/{skin_index}"), &skeleton)?;

    let bones: Vec<SceneEntityRef> =
        skin.joints().map(|joint| SceneEntityRef(joint.index())).collect();
    let bone_ids: Vec<Uuid> = skin
        .joints()
        .map(|joint| paths_to_uuid(&node_paths[&joint.index()].node_path))
        .collect();

    // Record for the node that references this skin.
    skins.push(SkinInfo { skeleton_index: skin_index, bones, bone_ids });
}
```

Use a named `struct SkinInfo { skeleton_index: usize, bones: Vec<SceneEntityRef>, bone_ids: Vec<Uuid> }` — no unnamed tuples.

When walking nodes, a node with `gltf_node.skin()` pushes a `SceneSkeleton` component built from the matching `SkinInfo`, with `skeleton: AssetHandle::weak(ctx.sub_asset_id(&format!("skeleton/{index}")))` and that id recorded into `referenced_assets`. `root` is the joint whose parent is not itself a joint; if that is ambiguous, use the first joint, matching the old loader's `root_bone` behaviour when no name was supplied.

- [ ] **Step 5: Emit animations**

Port the animation loop from the old loader (lines 357-428), replacing `load_context.asset_server().add(animation_clip)` with `ctx.emit(&format!("animation/{index}"), &animation_clip)?`. The channel target ids come from the same `paths_to_uuid(&node_paths[&target_node_idx].node_path)` call, so clips and skeletons agree on bone identity. Keep the existing `warn!` calls for channels with no time samples and nodes with no path.

- [ ] **Step 6: Run tests and build**

Run: `cargo test -p gltf-loader && cargo build --workspace && cargo test --workspace`
Expected: all pass; builds clean.

- [ ] **Step 7: Format and commit**

```bash
cargo fmt -p gltf-loader
cargo fmt --all -- --check
git add crates/gltf-loader
git commit -m "$(cat <<'EOF'
feat(gltf-loader): emit skeletons and animation clips as sub-assets

Restores the skin and animation parsing from the pre-pipeline loader,
emitting skeleton/N and animation/N sub-assets and a SceneSkeleton component
per skinned node. Bone ids come from the restored path-hashing helpers, so
clips and skeletons agree on bone identity.

Co-Authored-By: Claude Opus 5 <noreply@anthropic.com>
EOF
)"
```

---

### Task 10: `GltfImporter` emits cameras, lights and Blender extras

**Files:**
- Modify: `crates/gltf-loader/src/gltf_importer.rs`
- Modify: `crates/gltf-loader/tests/gltf_importer.rs`
- Modify: `crates/gltf-loader/tests/fixtures/triangle.gltf`

**Interfaces:**
- Consumes: `Camera`/`Light` `SceneComponent` impls (Task 5).
- Produces: `GltfImporter` emits `Camera`, `Light` (+ `SyncWithRenderWorld`), and arbitrary Blender-`extras` components on the owning nodes.

- [ ] **Step 1: Write the failing test**

Add a `KHR_lights_punctual` point light and an `extras` block to `crates/gltf-loader/tests/fixtures/triangle.gltf`'s node:

```json
"nodes": [{
  "name": "Triangle",
  "mesh": 0,
  "extensions": { "KHR_lights_punctual": { "light": 0 } },
  "extras": { "components": { "MeshCollider": {} } }
}],
"extensions": {
  "KHR_lights_punctual": {
    "lights": [{ "type": "point", "color": [1.0, 0.5, 0.25], "intensity": 683.0 }]
  }
},
"extensionsUsed": ["KHR_lights_punctual"],
```

Add to `crates/gltf-loader/tests/gltf_importer.rs`:

```rust
#[test]
fn import_emits_light_and_extras_components() {
    let fixture = Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/triangle.gltf");
    let mut ctx = ImportContext::new(std::path::PathBuf::from("triangle.gltf"));

    GltfImporter.import(&fixture, &mut ctx).expect("import should succeed");
    let outputs = ctx.into_parts();

    let scene_entry = outputs.sub_assets.iter().find(|s| s.name == "scene").unwrap();
    let cooked_scene: Scene = bincode::deserialize(&scene_entry.bytes).unwrap();
    let names: Vec<&str> = cooked_scene.nodes[0]
        .components
        .iter()
        .map(|c| c.type_name.as_str())
        .collect();

    assert!(names.contains(&"Light"), "the punctual light must become a Light component, got: {names:?}");
    assert!(
        names.contains(&"MeshCollider"),
        "a Blender extras entry must become a component payload verbatim, got: {names:?}"
    );
}
```

- [ ] **Step 2: Run test to verify it fails**

Run: `cargo test -p gltf-loader --test gltf_importer`
Expected: FAIL — no `Light` or `MeshCollider` component is emitted.

- [ ] **Step 3: Emit cameras and lights**

Port the camera and light loops from `/tmp/old-gltf-loader.rs` (lines 430-470), keeping the `LUMINOUS_EFFICACY` intensity division and the orthographic-camera warning. Instead of building `GLTFCamera`/`GLTFLight` intermediates, push real components onto the owning node:

```rust
if let Some(camera) = gltf_node.camera() {
    if let gltf::camera::Projection::Perspective(perspective) = camera.projection() {
        scene_node.push_component(&Camera {
            fovy: perspective.yfov(),
            znear: perspective.znear(),
            zfar: perspective.zfar().unwrap_or(100.0),
            ..Camera::default()
        })?;
    }
}

if let Some(light) = gltf_node.light() {
    let [r, g, b] = light.color();
    let light_type = match light.kind() {
        gltf::khr_lights_punctual::Kind::Directional => LightType::Directional,
        gltf::khr_lights_punctual::Kind::Point => LightType::Point,
        gltf::khr_lights_punctual::Kind::Spot { outer_cone_angle, .. } => {
            LightType::Spot { cone_angle: outer_cone_angle }
        }
    };
    scene_node.push_component(&Light {
        color: Color::rgba(r, g, b, 1.0),
        intensity: light.intensity() / LUMINOUS_EFFICACY,
        light_type,
        shadowmaps_enabled: false,
    })?;
    scene_node.push_component(&SyncWithRenderWorld)?;
}
```

`shadowmaps_enabled` was previously supplied by `GLTFSpawnerComponent::lights_cast_shadows`, which no longer exists; default it to `false` and note that per-scene shadow control is a follow-up.

- [ ] **Step 4: Emit Blender extras**

Port `parse_extras` from the old loader (lines 632-649). Each extracted entry becomes a `SerializedComponent` directly — the payload is already JSON, so it is pushed without re-serialization:

```rust
for extra in parse_extras(gltf_node.extras()) {
    scene_node.components.push(SerializedComponent {
        type_name: extra.name,
        data: extra.data,
    });
}
```

Keep the `EXTRAS_COMPONENTS_KEY` constant and use a named `struct ExtraComponentData { name: String, data: String }`.

- [ ] **Step 5: Run tests and build**

Run: `cargo test -p gltf-loader && cargo build --workspace && cargo test --workspace`
Expected: all pass; builds clean.

- [ ] **Step 6: Format and commit**

```bash
cargo fmt -p gltf-loader
cargo fmt --all -- --check
git add crates/gltf-loader
git commit -m "$(cat <<'EOF'
feat(gltf-loader): emit camera, light and Blender-extras components

Restores the camera/light parsing and the extras component injection from
PR #56, now as component payloads on the owning scene node. Extras pass
through verbatim since they are already JSON.

Co-Authored-By: Claude Opus 5 <noreply@anthropic.com>
EOF
)"
```

---

### Task 11: Un-park `tech-demo` and `animation-test`

**Files:**
- Modify: `Cargo.toml` (workspace root)
- Modify: `examples/tech-demo/src/main.rs`, `examples/tech-demo/src/scene.rs`, `examples/tech-demo/src/character.rs`
- Modify: `examples/animation-test/src/main.rs`, `examples/animation-test/src/movement_animation.rs`, `examples/animation-test/src/demo_overlay.rs`
- Create: `examples/tech-demo/assets.toml`, `examples/animation-test/assets.toml`

**Interfaces:**
- Consumes: everything from Tasks 1-10.
- Produces: both examples build under `cargo build --workspace` and animate at runtime.

- [ ] **Step 1: Rejoin the workspace**

In the root `Cargo.toml`, delete the `exclude = ["examples/tech-demo", "examples/animation-test"]` line and its `TODO(asset-import-pipeline)` comment block.

Run: `cargo build --workspace`
Expected: FAIL — both examples reference the removed `GLTFScene`/`GLTFSpawnerComponent`/`GLTFUsageSettings`/`GLTFInstance` API. The compiler errors are the task list for the following steps.

- [ ] **Step 2: Reorganize both examples' assets and write manifests**

For each example, move its DCC sources out of `res/` into `assets/` (`git mv examples/tech-demo/res examples/tech-demo/assets`, same for animation-test), and create an `assets.toml` listing every source the example loads — grep each example's `asset_server.load` calls to enumerate them exactly. Mirror `examples/render-test/assets.toml`:

```toml
[[assets]]
path = "UAL1.glb"
```

Add `/examples/tech-demo/res/` and `/examples/animation-test/res/` to the root `.gitignore` beside the render-test entry, and give each example the same `build.rs` and `[build-dependencies]` that Task 2 restored for render-test.

- [ ] **Step 3: Migrate the spawner call sites**

Replace `GLTFSpawnerComponent::from_handle(asset_server.load(PATH))` with `SceneSpawnerComponent(asset_server.load::<Scene>(PATH))`, where `PATH` gains a `#scene` fragment (`"UAL1.glb#scene"`). The `.with_shadows()` / `.with_physics_shapes()` builders have no `Scene` equivalent — drop them and add a `// TODO(asset-import-pipeline)` naming the lost behaviour, as render-test does.

`AssetHandle<GLTFScene>` fields become `AssetHandle<AnimationClip>` where the example was loading an animation-only glTF: `asset_server.load::<GLTFScene>(IDLE_ANIM)` becomes `asset_server.load::<AnimationClip>("idle.gltf#animation/0")`. `Res<AssetStore<GLTFScene>>` becomes `Res<AssetStore<AnimationClip>>` and the `.get_animation(name)` lookups become direct handle use, since each clip is now its own asset.

`GLTFUsageSettings { root_bone: ... }` has no replacement — the root bone is chosen by the importer. Delete those settings and use plain `asset_server.load::<Scene>(...)`.

`GLTFInstance` (used by tech-demo's `character.rs` to find spawned nodes by name and to know when spawning finished) has no replacement. Replace its uses with queries over the spawned hierarchy: the `SkeletonComponent`/`AnimationPlayer` a scene inserts are themselves the "spawning finished" signal, so `Query<(Entity, &AnimationPlayer), (With<Player>, Without<AnimationsReady>)>` replaces `Query<(Entity, &GLTFInstance), ...>`.

`app.register_reflection::<PlayerSpawner>()` becomes `app.register_component::<PlayerSpawner>()`, and `PlayerSpawner` swaps its `Facet` derive for `Serialize`/`Deserialize` plus an insert-self `impl SceneComponent`.

- [ ] **Step 4: Cook both examples**

```bash
cargo run -p cook -- examples/tech-demo/assets.toml examples/tech-demo/assets examples/tech-demo/res
cargo run -p cook -- examples/animation-test/assets.toml examples/animation-test/assets examples/animation-test/res
```
Expected: `errors: 0` for both.

- [ ] **Step 5: Build and test the workspace**

Run: `cargo build --workspace && cargo test --workspace && cargo fmt --all -- --check && cargo clippy -- -A clippy::type_complexity -A clippy::too_many_arguments -D warnings`
Expected: all green with zero warnings.

- [ ] **Step 6: Verify animation visually**

Run each example and confirm the character animates — not just renders in bind pose. Per `docs`/the project's known recipe, run with `env -u WAYLAND_DISPLAY` so winit uses XWayland, wait for asset loading to finish before judging, and capture a frame to compare against the pre-pipeline behaviour:

```bash
cargo run -p animation-test
cargo run -p tech-demo
```
Expected: the skinned character plays its idle/walk animations. A character stuck in bind pose means the skeleton bound but the clips are not driving it — check that `bone_ids` in the cooked `SceneSkeleton` match the channel keys in the cooked `AnimationClip`.

- [ ] **Step 7: Commit**

```bash
git add Cargo.toml .gitignore examples/tech-demo examples/animation-test
git commit -m "$(cat <<'EOF'
chore(examples): un-park tech-demo and animation-test

Both rejoin the workspace on the cooked-Scene pipeline: DCC sources move to
assets/, res/ becomes cook output, and the GLTFScene/GLTFSpawnerComponent/
GLTFInstance API is replaced by SceneSpawnerComponent plus per-clip
AnimationClip assets.

Co-Authored-By: Claude Opus 5 <noreply@anthropic.com>
EOF
)"
```

---

## Post-Plan Follow-Ups (out of scope)

- Physics-shape generation from scenes, replacing `GLTFSpawnerComponent::with_physics_shapes` (render-test and tech-demo both lose mesh colliders until then).
- Per-scene shadow control, replacing `GLTFSpawnerComponent::with_shadows` — `Light::shadowmaps_enabled` is currently hard-coded to `false` by the importer.
- A `#[derive(SceneComponent)]` emitting the trivial insert-self body, once the boilerplate is worth removing.
- Dropping the `SceneComponent: Component` bound so authoring-only types such as `SceneSkeleton` need not be components at all. Non-breaking when done.
- Per-usage colour space for standalone cooked textures (`ImageImporter` hard-codes sRGB).
- MTL texture addressing in `ObjImporter` (assumes manifest-root-relative, untested).
- Pruning orphaned `.cooked/` files when a source leaves the manifest.
- Per-importer `Importer::validate()` implementations.
