# Asset Import Pipeline Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Replace runtime DCC parsing (`.gltf`, `.obj`/`.mtl`, images) with an offline `cook` step that produces independently-loadable, pre-deserialized "engine assets," so that a single DCC file can be split into multiple sub-assets (mesh, material, texture, scene) without loading the whole source file to reach any one of them.

**Architecture:** A new `asset-cook` crate defines an `Importer` trait and a `cook` CLI that reads an explicit `assets.toml` manifest, runs the matching `Importer` per source file, and writes one `bincode`-serialized file per emitted sub-asset into a flat, `AssetId`-keyed `.cooked/` directory. `AssetId` becomes a stable, deterministic hash of a `path#fragment` string (`AssetId::from_path`) instead of a random UUID, so the cooked file for any asset — including one only reachable via a nested reference, with no path string in hand — is a pure function of its ID, with no index read at load time. `AssetHandle<T>` becomes a `Strong`/`Weak` enum (mirroring Bevy's `Handle<T>`) that serializes to just the bare `AssetId`; asset types that hold references to other assets (`StandardMaterial`, `SceneNode`) derive `Serialize`/`Deserialize` directly on their existing definitions — no separate cooked DTO types anywhere. Existing DCC-parsing crates (`gltf-loader`, `obj-loader`, texture loading in `render`) are converted into offline `Importer` impls. A new shared `Scene` asset type (replacing `GLTFScene`) lets both the glTF and OBJ importers — and any future DCC importer — drive the same ECS-spawn code.

**Tech Stack:** Rust, `bincode` + `serde` (new deps, added where needed, including `essential` for `AssetId`/`AssetHandle`), `toml` (new dep, cook manifest), existing `gltf`/`tobj`/`image` crates (now used offline instead of at runtime).

**Spec:** `docs/superpowers/specs/2026-08-31-asset-import-pipeline-design.md`

## Global Constraints

- No unnamed tuples as data types anywhere in new/modified code — use named structs with named fields, including small internal helper types.
- No runtime fallback to parsing DCC source files — once a phase lands, its asset type is loaded exclusively from cooked output.
- No separate "cooked DTO" types for asset types whose only blocker to direct serialization was an `AssetHandle<T>` field (`StandardMaterial`, `SceneNode`/`Scene`) — those derive `Serialize`/`Deserialize` directly on their real definitions. A DTO remains justified only when the live type holds genuinely non-serializable data unrelated to handles (e.g. `Texture`'s `wgpu::TextureDescriptor`-bearing `usage_settings`).
- `cook` only imports files explicitly listed in `assets.toml` — never walks directories looking for importable files.
- Cooked sub-asset files live in a flat directory keyed by `AssetId` (`res/.cooked/<id>.bin`), never mirrored by source path — the mapping from ID to file location must be a pure function of the ID alone, since a nested reference is often resolved with only an ID in hand, no path.
- Follow the existing test convention: plain `#[test]` functions (no async test runner — this codebase uses `pollster`/its own `TaskPool`, never `tokio`), integration tests in a crate-level `tests/` directory, shared setup in `tests/common/mod.rs` exposing small builder functions, `assert_eq!`/`assert!` with descriptive messages.
- Every new source file mutation must build with `warnings = "deny"` (workspace lint setting) — run `cargo build -p <crate>` (not just `cargo check`) before considering a step done where feasible.
- Naming disambiguation: `AssetHandle<T>`'s new `Weak(AssetId)` variant (meaning "not yet resolved to a loaded asset") is unrelated to the pre-existing `std::sync::Weak<StrongAssetHandle>` used internally by `AssetHandleProvider` for handle-reuse/dedup (meaning "doesn't keep the asset alive"). Both exist in the same crate after this plan — comment any code that could confuse the two.

---

## Phase 0: Foundational addressing & cook infrastructure

### Task 1: Stable, serializable `AssetId`

**Files:**
- Modify: `crates/essential/Cargo.toml`
- Modify: `crates/essential/src/assets/mod.rs`
- Test: `crates/essential/tests/asset_id.rs` (new)

**Interfaces:**
- Produces: `AssetId::from_path(path: &str) -> AssetId` (deterministic UUID v5 hash); `AssetId` gains `#[derive(Serialize, Deserialize)]`; `AssetId::new()` (today's random `v4`) is kept, unchanged, for assets created via `AssetServer::add()` with no stable path.

- [ ] **Step 1: Write the failing tests**

Create `crates/essential/tests/asset_id.rs`:

```rust
//! Covers AssetId::from_path's determinism (same input -> same ID, every
//! run) and its bincode round-trip, both load-bearing for the cook pipeline:
//! cook-time and run-time must independently compute the same ID from the
//! same "path#fragment" string with no shared state.
use essential::assets::AssetId;

#[test]
fn from_path_is_deterministic() {
    let a = AssetId::from_path("models/character.gltf#texture/albedo");
    let b = AssetId::from_path("models/character.gltf#texture/albedo");
    assert_eq!(a, b, "the same path string must hash to the same AssetId every time");
}

#[test]
fn from_path_differs_for_different_inputs() {
    let a = AssetId::from_path("models/character.gltf#texture/albedo");
    let b = AssetId::from_path("models/character.gltf#texture/normal");
    assert_ne!(a, b, "distinct sub-asset names must hash to distinct IDs");
}

#[test]
fn round_trips_through_bincode() {
    let id = AssetId::from_path("models/character.gltf#scene");
    let bytes = bincode::serialize(&id).unwrap();
    let decoded: AssetId = bincode::deserialize(&bytes).unwrap();
    assert_eq!(decoded, id);
}
```

- [ ] **Step 2: Run tests to verify they fail**

Run: `cargo test -p essential --test asset_id`
Expected: FAIL to compile — `AssetId::from_path` doesn't exist, and `AssetId` isn't `Serialize`.

- [ ] **Step 3: Implement**

Add to `crates/essential/Cargo.toml`:
```toml
serde = { version = "1", features = ["derive"] }
bincode = "1.3"
```
Change the existing `uuid` dependency line to enable its `serde` feature:
```toml
uuid = { version = "1.18.1", features = ["v4", "v5", "js", "serde"] }
```

In `crates/essential/src/assets/mod.rs`, change `AssetId`:

```rust
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct AssetId(Uuid);

/// Fixed namespace for deriving AssetIds from asset paths, so the same
/// path string always hashes to the same UUID (v5) regardless of process
/// or machine. Generated once via `uuid::Uuid::new_v4()` and hard-coded —
/// it must never change once assets have been cooked with it.
const ASSET_PATH_NAMESPACE: Uuid = Uuid::from_bytes([
    0x6d, 0x1a, 0x9a, 0x3e, 0x2f, 0x0b, 0x4a, 0x77,
    0x8e, 0x92, 0x1a, 0x64, 0xaf, 0x03, 0x5c, 0x11,
]);

impl AssetId {
    pub fn new() -> Self {
        AssetId(Uuid::new_v4())
    }

    /// Deterministically derives a stable AssetId from a full asset address
    /// string (e.g. "models/character.gltf#texture/albedo"). The same
    /// string always produces the same ID, with no shared state required —
    /// this is what lets the cook tool and the runtime independently agree
    /// on an asset's identity and cooked-file location.
    pub fn from_path(path: &str) -> Self {
        AssetId(Uuid::new_v5(&ASSET_PATH_NAMESPACE, path.as_bytes()))
    }
}
```

Leave `impl Default for AssetId` as-is (still calls `Self::new()`).

- [ ] **Step 4: Run tests to verify they pass**

Run: `cargo test -p essential --test asset_id`
Expected: PASS (all 3 tests)

- [ ] **Step 5: Build to check for regressions**

Run: `cargo build -p essential --tests`
Expected: builds clean, no warnings

- [ ] **Step 6: Commit**

```bash
git add crates/essential/Cargo.toml crates/essential/src/assets/mod.rs crates/essential/tests/asset_id.rs
git commit -m "feat(essential): add deterministic AssetId::from_path and make AssetId serializable"
```

---

### Task 2: `AssetHandle<T>` becomes a serializable `Strong`/`Weak` enum; `AssetServer::load_by_id`

**Files:**
- Modify: `crates/essential/src/assets/handle.rs`
- Modify: `crates/essential/src/assets/asset_server.rs`
- Test: `crates/essential/tests/asset_handle.rs` (new)

**Interfaces:**
- Consumes: `AssetId::from_path` (Task 1).
- Produces: `enum AssetHandle<A: Asset> { Strong(Arc<StrongAssetHandle>, PhantomData<A>), Weak(AssetId, PhantomData<A>) }` with `Serialize`/`Deserialize` (writes/reads just the `AssetId`), `AssetHandle::weak(id: AssetId) -> Self` (public — used by importers building references), `.id()` unchanged in behavior; `AssetServer::load_by_id::<A: LoadableAsset>(&self, id: AssetId) -> AssetHandle<A>`; `AssetServer::load` now derives its `AssetId` via `AssetId::from_path` on the full path string (including fragment) instead of a random one, then delegates to `load_by_id`.

- [ ] **Step 1: Write the failing tests**

Create `crates/essential/tests/asset_handle.rs`:

```rust
//! Covers AssetHandle's Strong/Weak split and its serialization contract:
//! any handle (regardless of variant) serializes to its bare AssetId, and
//! deserializing always produces a Weak handle (never a live Strong one,
//! since deserialization has no AssetServer to resolve against).
use essential::assets::{handle::AssetHandle, Asset, AssetId};

struct FakeAsset;
impl Asset for FakeAsset {
    fn name() -> &'static str {
        "FakeAsset"
    }
}

#[test]
fn weak_handle_serializes_to_its_id() {
    let id = AssetId::from_path("models/character.gltf#texture/albedo");
    let handle: AssetHandle<FakeAsset> = AssetHandle::weak(id);

    let bytes = bincode::serialize(&handle).unwrap();
    let decoded: AssetHandle<FakeAsset> = bincode::deserialize(&bytes).unwrap();

    assert_eq!(decoded.id(), id, "round-tripping a handle must preserve its AssetId");
}

#[test]
fn deserialized_handle_is_weak_and_id_matches() {
    let id = AssetId::from_path("models/character.gltf#mesh/0");
    let bytes = bincode::serialize(&id).unwrap();
    let decoded: AssetHandle<FakeAsset> = bincode::deserialize(&bytes).unwrap();
    assert_eq!(decoded.id(), id, "deserializing a bare AssetId must produce a handle with that ID");
}
```

- [ ] **Step 2: Run tests to verify they fail**

Run: `cargo test -p essential --test asset_handle`
Expected: FAIL to compile — `AssetHandle::weak` and `Serialize`/`Deserialize` for `AssetHandle` don't exist.

- [ ] **Step 3: Implement the `Strong`/`Weak` split**

Rewrite the relevant parts of `crates/essential/src/assets/handle.rs`. Keep `StrongAssetHandle` and its `Drop` impl (sends `AssetLifetimeEvent::Dropped`) exactly as they are today. Replace the `AssetHandle` struct with:

```rust
use std::marker::PhantomData;
use std::sync::Arc;
use serde::{Deserialize, Deserializer, Serialize, Serializer};

use super::{Asset, AssetId};

pub enum AssetHandle<A: Asset> {
    Strong(Arc<StrongAssetHandle>, PhantomData<A>),
    Weak(AssetId, PhantomData<A>),
}

impl<A: Asset> AssetHandle<A> {
    pub(crate) fn strong(handle: Arc<StrongAssetHandle>) -> Self {
        AssetHandle::Strong(handle, PhantomData)
    }

    pub fn weak(id: AssetId) -> Self {
        AssetHandle::Weak(id, PhantomData)
    }

    pub fn id(&self) -> AssetId {
        match self {
            AssetHandle::Strong(handle, _) => handle.id,
            AssetHandle::Weak(id, _) => *id,
        }
    }
}

impl<A: Asset> Clone for AssetHandle<A> {
    fn clone(&self) -> Self {
        match self {
            AssetHandle::Strong(handle, _) => AssetHandle::Strong(handle.clone(), PhantomData),
            AssetHandle::Weak(id, _) => AssetHandle::Weak(*id, PhantomData),
        }
    }
}

impl<A: Asset> std::fmt::Debug for AssetHandle<A> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            AssetHandle::Strong(_, _) => write!(f, "AssetHandle::Strong({:?})", self.id()),
            AssetHandle::Weak(id, _) => write!(f, "AssetHandle::Weak({id:?})"),
        }
    }
}

impl<A: Asset> Serialize for AssetHandle<A> {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        self.id().serialize(serializer)
    }
}

impl<'de, A: Asset> Deserialize<'de> for AssetHandle<A> {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        let id = AssetId::deserialize(deserializer)?;
        Ok(AssetHandle::weak(id))
    }
}
```

**Find every existing call site of the old `AssetHandle::new(arc)` constructor and `.handle`/`._marker` field access** (grep `AssetHandle::new\(` and any direct field access across `crates/essential/src/assets/asset_server.rs` and elsewhere) and update them to `AssetHandle::strong(arc)` / the `.id()` method, which already exists and is unaffected in behavior.

- [ ] **Step 4: Add `AssetServer::load_by_id` and rewire `load`**

In `crates/essential/src/assets/asset_server.rs`, locate `load_internal` (the private method both `load` and `load_with_usage_settings` funnel through) and change it so the `AssetId` it uses is derived from the path instead of freshly randomly generated. Add a new public method:

```rust
impl AssetServer {
    pub fn load_by_id<A: LoadableAsset + 'static>(&self, id: AssetId) -> AssetHandle<A> {
        // Reuse the existing per-ID handle dedup (AssetHandleProvider.asset_handles)
        // exactly as load_internal does today, then request_load::<A>(id, A::default_usage_settings())
        // if not already loaded/loading. The only behavioral difference from today's
        // load_internal is that there is no AssetPath to store on StrongAssetHandle —
        // pass None for the `path` field (used today only for AssetLifetimeEvent
        // debugging/logging), since callers of load_by_id only ever have an AssetId.
    }
}
```

Implement `load_by_id` by extracting the existing dedup-and-request-load logic out of `load_internal` into this new method (so `load_internal` becomes: compute `let id = AssetId::from_path(&path_string_with_fragment); self.load_by_id::<A>(id)`, after storing the human path/usage-settings mapping needed only for the request-load task to know what file to actually parse — see Step 5's note on `AssetLoadContext`).

**Verify during implementation:** the exact current body of `load_internal`/`request_load` in `asset_server.rs` (already dumped in full during exploration — re-read the file before this edit) to make sure the extraction preserves today's dedup-via-weak-handle-reuse behavior exactly; this is a refactor of existing working code, not new logic, so the safest approach is "extract method," not "rewrite from scratch."

- [ ] **Step 5: Give `AssetLoadContext` access to the resolved `AssetId`**

Cooked-format loaders (Texture, Mesh, StandardMaterial, Scene — added in later tasks) need to know which `AssetId` they're loading in order to compute their cooked file's flat location, even when the load originated from `load_by_id` with no path string at all. Add to `AssetLoadContext` (`asset_server.rs`):

```rust
pub struct AssetLoadContext {
    asset_server: AssetServer,
    asset_id: AssetId,
}

impl AssetLoadContext {
    pub fn asset_server(&self) -> &AssetServer {
        &self.asset_server
    }

    pub fn asset_id(&self) -> AssetId {
        self.asset_id
    }

    pub(crate) fn new(asset_server: AssetServer, asset_id: AssetId) -> Self {
        Self { asset_server, asset_id }
    }
}
```

Update the one existing call site of `AssetLoadContext::new` (inside `request_load`) to pass the `id` already in scope there. This is additive — every existing `AssetLoader` impl that never calls `.asset_id()` (animation, ui, skybox loaders, unrelated to this plan) is completely unaffected; only the cooked-format loaders added later in this plan will call it.

- [ ] **Step 6: Run tests, then the full essential suite**

Run: `cargo test -p essential --test asset_handle && cargo test -p essential`
Expected: PASS. Fix any compile errors in other files within `essential` (and, if `cargo build --workspace` surfaces them, in other crates) caused by the `AssetHandle::new` → `AssetHandle::strong` rename — grep the whole repo for `AssetHandle::new(` to find every call site.

- [ ] **Step 7: Build the whole workspace to catch downstream breakage early**

Run: `cargo build --workspace`
Expected: builds clean. Every crate that constructs or matches on `AssetHandle<T>` directly (rather than only calling `.id()`/`.clone()`) needs updating — this is worth doing now, in this task, rather than discovering it piecemeal in later tasks.

- [ ] **Step 8: Commit**

```bash
git add crates/essential
git commit -m "feat(essential): make AssetHandle a serializable Strong/Weak enum; add AssetServer::load_by_id"
```

---

### Task 3: `asset-cook` crate — `Importer` trait, `ImportContext`, `CookedAsset`

**Files:**
- Create: `crates/asset-cook/Cargo.toml`
- Create: `crates/asset-cook/src/lib.rs`
- Create: `crates/asset-cook/src/import_context.rs`
- Test: `crates/asset-cook/tests/import_context.rs`

**Interfaces:**
- Consumes: `AssetId::from_path` (Task 1).
- Produces: `trait CookedAsset: Serialize + DeserializeOwned { const TYPE_NAME: &'static str; fn referenced_sub_assets(&self) -> Vec<AssetId> { Vec::new() } }`; `struct EmittedSubAsset { name: String, asset_id: AssetId, type_name: &'static str, bytes: Vec<u8>, references: Vec<AssetId> }`; `struct ImportContext { relative_source: PathBuf, sub_assets: Vec<EmittedSubAsset>, dependencies: Vec<DependencyEntry> }` with `emit`, `track_dependency`, `sub_asset_id(name: &str) -> AssetId` (computes `AssetId::from_path("<relative_source>#<name>")` — the mechanism same-file references use to point at a sibling sub-asset without knowing the final on-disk layout), `into_parts`; `struct DependencyEntry { path: PathBuf, content_hash: u64 }`; `enum ImportError { ... }` (same variants as before); `trait Importer: Send + Sync { fn supported_extensions(&self) -> &'static [&'static str]; fn import(&self, source_path: &Path, ctx: &mut ImportContext) -> Result<(), ImportError>; fn validate(&self, sub_assets: &[EmittedSubAsset]) -> Vec<ValidationIssue> { Vec::new() } }`; `enum ValidationSeverity { Warning, Error }`; `struct ValidationIssue { severity: ValidationSeverity, message: String, source_path: PathBuf, sub_asset_name: Option<String> }`.

- [ ] **Step 1: Create the crate skeleton**

`crates/asset-cook/Cargo.toml`:

```toml
[package]
name = "asset-cook"
version = "0.1.0"
edition = "2021"

[dependencies]
essential = { path = "../essential" }
serde = { version = "1", features = ["derive"] }
bincode = "1.3"
anyhow = "1.0.97"

[dev-dependencies]
serde = { version = "1", features = ["derive"] }
```

`crates/asset-cook/src/lib.rs`:

```rust
mod import_context;

pub use import_context::{DependencyEntry, EmittedSubAsset, ImportContext, ImportError};

use std::path::{Path, PathBuf};

use essential::assets::AssetId;
use serde::{de::DeserializeOwned, Serialize};

/// A cooked, on-disk representation of one engine asset. Implemented
/// directly on the real asset type wherever possible (e.g. `StandardMaterial`,
/// `Scene`) — a separate DTO is only introduced when the live type holds
/// data that genuinely can't serialize (e.g. GPU descriptor types), never
/// merely because it holds an `AssetHandle<T>` field, since `AssetHandle<T>`
/// is itself serializable.
pub trait CookedAsset: Serialize + DeserializeOwned {
    const TYPE_NAME: &'static str;

    /// AssetIds of every other sub-asset this one references. Used by the
    /// cook tool's global reference-integrity validation pass.
    fn referenced_sub_assets(&self) -> Vec<AssetId> {
        Vec::new()
    }
}

pub trait Importer: Send + Sync {
    fn supported_extensions(&self) -> &'static [&'static str];

    fn import(&self, source_path: &Path, ctx: &mut ImportContext) -> Result<(), ImportError>;

    fn validate(&self, sub_assets: &[EmittedSubAsset]) -> Vec<ValidationIssue> {
        Vec::new()
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ValidationSeverity {
    Warning,
    Error,
}

#[derive(Debug, Clone)]
pub struct ValidationIssue {
    pub severity: ValidationSeverity,
    pub message: String,
    pub source_path: PathBuf,
    pub sub_asset_name: Option<String>,
}
```

`crates/asset-cook/src/import_context.rs`:

```rust
use std::path::PathBuf;

use essential::assets::AssetId;

use crate::CookedAsset;

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct DependencyEntry {
    pub path: PathBuf,
    pub content_hash: u64,
}

#[derive(Debug, Clone)]
pub struct EmittedSubAsset {
    pub name: String,
    pub asset_id: AssetId,
    pub type_name: &'static str,
    pub bytes: Vec<u8>,
    pub references: Vec<AssetId>,
}

#[derive(Debug, Clone)]
pub enum ImportError {
    SourceUnreadable { source_path: PathBuf, message: String },
    MalformedSource { source_path: PathBuf, message: String },
    MissingRequiredData { source_path: PathBuf, message: String },
    SerializationFailed { sub_asset_name: String, message: String },
}

pub struct ImportContext {
    relative_source: PathBuf,
    sub_assets: Vec<EmittedSubAsset>,
    dependencies: Vec<DependencyEntry>,
}

impl ImportContext {
    pub fn new(relative_source: PathBuf) -> Self {
        Self { relative_source, sub_assets: Vec::new(), dependencies: Vec::new() }
    }

    /// Computes the stable AssetId a sub-asset name resolves to *within this
    /// source file*, without needing to know the final on-disk cooked
    /// layout. Importers use this to build same-file cross-references
    /// (e.g. a material's texture, a scene node's mesh) as real
    /// `AssetHandle::weak(id)` values on the structs they emit.
    pub fn sub_asset_id(&self, name: &str) -> AssetId {
        AssetId::from_path(&format!("{}#{}", self.relative_source.display(), name))
    }

    pub fn emit<T: CookedAsset>(&mut self, name: &str, value: &T) -> Result<(), ImportError> {
        let bytes = bincode::serialize(value).map_err(|err| ImportError::SerializationFailed {
            sub_asset_name: name.to_string(),
            message: err.to_string(),
        })?;

        self.sub_assets.push(EmittedSubAsset {
            name: name.to_string(),
            asset_id: self.sub_asset_id(name),
            type_name: T::TYPE_NAME,
            bytes,
            references: value.referenced_sub_assets(),
        });

        Ok(())
    }

    pub fn track_dependency(&mut self, path: PathBuf, content_hash: u64) {
        self.dependencies.push(DependencyEntry { path, content_hash });
    }

    pub fn into_parts(self) -> (Vec<EmittedSubAsset>, Vec<DependencyEntry>) {
        (self.sub_assets, self.dependencies)
    }
}
```

- [ ] **Step 2: Write the failing test**

Create `crates/asset-cook/tests/import_context.rs`:

```rust
//! Covers ImportContext's sub-asset emission, same-file reference-ID
//! computation, and dependency tracking.
use asset_cook::{CookedAsset, ImportContext};
use essential::assets::AssetId;
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize)]
struct FakeCookedThing {
    referenced: AssetId,
}

impl CookedAsset for FakeCookedThing {
    const TYPE_NAME: &'static str = "FakeThing";

    fn referenced_sub_assets(&self) -> Vec<AssetId> {
        vec![self.referenced]
    }
}

#[test]
fn sub_asset_id_is_stable_and_scoped_to_the_source_file() {
    let ctx = ImportContext::new(std::path::PathBuf::from("models/character.gltf"));
    let id_a = ctx.sub_asset_id("mesh/0");
    let id_b = ctx.sub_asset_id("mesh/0");
    assert_eq!(id_a, id_b, "the same name in the same context must always resolve to the same id");
    assert_eq!(
        id_a,
        AssetId::from_path("models/character.gltf#mesh/0"),
        "sub_asset_id must match what a runtime load of the fully-qualified path would compute"
    );
}

#[test]
fn emit_records_sub_asset_with_serialized_bytes_and_references() {
    let mut ctx = ImportContext::new(std::path::PathBuf::from("models/character.gltf"));
    let referenced_id = ctx.sub_asset_id("texture/albedo");
    let thing = FakeCookedThing { referenced: referenced_id };

    ctx.emit("material/0", &thing).expect("emit should succeed for a serializable value");

    let (sub_assets, _dependencies) = ctx.into_parts();
    assert_eq!(sub_assets.len(), 1);

    let entry = &sub_assets[0];
    assert_eq!(entry.name, "material/0");
    assert_eq!(entry.asset_id, AssetId::from_path("models/character.gltf#material/0"));
    assert_eq!(entry.type_name, "FakeThing");
    assert_eq!(entry.references, vec![referenced_id]);

    let round_tripped: FakeCookedThing =
        bincode::deserialize(&entry.bytes).expect("emitted bytes must deserialize back");
    assert_eq!(round_tripped.referenced, referenced_id);
}

#[test]
fn track_dependency_records_path_and_hash() {
    let mut ctx = ImportContext::new(std::path::PathBuf::from("models/character.gltf"));
    ctx.track_dependency(std::path::PathBuf::from("assets/models/character.bin"), 12345);

    let (_sub_assets, dependencies) = ctx.into_parts();
    assert_eq!(dependencies.len(), 1);
    assert_eq!(dependencies[0].path, std::path::PathBuf::from("assets/models/character.bin"));
    assert_eq!(dependencies[0].content_hash, 12345);
}
```

- [ ] **Step 3: Run test to verify it fails**

Run: `cargo test -p asset-cook --test import_context`
Expected: FAIL — crate doesn't exist yet.

- [ ] **Step 4: Run test to verify it passes**

Run: `cargo test -p asset-cook --test import_context`
Expected: PASS (3 tests)

- [ ] **Step 5: Commit**

```bash
git add crates/asset-cook
git commit -m "feat(asset-cook): add Importer trait, ImportContext with stable sub-asset IDs, CookedAsset trait"
```

---

### Task 4: Manifest parsing + single-source cook orchestration (flat, ID-keyed output)

**Files:**
- Create: `crates/asset-cook/src/manifest.rs`
- Create: `crates/asset-cook/src/cook.rs`
- Modify: `crates/asset-cook/src/lib.rs`
- Modify: `crates/asset-cook/Cargo.toml` (add `toml`)
- Test: `crates/asset-cook/tests/cook.rs`

**Interfaces:**
- Consumes: `Importer`, `ImportContext`, `EmittedSubAsset`, `DependencyEntry` (Task 3).
- Produces: `struct AssetManifest { assets: Vec<ManifestEntry> }`, `struct ManifestEntry { path: String }`, `AssetManifest::load(path: &Path) -> anyhow::Result<Self>`; `struct SubAssetEntry { name: String, asset_id: AssetId, type_name: String, references: Vec<AssetId> }` (no `cooked_path` field — it's always derivable from `asset_id` alone via `cooked_file_path_for_id`); `struct SourceIndex { source_path: PathBuf, source_hash: u64, sub_assets: Vec<SubAssetEntry>, dependencies: Vec<DependencyEntry> }`; `struct CookOptions { manifest_path: PathBuf, source_root: PathBuf, output_root: PathBuf }`; `fn cooked_file_path_for_id(output_root: &Path, id: AssetId) -> PathBuf` (`output_root/.cooked/<id-simple-hex>.bin`); `fn cook_source(importer: &dyn Importer, source_path: &Path, relative_source: &Path, output_root: &Path) -> Result<SourceIndex, ImportError>`.

- [ ] **Step 1: Write the failing test**

Create `crates/asset-cook/tests/cook.rs`:

```rust
//! Covers cooking a single source file end-to-end: an Importer emits
//! sub-assets, cook_source writes each to its flat, AssetId-keyed location.
use std::path::Path;

use asset_cook::{cook_source, cooked_file_path_for_id, CookedAsset, ImportContext, ImportError, Importer};
use essential::assets::AssetId;
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize)]
struct FakeCookedThing {
    value: u32,
}

impl CookedAsset for FakeCookedThing {
    const TYPE_NAME: &'static str = "FakeThing";
}

struct FakeImporter;

impl Importer for FakeImporter {
    fn supported_extensions(&self) -> &'static [&'static str] {
        &["fake"]
    }

    fn import(&self, _source_path: &Path, ctx: &mut ImportContext) -> Result<(), ImportError> {
        ctx.emit("thing/0", &FakeCookedThing { value: 7 }).unwrap();
        ctx.emit("thing/1", &FakeCookedThing { value: 9 }).unwrap();
        Ok(())
    }
}

#[test]
fn cook_source_writes_one_flat_file_per_sub_asset() {
    let temp_dir = std::env::temp_dir().join(format!("asset-cook-test-{}", std::process::id()));
    let source_root = temp_dir.join("assets");
    let output_root = temp_dir.join("res");
    std::fs::create_dir_all(source_root.join("models")).unwrap();
    let relative_source = Path::new("models/character.fake");
    let source_path = source_root.join(relative_source);
    std::fs::write(&source_path, b"fake source content").unwrap();

    let index = cook_source(&FakeImporter, &source_path, relative_source, &output_root)
        .expect("cooking a valid fake source should succeed");

    assert_eq!(index.sub_assets.len(), 2, "both emitted sub-assets must appear in the index");

    let expected_id = AssetId::from_path("models/character.fake#thing/0");
    assert_eq!(index.sub_assets[0].asset_id, expected_id);

    let cooked_path = cooked_file_path_for_id(&output_root, expected_id);
    assert!(cooked_path.exists(), "cooked file must exist at the deterministic ID-keyed path");

    let bytes = std::fs::read(&cooked_path).unwrap();
    let decoded: FakeCookedThing = bincode::deserialize(&bytes).unwrap();
    assert_eq!(decoded.value, 7);

    std::fs::remove_dir_all(&temp_dir).ok();
}
```

- [ ] **Step 2: Run test to verify it fails**

Run: `cargo test -p asset-cook --test cook`
Expected: FAIL to compile.

- [ ] **Step 3: Implement manifest + cook_source**

Add to `crates/asset-cook/Cargo.toml`:
```toml
toml = "0.8"
```

`crates/asset-cook/src/manifest.rs`:

```rust
use std::path::Path;

use serde::Deserialize;

#[derive(Debug, Deserialize)]
pub struct ManifestEntry {
    pub path: String,
}

#[derive(Debug, Deserialize)]
pub struct AssetManifest {
    pub assets: Vec<ManifestEntry>,
}

impl AssetManifest {
    pub fn load(path: &Path) -> anyhow::Result<Self> {
        let text = std::fs::read_to_string(path)?;
        Ok(toml::from_str(&text)?)
    }
}
```

`crates/asset-cook/src/cook.rs`:

```rust
use std::path::{Path, PathBuf};

use essential::assets::AssetId;

use crate::{DependencyEntry, ImportContext, ImportError, Importer};

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct SubAssetEntry {
    pub name: String,
    pub asset_id: AssetId,
    pub type_name: String,
    pub references: Vec<AssetId>,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct SourceIndex {
    pub source_path: PathBuf,
    pub source_hash: u64,
    pub sub_assets: Vec<SubAssetEntry>,
    pub dependencies: Vec<DependencyEntry>,
}

pub struct CookOptions {
    pub manifest_path: PathBuf,
    pub source_root: PathBuf,
    pub output_root: PathBuf,
}

/// The cooked file location for any AssetId is a pure function of the ID
/// alone — no index, no source path needed. This is what lets a nested
/// reference (which only ever carries an AssetId, never a path) resolve to
/// its cooked bytes with no lookup.
pub fn cooked_file_path_for_id(output_root: &Path, id: AssetId) -> PathBuf {
    output_root.join(".cooked").join(format!("{}.bin", id_to_hex(id)))
}

fn id_to_hex(id: AssetId) -> String {
    // AssetId wraps a Uuid; format it as a plain hex string (no hyphens) for
    // a filesystem-friendly filename. Uses AssetId's Debug output and
    // strips non-hex characters rather than reaching into Uuid internals,
    // since AssetId intentionally doesn't expose its inner Uuid publicly.
    format!("{id:?}")
        .chars()
        .filter(|c| c.is_ascii_hexdigit())
        .collect()
}

pub fn hash_file_contents(path: &Path) -> Result<u64, ImportError> {
    use std::hash::{DefaultHasher, Hash, Hasher};

    let bytes = std::fs::read(path).map_err(|err| ImportError::SourceUnreadable {
        source_path: path.to_path_buf(),
        message: err.to_string(),
    })?;

    let mut hasher = DefaultHasher::new();
    bytes.hash(&mut hasher);
    Ok(hasher.finish())
}

/// Cooks a single source file with the given importer, writing one flat,
/// ID-keyed file per emitted sub-asset under `output_root/.cooked/`.
pub fn cook_source(
    importer: &dyn Importer,
    source_path: &Path,
    relative_source: &Path,
    output_root: &Path,
) -> Result<SourceIndex, ImportError> {
    let source_hash = hash_file_contents(source_path)?;

    let mut ctx = ImportContext::new(relative_source.to_path_buf());
    importer.import(source_path, &mut ctx)?;
    let (sub_assets, dependencies) = ctx.into_parts();

    let cooked_dir = output_root.join(".cooked");
    std::fs::create_dir_all(&cooked_dir).map_err(|err| ImportError::SourceUnreadable {
        source_path: source_path.to_path_buf(),
        message: format!("failed to create cooked output dir: {err}"),
    })?;

    let mut entries = Vec::with_capacity(sub_assets.len());
    for sub_asset in sub_assets {
        let cooked_path = cooked_file_path_for_id(output_root, sub_asset.asset_id);
        std::fs::write(&cooked_path, &sub_asset.bytes).map_err(|err| {
            ImportError::SerializationFailed {
                sub_asset_name: sub_asset.name.clone(),
                message: format!("failed to write cooked file: {err}"),
            }
        })?;

        entries.push(SubAssetEntry {
            name: sub_asset.name,
            asset_id: sub_asset.asset_id,
            type_name: sub_asset.type_name.to_string(),
            references: sub_asset.references,
        });
    }

    Ok(SourceIndex {
        source_path: source_path.to_path_buf(),
        source_hash,
        sub_assets: entries,
        dependencies,
    })
}
```

Update `crates/asset-cook/src/lib.rs` to add:
```rust
mod cook;
mod manifest;

pub use cook::{cook_source, cooked_file_path_for_id, hash_file_contents, CookOptions, SourceIndex, SubAssetEntry};
pub use manifest::{AssetManifest, ManifestEntry};
```

- [ ] **Step 4: Run test to verify it passes**

Run: `cargo test -p asset-cook --test cook`
Expected: PASS

- [ ] **Step 5: Commit**

```bash
git add crates/asset-cook
git commit -m "feat(asset-cook): add manifest parsing and cook_source with flat ID-keyed output"
```

---

### Task 5: Incremental rebuild (skip unchanged sources)

**Files:**
- Create: `crates/asset-cook/src/run.rs`
- Modify: `crates/asset-cook/src/lib.rs`
- Test: `crates/asset-cook/tests/incremental.rs`

**Interfaces:**
- Consumes: `AssetManifest`, `cook_source`, `SourceIndex`, `hash_file_contents` (Task 4).
- Produces: `struct CookReport { cooked: Vec<PathBuf>, skipped: Vec<PathBuf>, errors: Vec<ImportError> }`; `fn run_cook(importers: &[Box<dyn Importer>], options: &CookOptions) -> CookReport`. A persisted `SourceIndex` (bincode) lives at `output_root/.index/<relative-source-with-slashes-replaced-by-underscores>.bin` — this bookkeeping file, unlike the sub-asset payloads, is mirrored by source path since incremental-skip logic always has the relative source path in hand while iterating the manifest.

- [ ] **Step 1: Write the failing test**

Create `crates/asset-cook/tests/incremental.rs`:

```rust
//! Covers incremental cook skip logic: unchanged sources (and their tracked
//! dependencies) are not re-imported on a second cook run.
use std::path::Path;
use std::sync::atomic::{AtomicUsize, Ordering};

use asset_cook::{run_cook, CookOptions, CookedAsset, ImportContext, ImportError, Importer};
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize)]
struct CountedThing {
    value: u32,
}

impl CookedAsset for CountedThing {
    const TYPE_NAME: &'static str = "CountedThing";
}

struct CountingImporter {
    import_count: AtomicUsize,
}

impl Importer for CountingImporter {
    fn supported_extensions(&self) -> &'static [&'static str] {
        &["fake"]
    }

    fn import(&self, source_path: &Path, ctx: &mut ImportContext) -> Result<(), ImportError> {
        self.import_count.fetch_add(1, Ordering::SeqCst);
        let dep_path = source_path.with_extension("dep");
        let dep_hash = asset_cook::hash_file_contents(&dep_path)?;
        ctx.track_dependency(dep_path, dep_hash);
        ctx.emit("thing/0", &CountedThing { value: 1 }).unwrap();
        Ok(())
    }
}

fn setup() -> (std::path::PathBuf, std::path::PathBuf, std::path::PathBuf) {
    let temp_dir = std::env::temp_dir().join(format!("asset-cook-incremental-{}", std::process::id()));
    let source_root = temp_dir.join("assets");
    let output_root = temp_dir.join("res");
    std::fs::create_dir_all(&source_root).unwrap();
    std::fs::write(source_root.join("thing.fake"), b"source v1").unwrap();
    std::fs::write(source_root.join("thing.dep"), b"dep v1").unwrap();

    let manifest_path = temp_dir.join("assets.toml");
    std::fs::write(&manifest_path, "[[assets]]\npath = \"thing.fake\"\n").unwrap();

    (manifest_path, source_root, output_root)
}

#[test]
fn second_cook_skips_unchanged_source_and_dependency() {
    let (manifest_path, source_root, output_root) = setup();
    let importers: Vec<Box<dyn Importer>> = vec![Box::new(CountingImporter { import_count: AtomicUsize::new(0) })];
    let options = CookOptions { manifest_path, source_root: source_root.clone(), output_root };

    let first = run_cook(&importers, &options);
    assert_eq!(first.errors.len(), 0, "first cook must succeed: {:?}", first.errors);
    assert_eq!(first.cooked.len(), 1);

    let second = run_cook(&importers, &options);
    assert_eq!(second.errors.len(), 0);
    assert_eq!(second.cooked.len(), 0, "nothing changed, so nothing should be re-cooked");
    assert_eq!(second.skipped.len(), 1);

    std::fs::remove_dir_all(source_root.parent().unwrap()).ok();
}

#[test]
fn changing_a_tracked_dependency_forces_reimport() {
    let (manifest_path, source_root, output_root) = setup();
    let importers: Vec<Box<dyn Importer>> = vec![Box::new(CountingImporter { import_count: AtomicUsize::new(0) })];
    let options = CookOptions { manifest_path, source_root: source_root.clone(), output_root };

    run_cook(&importers, &options);
    std::fs::write(source_root.join("thing.dep"), b"dep v2 - changed").unwrap();
    let second = run_cook(&importers, &options);

    assert_eq!(second.cooked.len(), 1, "a changed dependency must force the source to re-import");
    assert_eq!(second.skipped.len(), 0);

    std::fs::remove_dir_all(source_root.parent().unwrap()).ok();
}
```

- [ ] **Step 2: Run test to verify it fails**

Run: `cargo test -p asset-cook --test incremental`
Expected: FAIL to compile — `run_cook`/`CookReport` don't exist.

- [ ] **Step 3: Implement `run_cook`**

Create `crates/asset-cook/src/run.rs`:

```rust
use std::path::PathBuf;

use crate::{cook_source, hash_file_contents, AssetManifest, CookOptions, ImportError, Importer, SourceIndex};

#[derive(Debug, Default)]
pub struct CookReport {
    pub cooked: Vec<PathBuf>,
    pub skipped: Vec<PathBuf>,
    pub errors: Vec<ImportError>,
}

fn index_path_for(output_root: &std::path::Path, relative_source: &std::path::Path) -> PathBuf {
    let flattened = relative_source.to_string_lossy().replace(['/', '\\'], "_");
    output_root.join(".index").join(format!("{flattened}.bin"))
}

fn source_is_unchanged(existing: &SourceIndex, source_path: &std::path::Path) -> bool {
    let Ok(current_source_hash) = hash_file_contents(source_path) else {
        return false;
    };
    if current_source_hash != existing.source_hash {
        return false;
    }

    existing.dependencies.iter().all(|dependency| {
        matches!(hash_file_contents(&dependency.path), Ok(hash) if hash == dependency.content_hash)
    })
}

pub fn run_cook(importers: &[Box<dyn Importer>], options: &CookOptions) -> CookReport {
    let mut report = CookReport::default();

    let manifest = match AssetManifest::load(&options.manifest_path) {
        Ok(manifest) => manifest,
        Err(err) => {
            report.errors.push(ImportError::SourceUnreadable {
                source_path: options.manifest_path.clone(),
                message: err.to_string(),
            });
            return report;
        }
    };

    for entry in manifest.assets {
        let relative_source = PathBuf::from(&entry.path);
        let source_path = options.source_root.join(&relative_source);

        let extension = source_path.extension().and_then(|ext| ext.to_str()).unwrap_or_default();
        let Some(importer) = importers.iter().find(|i| i.supported_extensions().contains(&extension)) else {
            report.errors.push(ImportError::MalformedSource {
                source_path: source_path.clone(),
                message: format!("no importer registered for extension '{extension}'"),
            });
            continue;
        };

        let index_path = index_path_for(&options.output_root, &relative_source);
        if let Ok(existing_bytes) = std::fs::read(&index_path) {
            if let Ok(existing_index) = bincode::deserialize::<SourceIndex>(&existing_bytes) {
                if source_is_unchanged(&existing_index, &source_path) {
                    report.skipped.push(source_path);
                    continue;
                }
            }
        }

        match cook_source(importer.as_ref(), &source_path, &relative_source, &options.output_root) {
            Ok(index) => {
                std::fs::create_dir_all(index_path.parent().unwrap()).ok();
                if let Ok(bytes) = bincode::serialize(&index) {
                    let _ = std::fs::write(&index_path, bytes);
                }
                report.cooked.push(source_path);
            }
            Err(err) => report.errors.push(err),
        }
    }

    report
}
```

Update `crates/asset-cook/src/lib.rs`:
```rust
mod run;
pub use run::{run_cook, CookReport};
```

- [ ] **Step 4: Run tests to verify they pass**

Run: `cargo test -p asset-cook --test incremental`
Expected: PASS

- [ ] **Step 5: Run all asset-cook tests**

Run: `cargo test -p asset-cook`
Expected: PASS across all test files

- [ ] **Step 6: Commit**

```bash
git add crates/asset-cook
git commit -m "feat(asset-cook): add incremental rebuild via persisted SourceIndex hashing"
```

---

### Task 6: Global reference-integrity validation + per-importer `validate()` hook

**Files:**
- Modify: `crates/asset-cook/src/run.rs`
- Modify: `crates/asset-cook/Cargo.toml` (add `log`)
- Test: `crates/asset-cook/tests/validation.rs`

**Interfaces:**
- `run_cook` now also runs a global reference-integrity pass after cooking all sources (checking every `SubAssetEntry.references` `AssetId` resolves to some produced `SubAssetEntry.asset_id`), appending `ImportError::MissingRequiredData` entries for unresolved references, and runs each importer's `validate()` hook per freshly-cooked source, treating `ValidationSeverity::Error` as a report error and logging `Warning` via `log::warn!`.

- [ ] **Step 1: Write the failing test**

Create `crates/asset-cook/tests/validation.rs`:

```rust
//! Covers cross-source reference-integrity checking and the per-Importer
//! validate() hook, both of which must fail a cook run when triggered.
use std::path::Path;

use asset_cook::{
    run_cook, CookOptions, CookedAsset, EmittedSubAsset, ImportContext, ImportError, Importer,
    ValidationIssue, ValidationSeverity,
};
use essential::assets::AssetId;
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize)]
struct RefThing {
    references: Vec<AssetId>,
}

impl CookedAsset for RefThing {
    const TYPE_NAME: &'static str = "RefThing";
    fn referenced_sub_assets(&self) -> Vec<AssetId> {
        self.references.clone()
    }
}

struct DanglingRefImporter;

impl Importer for DanglingRefImporter {
    fn supported_extensions(&self) -> &'static [&'static str] {
        &["fake"]
    }
    fn import(&self, _source_path: &Path, ctx: &mut ImportContext) -> Result<(), ImportError> {
        let dangling = AssetId::from_path("models/does_not_exist.fake#thing/0");
        ctx.emit("thing/0", &RefThing { references: vec![dangling] }).unwrap();
        Ok(())
    }
}

struct AlwaysErrorsImporter;

impl Importer for AlwaysErrorsImporter {
    fn supported_extensions(&self) -> &'static [&'static str] {
        &["fake"]
    }
    fn import(&self, _source_path: &Path, ctx: &mut ImportContext) -> Result<(), ImportError> {
        ctx.emit("thing/0", &RefThing { references: vec![] }).unwrap();
        Ok(())
    }
    fn validate(&self, _sub_assets: &[EmittedSubAsset]) -> Vec<ValidationIssue> {
        vec![ValidationIssue {
            severity: ValidationSeverity::Error,
            message: "always fails for this test".to_string(),
            source_path: std::path::PathBuf::from("thing.fake"),
            sub_asset_name: Some("thing/0".to_string()),
        }]
    }
}

fn write_fixture(name: &str) -> (std::path::PathBuf, std::path::PathBuf, std::path::PathBuf) {
    let temp_dir = std::env::temp_dir().join(format!("asset-cook-validation-{name}-{}", std::process::id()));
    let source_root = temp_dir.join("assets");
    let output_root = temp_dir.join("res");
    std::fs::create_dir_all(&source_root).unwrap();
    std::fs::write(source_root.join("thing.fake"), b"source").unwrap();
    let manifest_path = temp_dir.join("assets.toml");
    std::fs::write(&manifest_path, "[[assets]]\npath = \"thing.fake\"\n").unwrap();
    (manifest_path, source_root, output_root)
}

#[test]
fn dangling_reference_fails_the_cook_run() {
    let (manifest_path, source_root, output_root) = write_fixture("dangling");
    let importers: Vec<Box<dyn Importer>> = vec![Box::new(DanglingRefImporter)];
    let options = CookOptions { manifest_path, source_root: source_root.clone(), output_root };

    let report = run_cook(&importers, &options);
    assert!(!report.errors.is_empty(), "a reference to a sub-asset that was never produced must fail the run");

    std::fs::remove_dir_all(source_root.parent().unwrap()).ok();
}

#[test]
fn validate_error_severity_fails_the_cook_run() {
    let (manifest_path, source_root, output_root) = write_fixture("validate-error");
    let importers: Vec<Box<dyn Importer>> = vec![Box::new(AlwaysErrorsImporter)];
    let options = CookOptions { manifest_path, source_root: source_root.clone(), output_root };

    let report = run_cook(&importers, &options);
    assert!(!report.errors.is_empty(), "an Error-severity ValidationIssue must fail the run");

    std::fs::remove_dir_all(source_root.parent().unwrap()).ok();
}
```

- [ ] **Step 2: Run test to verify it fails**

Run: `cargo test -p asset-cook --test validation`
Expected: FAIL — `run_cook` neither checks references nor calls `validate()` yet.

- [ ] **Step 3: Implement validation passes**

Add `log = "0.4.27"` to `crates/asset-cook/Cargo.toml`.

In `crates/asset-cook/src/run.rs`, change `run_cook` to accumulate every freshly-produced-or-skipped `SourceIndex` into a `Vec<SourceIndex>`, and — for a freshly cooked source only — reconstruct `Vec<EmittedSubAsset>` from the cooked bytes on disk (read each `SubAssetEntry`'s cooked file back via `cooked_file_path_for_id`) to call `importer.validate(&emitted)`. After the manifest loop, cross-check every `SubAssetEntry.references` entry across all accumulated indices against the full set of produced `asset_id`s, pushing `ImportError::MissingRequiredData` for anything unresolved:

```rust
let produced: std::collections::HashSet<essential::assets::AssetId> = all_indices
    .iter()
    .flat_map(|index| index.sub_assets.iter().map(|s| s.asset_id))
    .collect();

for index in &all_indices {
    for sub_asset in &index.sub_assets {
        for reference in &sub_asset.references {
            if !produced.contains(reference) {
                report.errors.push(ImportError::MissingRequiredData {
                    source_path: index.source_path.clone(),
                    message: format!(
                        "'{}' references AssetId {:?}, which was never produced",
                        sub_asset.name, reference
                    ),
                });
            }
        }
    }
}
```

For the `validate()` hook, build each `EmittedSubAsset` from a `SubAssetEntry` plus its cooked bytes:
```rust
let emitted: Vec<crate::EmittedSubAsset> = index
    .sub_assets
    .iter()
    .map(|entry| crate::EmittedSubAsset {
        name: entry.name.clone(),
        asset_id: entry.asset_id,
        type_name: Box::leak(entry.type_name.clone().into_boxed_str()),
        bytes: std::fs::read(cooked_file_path_for_id(&options.output_root, entry.asset_id)).unwrap_or_default(),
        references: entry.references.clone(),
    })
    .collect();

for issue in importer.validate(&emitted) {
    if issue.severity == crate::ValidationSeverity::Error {
        report.errors.push(ImportError::MissingRequiredData {
            source_path: issue.source_path.clone(),
            message: issue.message.clone(),
        });
    } else {
        log::warn!("validation warning for '{}' ({:?}): {}", issue.source_path.display(), issue.sub_asset_name, issue.message);
    }
}
```

Note the same deliberate, process-scoped `Box::leak` as originally noted for `type_name` — flagged again here as a follow-up if `EmittedSubAsset::type_name` ever becomes an owned `String`.

- [ ] **Step 4: Run tests to verify they pass**

Run: `cargo test -p asset-cook --test validation`
Expected: PASS

- [ ] **Step 5: Run the full asset-cook suite**

Run: `cargo test -p asset-cook`
Expected: PASS

- [ ] **Step 6: Commit**

```bash
git add crates/asset-cook
git commit -m "feat(asset-cook): add cross-source reference-integrity validation and Importer::validate hook"
```

---

### Task 7: `cook` CLI binary

**Files:**
- Create: `crates/asset-cook/src/bin/cook.rs`
- Modify: `crates/asset-cook/Cargo.toml`

**Interfaces:**
- `cook <manifest.toml> <source_root> <output_root>`, printing a summary, exiting non-zero on any error, with an empty importer list for now (later phases populate it).

- [ ] **Step 1: Write the binary**

`crates/asset-cook/src/bin/cook.rs`:

```rust
use std::path::PathBuf;

use asset_cook::{run_cook, CookOptions, Importer};

fn registered_importers() -> Vec<Box<dyn Importer>> {
    // Later phases push their Importer impls here as each is migrated.
    Vec::new()
}

fn main() {
    env_logger::init();

    let args: Vec<String> = std::env::args().collect();
    if args.len() != 4 {
        eprintln!("usage: cook <manifest.toml> <source_root> <output_root>");
        std::process::exit(2);
    }

    let options = CookOptions {
        manifest_path: PathBuf::from(&args[1]),
        source_root: PathBuf::from(&args[2]),
        output_root: PathBuf::from(&args[3]),
    };

    let report = run_cook(&registered_importers(), &options);
    println!("cooked: {}, skipped: {}, errors: {}", report.cooked.len(), report.skipped.len(), report.errors.len());
    for error in &report.errors {
        eprintln!("error: {error:?}");
    }

    if !report.errors.is_empty() {
        std::process::exit(1);
    }
}
```

Add `env_logger = "0.11.6"` to `crates/asset-cook/Cargo.toml`.

- [ ] **Step 2: Build and smoke-test**

Run: `cargo build -p asset-cook --bin cook`
Expected: builds clean

Run (adjust temp paths as needed):
```bash
mkdir -p /tmp/cook-smoke/assets
echo "assets = []" > /tmp/cook-smoke/assets.toml
cargo run -p asset-cook --bin cook -- /tmp/cook-smoke/assets.toml /tmp/cook-smoke/assets /tmp/cook-smoke/res
```
Expected: `cooked: 0, skipped: 0, errors: 0`, exit code 0.

- [ ] **Step 3: Commit**

```bash
git add crates/asset-cook
git commit -m "feat(asset-cook): add cook CLI binary"
```

---

## Phase 1: Prove the pipeline end-to-end — image → texture

`Texture` holds `usage_settings: TextureUsageSettings`, containing `wgpu::TextureDescriptor`/`TextureViewDescriptor` — genuinely non-serializable GPU types, unrelated to the `AssetHandle` problem the rest of this plan resolved. So `Texture` is the one asset type that legitimately still needs a small cooked DTO (`CookedTexture`), per the Global Constraints note.

### Task 8: `ImageImporter` — offline image decode into `CookedTexture`

**Files:**
- Create: `crates/render/src/assets/cooked_texture.rs`
- Create: `crates/render/src/importers/image_importer.rs`
- Modify: `crates/render/src/lib.rs`
- Modify: `crates/render/Cargo.toml` (add `serde`, `bincode`, `asset-cook`)
- Test: `crates/render/tests/image_importer.rs`

**Interfaces:**
- `struct CookedTexture { width: u32, height: u32, srgb: bool, pixels: Vec<u8> }` implementing `CookedAsset` with `TYPE_NAME = "Texture"` (no `referenced_sub_assets` override needed — textures reference nothing); `struct ImageImporter` implementing `Importer` for `["png", "jpg", "jpeg"]`, emitting one `"main"` sub-asset per source image.

- [ ] **Step 1: Write the failing test**

Create `crates/render/tests/image_importer.rs`:

```rust
//! Covers ImageImporter producing a CookedTexture from a raw image file.
use std::path::Path;

use asset_cook::{CookedAsset, ImportContext, Importer};
use render::importers::image_importer::ImageImporter;
use render::assets::cooked_texture::CookedTexture;

fn write_test_png(path: &Path) {
    let img = image::RgbaImage::from_pixel(2, 2, image::Rgba([255, 0, 0, 255]));
    img.save(path).expect("failed to write test fixture PNG");
}

#[test]
fn import_produces_one_main_sub_asset_with_correct_pixels() {
    let temp_dir = std::env::temp_dir().join(format!("image-importer-test-{}", std::process::id()));
    std::fs::create_dir_all(&temp_dir).unwrap();
    let source_path = temp_dir.join("swatch.png");
    write_test_png(&source_path);

    let mut ctx = ImportContext::new(std::path::PathBuf::from("swatch.png"));
    ImageImporter.import(&source_path, &mut ctx).expect("importing a valid PNG should succeed");
    let (sub_assets, _dependencies) = ctx.into_parts();

    assert_eq!(sub_assets.len(), 1);
    assert_eq!(sub_assets[0].name, "main");
    assert_eq!(sub_assets[0].type_name, CookedTexture::TYPE_NAME);

    let cooked: CookedTexture = bincode::deserialize(&sub_assets[0].bytes).unwrap();
    assert_eq!(cooked.width, 2);
    assert_eq!(cooked.height, 2);
    assert_eq!(cooked.pixels.len(), 2 * 2 * 4);
    assert_eq!(&cooked.pixels[0..4], &[255, 0, 0, 255]);

    std::fs::remove_dir_all(&temp_dir).ok();
}
```

- [ ] **Step 2: Run test to verify it fails**

Run: `cargo test -p render --test image_importer`
Expected: FAIL to compile.

- [ ] **Step 3: Implement `CookedTexture` and `ImageImporter`**

Add to `crates/render/Cargo.toml`:
```toml
asset-cook = { path = "../asset-cook" }
serde = { version = "1", features = ["derive"] }
bincode = "1.3"
```

`crates/render/src/assets/cooked_texture.rs`:

```rust
use asset_cook::CookedAsset;
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize)]
pub struct CookedTexture {
    pub width: u32,
    pub height: u32,
    pub srgb: bool,
    pub pixels: Vec<u8>,
}

impl CookedAsset for CookedTexture {
    const TYPE_NAME: &'static str = "Texture";
}
```

`crates/render/src/importers/image_importer.rs` (new `importers` module — check `crates/render/src/lib.rs` for how `loaders`/`assets` are declared and match that style):

```rust
use std::path::Path;

use asset_cook::{ImportContext, ImportError, Importer};
use image::GenericImageView;

use crate::assets::cooked_texture::CookedTexture;

pub struct ImageImporter;

impl Importer for ImageImporter {
    fn supported_extensions(&self) -> &'static [&'static str] {
        &["png", "jpg", "jpeg"]
    }

    fn import(&self, source_path: &Path, ctx: &mut ImportContext) -> Result<(), ImportError> {
        let img = image::open(source_path).map_err(|err| ImportError::MalformedSource {
            source_path: source_path.to_path_buf(),
            message: err.to_string(),
        })?;

        let (width, height) = img.dimensions();
        let cooked = CookedTexture { width, height, srgb: true, pixels: img.to_rgba8().into_raw() };

        ctx.emit("main", &cooked).map_err(|err| ImportError::MalformedSource {
            source_path: source_path.to_path_buf(),
            message: format!("{err:?}"),
        })?;

        Ok(())
    }
}
```

- [ ] **Step 4: Run test to verify it passes**

Run: `cargo test -p render --test image_importer`
Expected: PASS

- [ ] **Step 5: Commit**

```bash
git add crates/render/src/assets/cooked_texture.rs crates/render/src/importers crates/render/Cargo.toml crates/render/tests/image_importer.rs crates/render/src/lib.rs
git commit -m "feat(render): add ImageImporter producing CookedTexture"
```

---

### Task 9: Rewrite `TextureLoader` to read cooked bytes via `AssetLoadContext::asset_id()`

**Files:**
- Modify: `crates/render/src/loaders/texture_loader.rs`
- Modify: `crates/render/src/assets/texture.rs`
- Modify: `crates/asset-cook/src/bin/cook.rs` (register `ImageImporter`)
- Modify: `crates/asset-cook/Cargo.toml` (add `render`)
- Test: extend `crates/render/tests/image_importer.rs`

**Interfaces:**
- `Texture::from_cooked(cooked: CookedTexture) -> Texture`; `TextureLoader::load` ignores its `path` parameter and instead reads `load_context.asset_id()`, computes the cooked file location via `asset_cook::cooked_file_path_for_id`, and deserializes.

Note: `cook` depending on `render` pulls in `wgpu`/`winit` at build time (unused by the importer path, since it never creates a device/surface) — accepted for now per the same reasoning as the original plan; a future split is a listed follow-up, not required here.

- [ ] **Step 1: Write the failing test**

Add to `crates/render/tests/image_importer.rs`:

```rust
#[test]
fn texture_from_cooked_preserves_dimensions_and_pixels() {
    let cooked = CookedTexture { width: 2, height: 1, srgb: true, pixels: vec![10, 20, 30, 255, 40, 50, 60, 255] };
    let texture = render::assets::texture::Texture::from_cooked(cooked);
    assert_eq!(texture.size().width, 2);
    assert_eq!(texture.size().height, 1);
    assert_eq!(texture.data(), &[10, 20, 30, 255, 40, 50, 60, 255]);
}
```

- [ ] **Step 2: Run test to verify it fails**

Run: `cargo test -p render --test image_importer`
Expected: FAIL to compile — `Texture::from_cooked` doesn't exist.

- [ ] **Step 3: Implement `Texture::from_cooked` and rewrite `TextureLoader`**

Add to `crates/render/src/assets/texture.rs`:

```rust
use crate::assets::cooked_texture::CookedTexture;

impl Texture {
    pub fn from_cooked(cooked: CookedTexture) -> Self {
        let mut usage_settings = TextureUsageSettings::default();
        usage_settings.texture_descriptor.size = Extent3d {
            width: cooked.width,
            height: cooked.height,
            depth_or_array_layers: 1,
        };
        usage_settings.texture_descriptor.format = if cooked.srgb {
            TextureFormat::Rgba8UnormSrgb
        } else {
            TextureFormat::Rgba8Unorm
        };
        Texture { data: cooked.pixels, usage_settings }
    }
}
```

Rewrite `crates/render/src/loaders/texture_loader.rs`:

```rust
use anyhow::Context;
use asset_cook::cooked_file_path_for_id;
use essential::assets::{asset_loader::AssetLoader, asset_server::AssetLoadContext, AssetPath, LoadableAsset};
use async_trait::async_trait;
use crate::assets::{cooked_texture::CookedTexture, texture::Texture};

pub struct TextureLoader;

#[cfg_attr(target_arch = "wasm32", async_trait(?Send))]
#[cfg_attr(not(target_arch = "wasm32"), async_trait)]
impl AssetLoader for TextureLoader {
    type Asset = Texture;

    async fn load(
        &self,
        _path: AssetPath<'static>,
        load_context: &mut AssetLoadContext,
        _usage_settings: <Self::Asset as LoadableAsset>::UsageSettings,
    ) -> anyhow::Result<Self::Asset> {
        // TODO(follow-up): output_root is hard-coded here as "res" (matching
        // AssetPath's existing "res/" rooting convention) rather than being
        // threaded through AssetLoadContext; revisit if cooked output ever
        // needs to live somewhere else at runtime.
        let cooked_path = cooked_file_path_for_id(std::path::Path::new("res"), load_context.asset_id());
        let bytes = std::fs::read(&cooked_path).with_context(|| {
            format!("failed to read cooked texture at '{}'", cooked_path.display())
        })?;
        let cooked: CookedTexture = bincode::deserialize(&bytes)?;
        Ok(Texture::from_cooked(cooked))
    }
}
```

Note this reads via `std::fs::read` directly rather than `essential::assets::utils::load_binary`, since `load_binary` resolves relative to the human `AssetPath` it's given — but this loader intentionally ignores its `path` parameter in favor of `load_context.asset_id()`. **Verify during implementation:** whether `load_binary`'s exe-relative resolution and wasm-fetch branching should be factored out into a small shared helper both path-based and ID-based loading can call, rather than duplicating the exe-dir-join logic here — check `crates/essential/src/assets/utils.rs`'s current implementation before deciding; for a first pass, duplicating the few lines is acceptable and lower-risk than refactoring shared infrastructure mid-task.

Register `ImageImporter` in `crates/asset-cook/src/bin/cook.rs`:
```rust
use render::importers::image_importer::ImageImporter;

fn registered_importers() -> Vec<Box<dyn Importer>> {
    vec![Box::new(ImageImporter)]
}
```
Add `render = { path = "../render" }` to `crates/asset-cook/Cargo.toml`.

- [ ] **Step 4: Run tests, build the cook binary**

Run: `cargo test -p render --test image_importer && cargo build -p asset-cook --bin cook`
Expected: PASS / builds clean

- [ ] **Step 5: Commit**

```bash
git add crates/render/src/assets/texture.rs crates/render/src/loaders/texture_loader.rs crates/asset-cook/src/bin/cook.rs crates/asset-cook/Cargo.toml crates/render/tests/image_importer.rs
git commit -m "feat(render): read Texture from cooked bytes via AssetId instead of decoding source images at runtime"
```

---

### Task 10: End-to-end regression test — cook then load, touching only the requested file

**Files:**
- Test: `crates/render/tests/texture_pipeline_e2e.rs`

- [ ] **Step 1: Write the test**

Create `crates/render/tests/texture_pipeline_e2e.rs`:

```rust
//! End-to-end proof that a texture can be cooked and then read at its
//! deterministic ID-keyed location — no source image decode at load time.
use asset_cook::{cooked_file_path_for_id, run_cook, CookOptions, Importer};
use essential::assets::AssetId;
use render::importers::image_importer::ImageImporter;

#[test]
fn cooked_texture_is_reachable_by_its_deterministic_id() {
    let temp_dir = std::env::temp_dir().join(format!("texture-e2e-{}", std::process::id()));
    let source_root = temp_dir.join("assets");
    let output_root = temp_dir.join("res");
    std::fs::create_dir_all(source_root.join("textures")).unwrap();

    let img = image::RgbaImage::from_pixel(1, 1, image::Rgba([9, 9, 9, 255]));
    img.save(source_root.join("textures/swatch.png")).unwrap();

    let manifest_path = temp_dir.join("assets.toml");
    std::fs::write(&manifest_path, "[[assets]]\npath = \"textures/swatch.png\"\n").unwrap();

    let importers: Vec<Box<dyn Importer>> = vec![Box::new(ImageImporter)];
    let options = CookOptions { manifest_path, source_root, output_root: output_root.clone() };
    let report = run_cook(&importers, &options);
    assert!(report.errors.is_empty(), "cooking the fixture texture must succeed: {:?}", report.errors);

    let expected_id = AssetId::from_path("textures/swatch.png#main");
    let cooked_path = cooked_file_path_for_id(&output_root, expected_id);
    assert!(cooked_path.exists(), "the cooked texture must be reachable purely from its AssetId");

    let cooked_bytes = std::fs::read(&cooked_path).unwrap();
    let cooked: render::assets::cooked_texture::CookedTexture = bincode::deserialize(&cooked_bytes).unwrap();
    assert_eq!(cooked.pixels, vec![9, 9, 9, 255]);

    std::fs::remove_dir_all(&temp_dir).ok();
}
```

- [ ] **Step 2: Run test**

Run: `cargo test -p render --test texture_pipeline_e2e`
Expected: PASS

- [ ] **Step 3: Run the full render and asset-cook suites**

Run: `cargo test -p render -p asset-cook`
Expected: PASS

- [ ] **Step 4: Commit**

```bash
git add crates/render/tests/texture_pipeline_e2e.rs
git commit -m "test(render): add end-to-end regression test for the cook->load texture pipeline"
```

---

## Phase 2: `Mesh` becomes directly serializable

Unlike `Texture`, `Mesh`/`Vertex` hold no non-serializable data — `Vertex` is `#[repr(C)]`/`Pod`/`Zeroable`, which coexists fine with `Serialize`/`Deserialize` derives (unrelated derive macros, no conflict). So per the Global Constraints, no DTO here: derive directly.

### Task 11: `Mesh`/`Vertex` derive `Serialize`/`Deserialize`; `Mesh` becomes a `LoadableAsset`

**Files:**
- Modify: `crates/mesh/src/mesh.rs`
- Modify: `crates/mesh/src/vertex.rs`
- Modify: `crates/mesh/Cargo.toml` (add `serde`, `bincode`, `asset-cook`, `async-trait` if not already present)
- Test: `crates/mesh/tests/mesh_serialization.rs`

**Interfaces:**
- `Vertex` and `Mesh` gain `#[derive(Serialize, Deserialize)]`; `impl CookedAsset for Mesh { const TYPE_NAME = "Mesh"; }` (no references — meshes point at nothing); `impl LoadableAsset for Mesh` with `MeshLoader: AssetLoader<Asset = Mesh>` reading cooked bytes via `load_context.asset_id()`.

- [ ] **Step 1: Write the failing test**

Create `crates/mesh/tests/mesh_serialization.rs`:

```rust
//! Covers Mesh/Vertex round-tripping through bincode directly (no DTO).
use mesh::mesh::Mesh;
use mesh::vertex::Vertex;

fn sample_vertex(x: f32) -> Vertex {
    Vertex {
        pos_coords: [x, 0.0, 0.0],
        uv_coords: [0.5, 0.5],
        normal: [0.0, 1.0, 0.0],
        tangent: [1.0, 0.0, 0.0],
        bitangent: [0.0, 0.0, 1.0],
        bone_indices: [0, 0, 0, 0],
        bone_weights: [1.0, 0.0, 0.0, 0.0],
    }
}

#[test]
fn mesh_round_trips_through_bincode_directly() {
    let mesh = Mesh { vertices: vec![sample_vertex(0.0), sample_vertex(1.0)], indices: vec![0, 1, 0] };
    let bytes = bincode::serialize(&mesh).unwrap();
    let decoded: Mesh = bincode::deserialize(&bytes).unwrap();
    assert_eq!(decoded.indices, mesh.indices);
    assert_eq!(decoded.vertices[1].pos_coords, [1.0, 0.0, 0.0]);
}
```

- [ ] **Step 2: Run test to verify it fails**

Run: `cargo test -p mesh --test mesh_serialization`
Expected: FAIL to compile — `Mesh`/`Vertex` aren't `Serialize`/`Deserialize` yet.

- [ ] **Step 3: Add derives, `CookedAsset`, `LoadableAsset`, `MeshLoader`**

Add to `crates/mesh/Cargo.toml`:
```toml
asset-cook = { path = "../asset-cook" }
serde = { version = "1", features = ["derive"] }
bincode = "1.3"
```
(check whether `async-trait` is already a direct dependency; add `async-trait = "0.1.50"` if not.)

In `crates/mesh/src/vertex.rs`, add `serde::Serialize, serde::Deserialize` to `Vertex`'s existing derive list (alongside `Copy, Clone, Debug, Default`).

In `crates/mesh/src/mesh.rs`, add `#[derive(serde::Serialize, serde::Deserialize)]` to `Mesh` (alongside its existing `#[derive(Asset)]`), and add:

```rust
use asset_cook::CookedAsset;
use essential::assets::{asset_loader::AssetLoader, asset_server::AssetLoadContext, AssetPath, LoadableAsset};
use async_trait::async_trait;

impl CookedAsset for Mesh {
    const TYPE_NAME: &'static str = "Mesh";
}

impl LoadableAsset for Mesh {
    type UsageSettings = ();
    fn loader() -> Box<dyn AssetLoader<Asset = Self>> {
        Box::new(MeshLoader)
    }
    fn default_usage_settings() -> Self::UsageSettings {}
}

pub struct MeshLoader;

#[cfg_attr(target_arch = "wasm32", async_trait(?Send))]
#[cfg_attr(not(target_arch = "wasm32"), async_trait)]
impl AssetLoader for MeshLoader {
    type Asset = Mesh;

    async fn load(
        &self,
        _path: AssetPath<'static>,
        load_context: &mut AssetLoadContext,
        _usage_settings: (),
    ) -> anyhow::Result<Self::Asset> {
        let cooked_path = asset_cook::cooked_file_path_for_id(std::path::Path::new("res"), load_context.asset_id());
        let bytes = std::fs::read(&cooked_path)?;
        Ok(bincode::deserialize(&bytes)?)
    }
}
```

- [ ] **Step 4: Run test to verify it passes**

Run: `cargo test -p mesh --test mesh_serialization`
Expected: PASS

- [ ] **Step 5: Run the full mesh suite**

Run: `cargo test -p mesh`
Expected: PASS

- [ ] **Step 6: Commit**

```bash
git add crates/mesh
git commit -m "feat(mesh): derive Serialize/Deserialize directly on Mesh/Vertex; make Mesh a LoadableAsset"
```

---

## Phase 3: `StandardMaterial` becomes directly serializable

### Task 12: `StandardMaterial` derives `Serialize`/`Deserialize` directly; `resolve_asset_handles`; `LoadableAsset`

**Files:**
- Modify: `crates/render/src/assets/material.rs`
- Modify: `crates/render/Cargo.toml` (confirm `serde`/`bincode`/`asset-cook` present from Task 8)
- Test: `crates/render/tests/material_serialization.rs`

**Interfaces:**
- `StandardMaterial` gains `#[derive(Serialize, Deserialize)]` directly (its texture fields are `Option<AssetHandle<Texture>>`, now serializable per Task 2) and `impl CookedAsset for StandardMaterial { const TYPE_NAME = "StandardMaterial"; fn referenced_sub_assets(&self) -> Vec<AssetId> { ... } }`, plus `pub(crate) fn resolve_asset_handles(&mut self, asset_server: &AssetServer)` upgrading every `Weak` texture handle to `Strong`; `impl LoadableAsset for StandardMaterial` with `StandardMaterialLoader`.

- [ ] **Step 1: Write the failing test**

Create `crates/render/tests/material_serialization.rs`:

```rust
//! Covers StandardMaterial round-tripping directly through bincode (no DTO)
//! and reporting its texture references for cook-time validation.
use essential::assets::{handle::AssetHandle, AssetId};
use render::assets::material::StandardMaterial;
use render::assets::texture::Texture;

#[test]
fn round_trips_through_bincode_with_weak_texture_handles() {
    let albedo_id = AssetId::from_path("models/character.gltf#texture/albedo");
    let material = StandardMaterial::new(Some(AssetHandle::weak(albedo_id)), None);

    let bytes = bincode::serialize(&material).unwrap();
    let decoded: StandardMaterial = bincode::deserialize(&bytes).unwrap();

    assert_eq!(
        decoded.base_color_texture().map(|h| h.id()),
        Some(albedo_id),
        "the deserialized material's texture field must carry the same AssetId, as a Weak handle"
    );
}

#[test]
fn referenced_sub_assets_lists_present_textures_only() {
    let albedo_id = AssetId::from_path("models/character.gltf#texture/albedo");
    let material = StandardMaterial::new(Some(AssetHandle::<Texture>::weak(albedo_id)), None);

    let refs = asset_cook::CookedAsset::referenced_sub_assets(&material);
    assert_eq!(refs, vec![albedo_id], "only Some(..) texture fields should be reported as references");
}
```

- [ ] **Step 2: Run test to verify it fails**

Run: `cargo test -p render --test material_serialization`
Expected: FAIL to compile — `StandardMaterial` isn't `Serialize`/`Deserialize`, no `CookedAsset` impl.

- [ ] **Step 3: Implement**

Add to `crates/render/src/assets/material.rs`, on the `StandardMaterial` struct definition, add `serde::Serialize, serde::Deserialize` to its existing derive list (alongside `Asset, AsBindGroup, Default`) — this is safe because the derive expands within this module, where the fields (private) are visible, and because `AsBindGroup`'s macro (verified during the design discussion) never inspects or depends on any `Serialize`/`Deserialize` impl, so adding these derives cannot affect its generated code.

Then add:

```rust
use asset_cook::CookedAsset;
use essential::assets::{
    asset_loader::AssetLoader, asset_server::{AssetLoadContext, AssetServer}, AssetId, AssetPath,
    LoadableAsset,
};

impl CookedAsset for StandardMaterial {
    const TYPE_NAME: &'static str = "StandardMaterial";

    fn referenced_sub_assets(&self) -> Vec<AssetId> {
        [
            &self.base_color_texture,
            &self.normal_texture,
            &self.metallic_roughness_texture,
            &self.emissive_texture,
            &self.occlusion_texture,
        ]
        .into_iter()
        .filter_map(|field| field.as_ref().map(|handle| handle.id()))
        .collect()
    }
}

impl StandardMaterial {
    /// Upgrades every Weak texture handle to a live Strong one via the given
    /// AssetServer. Defined in this module so it can reach the private
    /// texture fields directly.
    pub(crate) fn resolve_asset_handles(&mut self, asset_server: &AssetServer) {
        for handle in [
            &mut self.base_color_texture,
            &mut self.normal_texture,
            &mut self.metallic_roughness_texture,
            &mut self.emissive_texture,
            &mut self.occlusion_texture,
        ] {
            if let Some(h) = handle {
                *h = asset_server.load_by_id(h.id());
            }
        }
    }
}

impl LoadableAsset for StandardMaterial {
    type UsageSettings = ();
    fn loader() -> Box<dyn AssetLoader<Asset = Self>> {
        Box::new(StandardMaterialLoader)
    }
    fn default_usage_settings() -> Self::UsageSettings {}
}

pub struct StandardMaterialLoader;

#[cfg_attr(target_arch = "wasm32", async_trait::async_trait(?Send))]
#[cfg_attr(not(target_arch = "wasm32"), async_trait::async_trait)]
impl AssetLoader for StandardMaterialLoader {
    type Asset = StandardMaterial;

    async fn load(
        &self,
        _path: AssetPath<'static>,
        load_context: &mut AssetLoadContext,
        _usage_settings: (),
    ) -> anyhow::Result<Self::Asset> {
        let cooked_path = asset_cook::cooked_file_path_for_id(std::path::Path::new("res"), load_context.asset_id());
        let bytes = std::fs::read(&cooked_path)?;
        let mut material: StandardMaterial = bincode::deserialize(&bytes)?;
        material.resolve_asset_handles(load_context.asset_server());
        Ok(material)
    }
}
```

**Verify during implementation:** `resolve_asset_handles` calling `asset_server.load_by_id(h.id())` unconditionally, even when `h` is already `Strong` — this is harmless (re-requesting an already-loaded/loading ID just returns a fresh handle to the same underlying asset via the existing dedup path) but slightly wasteful; if profiling later shows this matters, match on the variant first and skip already-`Strong` handles. Not worth the complexity in this first pass — every handle here is freshly deserialized and therefore always `Weak` in practice, so the "already Strong" branch never actually triggers yet, only becoming relevant if this method is ever reused on a live, already-resolved material.

- [ ] **Step 4: Run tests to verify they pass**

Run: `cargo test -p render --test material_serialization`
Expected: PASS

- [ ] **Step 5: Run the full render suite**

Run: `cargo test -p render`
Expected: PASS

- [ ] **Step 6: Commit**

```bash
git add crates/render/src/assets/material.rs crates/render/tests/material_serialization.rs
git commit -m "feat(render): derive Serialize/Deserialize directly on StandardMaterial, add resolve_asset_handles"
```

---

## Phase 4: Shared `Scene` type + glTF importer migration

### Task 13: New `crates/scene` — `Scene`/`SceneNode`, directly serializable, `spawn_scene_components`

**Files:**
- Create: `crates/scene/Cargo.toml`
- Create: `crates/scene/src/lib.rs`
- Create: `crates/scene/src/scene.rs`
- Create: `crates/scene/src/spawner.rs`
- Modify: `Cargo.toml` (workspace root — add `scene` to `[dependencies]` for consistency with other crates listed there)
- Test: `crates/scene/tests/scene_serialization.rs`

**Interfaces:**
- `struct SceneNode { name: String, transform: Transform, children: Vec<usize>, mesh: Option<AssetHandle<Mesh>>, material: Option<AssetHandle<StandardMaterial>> }` (skeleton/camera/light/extras fields are an explicit follow-up, per Task 14 Step 3.6 below — this task establishes the mesh/material/hierarchy core, which is also all OBJ needs); `struct Scene { nodes: Vec<SceneNode> }`, both deriving `Serialize`/`Deserialize` directly; `impl CookedAsset for Scene`; `pub(crate) fn resolve_asset_handles(&mut self, asset_server: &AssetServer)`; `impl LoadableAsset for Scene` with `SceneLoader`; `struct SceneSpawnerComponent(pub AssetHandle<Scene>)`; `fn spawn_scene_components(cmd: CommandQueue, spawners: Query<(Entity, &SceneSpawnerComponent)>, scenes: Res<AssetStore<Scene>>)`.

- [ ] **Step 1: Write the failing test**

Create `crates/scene/tests/scene_serialization.rs`:

```rust
//! Covers Scene/SceneNode round-tripping directly through bincode and
//! reporting mesh/material references for cook-time validation.
use asset_cook::CookedAsset;
use essential::assets::{handle::AssetHandle, AssetId};
use essential::transform::Transform;
use scene::scene::{Scene, SceneNode};

#[test]
fn round_trips_and_reports_references() {
    let mesh_id = AssetId::from_path("models/character.gltf#mesh/0");
    let material_id = AssetId::from_path("models/character.gltf#material/0");

    let sc = Scene {
        nodes: vec![
            SceneNode { name: "root".to_string(), transform: Transform::default(), children: vec![1], mesh: None, material: None },
            SceneNode {
                name: "child".to_string(),
                transform: Transform::default(),
                children: vec![],
                mesh: Some(AssetHandle::weak(mesh_id)),
                material: Some(AssetHandle::weak(material_id)),
            },
        ],
    };

    let bytes = bincode::serialize(&sc).unwrap();
    let decoded: Scene = bincode::deserialize(&bytes).unwrap();
    assert_eq!(decoded.nodes.len(), 2);
    assert_eq!(decoded.nodes[0].children, vec![1]);
    assert_eq!(decoded.nodes[1].mesh.as_ref().unwrap().id(), mesh_id);

    assert_eq!(
        sc.referenced_sub_assets(),
        vec![mesh_id, material_id],
        "every node's mesh/material reference must be collected across the whole scene"
    );
}
```

- [ ] **Step 2: Run test to verify it fails**

Run: `cargo test -p scene --test scene_serialization`
Expected: FAIL — crate doesn't exist yet.

- [ ] **Step 3: Implement the `scene` crate**

`crates/scene/Cargo.toml`:

```toml
[package]
name = "scene"
version = "0.1.0"
edition = "2021"

[dependencies]
essential = { path = "../essential" }
ecs = { path = "../ecs" }
mesh = { path = "../mesh" }
render = { path = "../render" }
asset-cook = { path = "../asset-cook" }
async-trait = "0.1.50"
anyhow = "1.0.97"
serde = { version = "1", features = ["derive"] }
bincode = "1.3"
```

Check whether `essential::transform::Transform` already derives `Serialize`/`Deserialize`; if not, add the derive there (in `crates/essential`, wherever `Transform` is actually defined — `grep -rn "struct Transform" crates/essential/src` to find the exact file/path before editing) and add `serde`'s `derive` feature to `crates/essential/Cargo.toml` if it isn't already present from Task 1 (it should be, since Task 1 already added `serde`).

`crates/scene/src/scene.rs`:

```rust
use asset_cook::CookedAsset;
use essential::assets::{
    asset_loader::AssetLoader, asset_server::{AssetLoadContext, AssetServer}, handle::AssetHandle,
    Asset, AssetId, AssetPath, LoadableAsset,
};
use essential::transform::Transform;
use mesh::mesh::Mesh;
use render::assets::material::StandardMaterial;
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize)]
pub struct SceneNode {
    pub name: String,
    pub transform: Transform,
    pub children: Vec<usize>,
    pub mesh: Option<AssetHandle<Mesh>>,
    pub material: Option<AssetHandle<StandardMaterial>>,
}

#[derive(Asset, Serialize, Deserialize)]
pub struct Scene {
    pub nodes: Vec<SceneNode>,
}

impl CookedAsset for Scene {
    const TYPE_NAME: &'static str = "Scene";

    fn referenced_sub_assets(&self) -> Vec<AssetId> {
        self.nodes
            .iter()
            .flat_map(|node| [node.mesh.as_ref().map(|h| h.id()), node.material.as_ref().map(|h| h.id())])
            .flatten()
            .collect()
    }
}

impl Scene {
    pub(crate) fn resolve_asset_handles(&mut self, asset_server: &AssetServer) {
        for node in &mut self.nodes {
            if let Some(h) = &mut node.mesh {
                *h = asset_server.load_by_id(h.id());
            }
            if let Some(h) = &mut node.material {
                *h = asset_server.load_by_id(h.id());
            }
        }
    }
}

impl LoadableAsset for Scene {
    type UsageSettings = ();
    fn loader() -> Box<dyn AssetLoader<Asset = Self>> {
        Box::new(SceneLoader)
    }
    fn default_usage_settings() -> Self::UsageSettings {}
}

pub struct SceneLoader;

#[cfg_attr(target_arch = "wasm32", async_trait::async_trait(?Send))]
#[cfg_attr(not(target_arch = "wasm32"), async_trait::async_trait)]
impl AssetLoader for SceneLoader {
    type Asset = Scene;

    async fn load(
        &self,
        _path: AssetPath<'static>,
        load_context: &mut AssetLoadContext,
        _usage_settings: (),
    ) -> anyhow::Result<Self::Asset> {
        let cooked_path = asset_cook::cooked_file_path_for_id(std::path::Path::new("res"), load_context.asset_id());
        let bytes = std::fs::read(&cooked_path)?;
        let mut sc: Scene = bincode::deserialize(&bytes)?;
        sc.resolve_asset_handles(load_context.asset_server());
        Ok(sc)
    }
}
```

`crates/scene/src/spawner.rs` — a direct structural port of `spawn_gltf_components`'s node-walking loop (`crates/gltf-loader/src/loader.rs:717`), trimmed to the mesh/material/hierarchy fields defined above:

```rust
use ecs::{command::CommandQueue, component::Component, entity::Entity, query::Query, system::resource::Res};
use essential::assets::{asset_store::AssetStore, handle::AssetHandle};
use mesh::mesh::MeshComponent;
use render::components::material::MaterialComponent;

use crate::scene::Scene;

#[derive(Component)]
pub struct SceneSpawnerComponent(pub AssetHandle<Scene>);

pub fn spawn_scene_components(
    mut cmd: CommandQueue,
    spawners: Query<(Entity, &SceneSpawnerComponent)>,
    scenes: Res<AssetStore<Scene>>,
) {
    for (_entity, spawner) in spawners.iter() {
        let Some(scene) = scenes.get(&spawner.0) else { continue };

        let mut entities = Vec::with_capacity(scene.nodes.len());
        for node in &scene.nodes {
            let child = cmd.spawn();
            entities.push(child);

            if let Some(mesh) = &node.mesh {
                cmd.insert(MeshComponent { handle: mesh.clone() }, child);
            }
            if let Some(material) = &node.material {
                cmd.insert(MaterialComponent(material.clone()), child);
            }
        }

        for (index, node) in scene.nodes.iter().enumerate() {
            for &child_index in &node.children {
                cmd.set_parent(entities[child_index], entities[index]);
            }
        }
    }
}
```

**Verify during implementation:** the exact `CommandQueue` API (`cmd.spawn()`, `cmd.insert(component, entity)`, `cmd.set_parent(child, parent)`) and `MaterialComponent`'s real shape — check `crates/gltf-loader/src/loader.rs` around line 717-900 and `crates/ecs/src/command.rs` for the real call patterns before finalizing this file, mirroring them exactly rather than the sketch above.

Update `crates/scene/src/lib.rs`:
```rust
pub mod scene;
pub mod spawner;
```

Add `scene = { path = "crates/scene" }` to the workspace root `Cargo.toml`'s `[dependencies]` table.

- [ ] **Step 4: Run tests, build the workspace**

Run: `cargo test -p scene --test scene_serialization && cargo build --workspace`
Expected: PASS / builds clean

- [ ] **Step 5: Commit**

```bash
git add crates/scene Cargo.toml
git commit -m "feat(scene): add format-agnostic Scene/SceneNode (directly serializable) and spawn_scene_components"
```

---

### Task 14: Migrate `GLTFLoader` parsing into `GltfImporter`

**Files:**
- Create: `crates/gltf-loader/src/gltf_importer.rs`
- Modify: `crates/gltf-loader/src/loader.rs` (remove the runtime `AssetLoader`/`GLTFScene` machinery this replaces)
- Modify: `crates/gltf-loader/Cargo.toml` (add `asset-cook`, `serde`, `bincode`, `scene`)
- Modify: `crates/asset-cook/src/bin/cook.rs` (register `GltfImporter`)
- Test: `crates/gltf-loader/tests/gltf_importer.rs`

This is the largest single task in the plan because it relocates ~900 lines of existing, already-correct parsing logic. The instructions identify exactly what changes and what is copied unchanged.

**Interfaces:**
- Consumes: `Scene`/`SceneNode` (Task 13), `Mesh` (Task 11), `StandardMaterial` (Task 12), `CookedTexture` (Task 8), `Importer`/`ImportContext` (Task 3).
- Produces: `struct GltfImporter` implementing `Importer` for `["gltf", "glb"]`.

- [ ] **Step 1: Write the failing test with a minimal fixture glTF**

Create `crates/gltf-loader/tests/fixtures/triangle.gltf` (self-contained, single-triangle, no textures):

```json
{
  "asset": { "version": "2.0" },
  "scene": 0,
  "scenes": [{ "nodes": [0] }],
  "nodes": [{ "name": "Triangle", "mesh": 0 }],
  "meshes": [{
    "primitives": [{ "attributes": { "POSITION": 0 }, "indices": 1, "material": 0 }]
  }],
  "materials": [{
    "name": "Red",
    "pbrMetallicRoughness": { "baseColorFactor": [1.0, 0.0, 0.0, 1.0] }
  }],
  "accessors": [
    { "bufferView": 0, "componentType": 5126, "count": 3, "type": "VEC3", "max": [1.0, 1.0, 0.0], "min": [0.0, 0.0, 0.0] },
    { "bufferView": 1, "componentType": 5123, "count": 3, "type": "SCALAR" }
  ],
  "bufferViews": [
    { "buffer": 0, "byteOffset": 0, "byteLength": 36 },
    { "buffer": 0, "byteOffset": 36, "byteLength": 6 }
  ],
  "buffers": [{
    "byteLength": 42,
    "uri": "data:application/octet-stream;base64,AAAAAAAAAAAAAAAAAACAPwAAAAAAAAAAAAAAAAAAgD8AAIA/AAAAAAAAAAABAAIA"
  }]
}
```

Create `crates/gltf-loader/tests/gltf_importer.rs`:

```rust
//! Covers GltfImporter splitting a single .gltf into independently-cooked
//! mesh, material, and scene sub-assets (this fixture has no textures),
//! with the scene's node referencing the mesh/material by stable AssetId.
use std::path::Path;

use asset_cook::{ImportContext, Importer};
use essential::assets::AssetId;
use gltf_loader::gltf_importer::GltfImporter;
use scene::scene::Scene;

#[test]
fn import_emits_mesh_material_and_scene_sub_assets() {
    let fixture = Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/triangle.gltf");
    let relative_source = Path::new("triangle.gltf");
    let mut ctx = ImportContext::new(relative_source.to_path_buf());

    GltfImporter.import(&fixture, &mut ctx).expect("importing the triangle fixture should succeed");
    let (sub_assets, _dependencies) = ctx.into_parts();

    let names: Vec<&str> = sub_assets.iter().map(|s| s.name.as_str()).collect();
    assert!(names.contains(&"mesh/0"), "expected a mesh/0 sub-asset, got: {names:?}");
    assert!(names.contains(&"material/0"), "expected a material/0 sub-asset, got: {names:?}");
    assert!(names.contains(&"scene"), "expected a scene sub-asset, got: {names:?}");

    let scene_entry = sub_assets.iter().find(|s| s.name == "scene").unwrap();
    let cooked_scene: Scene = bincode::deserialize(&scene_entry.bytes).unwrap();
    assert_eq!(cooked_scene.nodes.len(), 1);
    assert_eq!(cooked_scene.nodes[0].name, "Triangle");
    assert_eq!(
        cooked_scene.nodes[0].mesh.as_ref().unwrap().id(),
        AssetId::from_path("triangle.gltf#mesh/0"),
        "the scene node's mesh handle must carry the exact same AssetId a runtime load of 'triangle.gltf#mesh/0' would compute"
    );
}
```

- [ ] **Step 2: Run test to verify it fails**

Run: `cargo test -p gltf-loader --test gltf_importer`
Expected: FAIL to compile.

- [ ] **Step 3: Implement `GltfImporter`**

Add to `crates/gltf-loader/Cargo.toml`:
```toml
asset-cook = { path = "../asset-cook" }
scene = { path = "../scene" }
serde = { version = "1", features = ["derive"] }
bincode = "1.3"
```

`crates/gltf-loader/src/gltf_importer.rs` — adapt the existing `GLTFLoader::load` body (currently in `crates/gltf-loader/src/loader.rs`) with these substitutions:

1. Replace `gltf::import(path)` (native) / `gltf::import_slice` (wasm) with a synchronous `gltf::import(source_path)` call — the importer runs offline, so the wasm-fetch branch (`loader.rs` lines ~193-207) is dropped entirely.
2. Everywhere the current code does `asset_server.add(primitive)` for a mesh primitive (`load_primitive`, `loader.rs:514`), instead call `ctx.emit(&format!("mesh/{index}"), &primitive)?` directly on the real `Mesh` value (no conversion needed — `Mesh` is `CookedAsset` per Task 11).
3. Everywhere the current code builds a `StandardMaterial` from glTF PBR data (the materials loop), keep building a real `StandardMaterial` exactly as today (same public constructor/setters, same `Color`/factor conversions — nothing about material *construction* changes), but wherever it previously assigned a live `AssetHandle<Texture>` obtained via `asset_server.load`/`texture_cache`, instead assign `AssetHandle::weak(ctx.sub_asset_id(&format!("texture/{image_index}")))`. Then `ctx.emit(&format!("material/{index}"), &material)?`.
4. Everywhere the current code decodes an image via `dynamic_image_from_gltf` (`loader.rs:211-237`), build a `render::assets::cooked_texture::CookedTexture` from the decoded `image::DynamicImage` (mirroring `ImageImporter::import`'s conversion from Task 8) and `ctx.emit(&format!("texture/{image_index}"), &cooked_texture)?`, keyed by image index — this changes the current texture-cache's dedup key from `(image_index, is_srgb)` to plain `image_index`, a deliberate simplification (see the same note as the original plan draft: `CookedTexture` records `srgb` itself now, so this is fine unless an asset relies on both sRGB and linear variants of the same reused image, in which case key as `texture/{image_index}_{srgb|linear}` instead).
5. Node walking: reuse the current node-iteration structure, building `Vec<scene::scene::SceneNode>` with `mesh: Some(AssetHandle::weak(ctx.sub_asset_id(&format!("mesh/{primitive_index}"))))` / `material: Some(AssetHandle::weak(ctx.sub_asset_id(&format!("material/{material_index}"))))` per node, then `ctx.emit("scene", &Scene { nodes })?`.
6. `paths_to_uuid`/`collect_paths` (`loader.rs:919-946`) and skeleton/animation/camera/light handling are **out of scope for this task** (same explicit, tracked gap as originally planned — `Scene`/`SceneNode` from Task 13 doesn't yet have fields for them). Add a doc comment at the top of `gltf_importer.rs`: `// TODO(follow-up): skeleton, animation, camera, light, and Blender-extras component data are not yet ported from the original GLTFLoader — see loader.rs for the reference implementation.`

- [ ] **Step 4: Run test to verify it passes**

Run: `cargo test -p gltf-loader --test gltf_importer`
Expected: PASS

- [ ] **Step 5: Remove the now-dead runtime glTF loading code and update plugin wiring**

Delete `GLTFLoader`, its `AssetLoader` impl, and `GLTFScene`'s `LoadableAsset` impl from `crates/gltf-loader/src/loader.rs`. Keep `paths_to_uuid`/`collect_paths` if still referenced by anything not yet migrated; otherwise delete (retrievable from git history for the skeleton/animation follow-up).

Update the crate's plugin registration (grep `impl Plugin for` in `crates/gltf-loader/src`) to register `scene::scene::Scene`, `mesh::mesh::Mesh`, `render::assets::material::StandardMaterial`, `render::assets::texture::Texture` as loadable assets (if not already registered elsewhere) and add `scene::spawner::spawn_scene_components` as an `Update`-schedule system in place of the old `spawn_gltf_components`.

- [ ] **Step 6: Register `GltfImporter` in the cook binary**

```rust
use gltf_loader::gltf_importer::GltfImporter;

fn registered_importers() -> Vec<Box<dyn Importer>> {
    vec![Box::new(ImageImporter), Box::new(GltfImporter)]
}
```
Add `gltf-loader = { path = "../gltf-loader" }` to `crates/asset-cook/Cargo.toml`.

- [ ] **Step 7: Build and test everything touched so far**

Run: `cargo build --workspace && cargo test -p gltf-loader -p scene -p render -p mesh -p asset-cook`
Expected: builds and all tests pass. Search for and update any remaining external references to `GLTFScene`/`GLTFSpawnerComponent`/`spawn_gltf_components` (`grep -rn "GLTFScene\|GLTFSpawnerComponent\|spawn_gltf_components" --include=*.rs`).

- [ ] **Step 8: Commit**

```bash
git add crates/gltf-loader crates/asset-cook crates/scene
git commit -m "feat(gltf-loader): migrate glTF parsing to an offline GltfImporter emitting split sub-assets"
```

---

## Phase 5: OBJ importer migration

### Task 15: Migrate `obj_loader.rs`/`mtl_loader.rs` into `ObjImporter`

**Files:**
- Create: `crates/obj-loader/src/obj_importer.rs`
- Delete: runtime pieces of `crates/obj-loader/src/obj_loader.rs` (`OBJLoader`, `OBJAsset`, `OBJSpawnerComponent`, `spawn_obj_component`) and `mtl_loader.rs`'s `LoadableAsset` impl
- Modify: `crates/obj-loader/Cargo.toml`
- Modify: `crates/asset-cook/src/bin/cook.rs` (register `ObjImporter`)
- Test: `crates/obj-loader/tests/obj_importer.rs`

**Interfaces:**
- `struct ObjImporter` implementing `Importer` for `["obj"]`, emitting `mesh/{n}` (one per `tobj::Model`), `material/{mtl_stem}` (one per referenced `.mtl` file, preserving today's pre-existing behavior of collapsing every material entry within one `.mtl` file into a single `StandardMaterial` — a known quirk, not fixed here, call it out in the PR description), and a flat `scene` sub-asset with one `SceneNode` per mesh (identity transform, no hierarchy) referencing that mesh's material.

- [ ] **Step 1: Write the failing test with a minimal fixture**

Create `crates/obj-loader/tests/fixtures/square.obj`:
```
mtllib square.mtl
o Square
v 0.0 0.0 0.0
v 1.0 0.0 0.0
v 1.0 1.0 0.0
v 0.0 1.0 0.0
vt 0.0 0.0
vt 1.0 0.0
vt 1.0 1.0
vt 0.0 1.0
usemtl Red
f 1/1 2/2 3/3
f 1/1 3/3 4/4
```

Create `crates/obj-loader/tests/fixtures/square.mtl`:
```
newmtl Red
Kd 1.0 0.0 0.0
```

Create `crates/obj-loader/tests/obj_importer.rs`:

```rust
//! Covers ObjImporter splitting a single .obj/.mtl pair into mesh, material,
//! and scene sub-assets, reusing the same Scene shape as glTF.
use std::path::Path;

use asset_cook::{ImportContext, Importer};
use obj_loader::obj_importer::ObjImporter;
use scene::scene::Scene;

#[test]
fn import_emits_mesh_material_and_flat_scene() {
    let fixture = Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/square.obj");
    let mut ctx = ImportContext::new(std::path::PathBuf::from("square.obj"));

    ObjImporter.import(&fixture, &mut ctx).expect("importing the square fixture should succeed");
    let (sub_assets, dependencies) = ctx.into_parts();

    let names: Vec<&str> = sub_assets.iter().map(|s| s.name.as_str()).collect();
    assert!(names.iter().any(|n| n.starts_with("mesh/")), "expected a mesh sub-asset, got: {names:?}");
    assert!(names.iter().any(|n| n.starts_with("material/")), "expected a material sub-asset, got: {names:?}");
    assert!(names.contains(&"scene"), "expected a scene sub-asset, got: {names:?}");

    assert!(
        dependencies.iter().any(|d| d.path.file_name().unwrap() == "square.mtl"),
        "the referenced .mtl file must be tracked as a dependency for incremental rebuilds"
    );

    let scene_entry = sub_assets.iter().find(|s| s.name == "scene").unwrap();
    let cooked_scene: Scene = bincode::deserialize(&scene_entry.bytes).unwrap();
    assert_eq!(cooked_scene.nodes.len(), 1, "the fixture has one mesh, so one flat scene node");
    assert!(cooked_scene.nodes[0].children.is_empty(), "OBJ has no hierarchy");
}
```

- [ ] **Step 2: Run test to verify it fails**

Run: `cargo test -p obj-loader --test obj_importer`
Expected: FAIL to compile.

- [ ] **Step 3: Implement `ObjImporter`**

Add to `crates/obj-loader/Cargo.toml`:
```toml
asset-cook = { path = "../asset-cook" }
scene = { path = "../scene" }
serde = { version = "1", features = ["derive"] }
bincode = "1.3"
```

`crates/obj-loader/src/obj_importer.rs` — adapt `OBJLoader::load` (`obj_loader.rs`) and `MTLLoader::load` (`mtl_loader.rs`) with these substitutions:

1. Use `tobj::load_obj(source_path, &tobj::LoadOptions { single_index: true, triangulate: true, ..Default::default() })` (synchronous, since importing is offline) instead of the current `_async` call.
2. For the `mtllib` line the current code scans for manually, resolve that `.mtl` path relative to `source_path`'s directory, `ctx.track_dependency(mtl_path.clone(), asset_cook::hash_file_contents(&mtl_path)?)`, then run today's `MTLLoader` conversion logic (preserving the existing collapse-all-materials-into-one behavior verbatim) to build a real `StandardMaterial` directly (no DTO, no live texture handles needed for `.obj` since it has none) and `ctx.emit(&format!("material/{mtl_stem}"), &material)?` where `mtl_stem` is the `.mtl` file's stem.
3. For each `tobj::Model`, build a `Mesh` from its `mesh.positions`/`.texcoords`/`.normals`/`.indices` (following the same per-vertex assembly and `compute_normals`/`compute_tangents` fallback already in `obj_loader.rs`) and `ctx.emit(&format!("mesh/{index}"), &mesh)?`.
4. Build one `SceneNode` per model: `name: model.name.clone()`, `transform: Transform::default()`, `children: vec![]`, `mesh: Some(AssetHandle::weak(ctx.sub_asset_id(&format!("mesh/{index}"))))`, `material: Some(AssetHandle::weak(ctx.sub_asset_id(&format!("material/{mtl_stem}"))))`. Emit `ctx.emit("scene", &Scene { nodes })?`.

- [ ] **Step 4: Run test to verify it passes**

Run: `cargo test -p obj-loader --test obj_importer`
Expected: PASS

- [ ] **Step 5: Remove dead runtime OBJ loading code and update plugin wiring**

Delete `OBJLoader`, `OBJAsset`, `OBJSpawnerComponent`, `spawn_obj_component` from `obj_loader.rs` and `MTLLoader`/`MTLMaterial`'s `LoadableAsset` impl from `mtl_loader.rs`. Update `obj-loader`'s plugin registration the same way as Task 14 Step 5 — register `Scene`/`Mesh`/`StandardMaterial` if not already registered elsewhere, add `spawn_scene_components` only if not already added by the gltf-loader plugin (avoid double-registering the same system).

- [ ] **Step 6: Register `ObjImporter` in the cook binary**

```rust
use obj_loader::obj_importer::ObjImporter;

fn registered_importers() -> Vec<Box<dyn Importer>> {
    vec![Box::new(ImageImporter), Box::new(GltfImporter), Box::new(ObjImporter)]
}
```
Add `obj-loader = { path = "../obj-loader" }` to `crates/asset-cook/Cargo.toml`.

- [ ] **Step 7: Build and test the whole workspace**

Run: `cargo build --workspace && cargo test --workspace`
Expected: builds and all tests pass. Update any remaining call sites referencing `OBJAsset`/`OBJSpawnerComponent` (grep across the repo, same as Task 14 Step 7).

- [ ] **Step 8: Commit**

```bash
git add crates/obj-loader crates/asset-cook
git commit -m "feat(obj-loader): migrate OBJ/MTL parsing to an offline ObjImporter, reusing the shared Scene type"
```

---

## Task 16: Wire an example over to the cook pipeline and manually verify rendering

**Files:**
- Modify: `examples/render-test/` (move DCC source files, add `assets.toml`, add a pre-run cook step)
- Manual verification only — no new automated test, since automated tests can't confirm the renderer still draws the migrated assets correctly.

- [ ] **Step 1: Reorganize the example's asset tree**

Move `examples/render-test/res/Sponza/*` (and any other DCC source files under this example's `res/`) into a new `examples/render-test/assets/Sponza/`. `res/` becomes purely the cook output directory (add `res/.cooked/`, `res/.index/` to `.gitignore` if it isn't already covered).

- [ ] **Step 2: Write `assets.toml` for the example**

Create `examples/render-test/assets.toml` listing every DCC source file the example loads (grep the example's setup code for `asset_server.load(` calls to enumerate them exactly).

- [ ] **Step 3: Run cook against the example**

Run: `cargo run -p asset-cook --bin cook -- examples/render-test/assets.toml examples/render-test/assets examples/render-test/res`
Expected: `errors: 0`.

- [ ] **Step 4: Update the example's load call sites**

Wherever the example currently does `asset_server.load::<GLTFScene>("Sponza/Sponza.gltf")`, update to `asset_server.load::<scene::scene::Scene>("Sponza/Sponza.gltf#scene")`.

- [ ] **Step 5: Run the example and visually confirm the scene renders correctly**

Run: `cargo run -p render-test` (check the example's own README/binary name first if this differs)
Expected: the window opens and displays the same scene geometry/materials as before this migration, allowing for the known sRGB/linear texture-cache simplification and the explicitly out-of-scope skeleton/animation/camera/light/extras gap (Task 14 Step 3.6) — if the example's current scene relies on any of those, note the visual regression explicitly rather than silently accepting it.

- [ ] **Step 6: Commit**

```bash
git add examples/render-test
git commit -m "chore(render-test): wire the example over to the cook pipeline"
```

---

## Post-Plan Follow-Ups (explicitly out of scope for this plan)

- Skeleton, animation, camera, light, and Blender-extras component data in `Scene`/`SceneNode` (flagged in Task 14 Step 3.6) — needed before this pipeline fully replaces the current glTF-driven level-editing workflow from PR #56.
- The `Box::leak` workaround in `run_cook`'s validation pass (Task 6) — revisit if `EmittedSubAsset::type_name` should become an owned `String`.
- The `cook` binary's dependency on `render` pulling in GPU/windowing crates it never uses at cook time (Task 9) — worth splitting `CookedTexture`/`ImageImporter` into a lighter crate if `cook`'s build time becomes a problem.
- sRGB vs. linear texture-cache-key simplification introduced in Task 14 Step 3.4.
- `resolve_asset_handles`'s unconditional re-`load_by_id` even for already-`Strong` handles (Task 12) — revisit if this method is ever called on a live, already-resolved asset rather than only immediately post-deserialize.
- Auditing whether `AssetHandle<T>` needs `PartialEq`/`Hash` beyond what exists today, now that it's an enum (spec's Open Items).
- The output-root-hard-coded-as-`"res"` in every cooked-format loader (Tasks 9, 11, 12, 13) — fine for a single-executable-relative-`res/`-directory setup matching today's convention, but worth threading through `AssetLoadContext` explicitly if that ever needs to vary.
