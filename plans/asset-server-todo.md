# Asset Server Improvements

## Current State

The asset server (`crates/essential/src/assets/`) provides:

- **Path-based deduplication** via `path_to_id: HashMap<AssetPath, AssetId>` — loading the same path twice reuses the existing `AssetId` and skips re-loading if the asset is already pending or loaded.
- **Async loading** via a dedicated `LoadTaskPool` with channel-based event delivery.
- **Reference-counted handles** — `AssetHandle<A>` holds an `Arc<StrongAssetHandle>`; when the last handle drops, the path mapping is cleaned up.
- **Two-level storage** — CPU `AssetStore<A>` and GPU `RenderAssets<A>` for prepared GPU resources.

---

## Proposed Improvements

### 1. Deduplicate `add()` via Content Hashing

**Problem:** `asset_server.add(asset)` always allocates a new `AssetId`, so inserting the same asset data twice creates duplicate entries. This is common when multiple GLTF loaders generate the same default material or unit mesh.

**Approach:**
- Add an optional `Fingerprint` associated type to `LoadableAsset` (or a separate `Hashable` trait bound).
- Before inserting via `add()`, compute a fast hash (e.g. `xxHash` or `FxHasher` over key fields) and check a `fingerprint_to_id: HashMap<u64, AssetId>` table.
- Return the existing handle if a match is found.
- Fall back to the current path for unstructured or GPU-only assets where hashing is impractical.

**Tradeoffs:**
- Hashing adds a small upfront cost per `add()` call.
- Hash collisions are unlikely but possible — consider storing a type discriminant alongside the hash.
- Assets that embed large GPU buffers should opt out; only hash the descriptor struct.

---

### 2. Strengthen Path Deduplication (Usage Settings Awareness)

**Problem:** `load_with_usage_settings(path, settings)` deduplicates by path only. If the same file is loaded twice with different `UsageSettings` (e.g. a texture requested as both sRGB and linear), the second call silently returns the first asset with the wrong settings.

**Approach:**
- Change the deduplication key from `AssetPath` to `(AssetPath, UsageSettingsHash)`.
- `path_to_id` becomes `HashMap<(AssetPath, u64), AssetId>` where the second field is a hash of the serialised settings.
- Assets loaded with default settings (the common case) hash to a stable zero/unit value with no overhead.

---

### 3. Dependency Tracking

**Problem:** There is currently no record of which assets were loaded as sub-assets of another. This makes targeted hot-reloading, cascade invalidation, and bake freshness checks difficult.

**Approach:**
- Add a `dependencies: HashMap<AssetId, HashSet<AssetId>>` table to `AssetServerData` (child → parents, or parent → children; both directions are useful).
- `AssetLoadContext` already has access to the `AssetServer`; extend it with `fn record_dependency(&mut self, child_id: AssetId)` so loaders can register their sub-asset loads.
- Expose `asset_server.dependencies_of(id) -> &[AssetId]` and `asset_server.dependents_of(id) -> &[AssetId]`.
- When a root asset is freed, its sub-assets that have no other dependents become candidates for eviction.

**Use cases unlocked:**
- Targeted hot-reload: changing a texture triggers re-upload only for materials that reference it.
- Bake invalidation: a baked scene file is stale if any dependency's source has changed.
- Debug tooling: visualise the full asset graph in an editor.

---

### 4. Baked Asset Format

**Problem:** Every startup re-runs the full import pipeline (GLTF parsing, mesh processing, image decoding). For large scenes this is the dominant load time.

**Approach:**

#### 4a. Baked File Format
Define a canonical binary format (e.g. `.baked`) per asset type stored in a `bake-cache/` directory alongside `res/`. A baked file contains:
- A header with source path, source file hash (e.g. SHA-256 of the original file), engine version, and `UsageSettings` hash.
- A compact binary payload — e.g. raw vertex/index buffers for meshes, raw pixel data + GPU format metadata for textures, pre-skinned bind poses for skeletons.
- An embedded dependency manifest listing the `AssetId`s and source hashes of all sub-assets.

#### 4b. Loader Integration
Extend the `AssetLoader` trait with an optional bake/unbake interface:

```rust
pub trait AssetLoader: Send + Sync + 'static {
    type Asset: LoadableAsset;
    type BakedAsset: BakedAsset = Self::Asset; // default = same

    async fn load(...) -> anyhow::Result<Self::Asset>;

    // Optional: override to write a compact baked form
    fn bake(asset: &Self::Asset) -> anyhow::Result<Vec<u8>> { ... }

    // Optional: override to restore from baked bytes (skips full parse)
    async fn load_baked(bytes: &[u8]) -> anyhow::Result<Self::Asset> { ... }
}
```

The asset server's `load_internal` checks for a fresh baked file before spawning the full loader task. If the baked file's source hash matches the current source file, `load_baked` is called instead.

#### 4c. Cache Invalidation
A baked file is considered stale if:
- The source file's modification time or content hash has changed.
- The engine version in the header does not match the running build.
- The `UsageSettings` hash has changed.
- Any dependency listed in the manifest is itself stale.

A CLI tool (or editor command) should be able to pre-bake all assets in `res/` into `bake-cache/` for distribution builds — making runtime load a pure deserialise step.

#### 4d. Memory Layout
Baked meshes should store vertex data in the GPU-ready interleaved layout to skip the CPU-side reformat step. Baked textures should store GPU-compressed formats (BC7/ASTC) where supported by the target platform.

---

### 5. Async Handle Status API

**Problem:** After calling `asset_server.load(path)`, callers poll `asset_store.get(&handle)` returning `Option<&A>`. There is no way to distinguish "not yet loaded" from "failed to load" without reading logs.

**Approach:**
- Add a `AssetStatus` enum: `Loading`, `Loaded`, `Failed(String)`.
- Expose `asset_server.status(id: AssetId) -> AssetStatus` backed by the existing `pending_tasks` / `loaded_assets` sets plus a new `failed_assets: HashSet<AssetId>`.
- Add `asset_server.wait_for(id) -> impl Future<Output = Result<(), AssetError>>` for systems that need to block on an asset before proceeding (e.g. loading screens).

---

### 6. Hot Reloading

**Problem:** Changing a texture or mesh on disk requires restarting the application to see the update.

**Approach:**
- Integrate a file watcher (e.g. `notify` crate) into the `AssetManagerPlugin`. On native targets only; WASM uses a polling fallback.
- When a source file changes, mark the corresponding `AssetId` as dirty.
- Re-run the full loader (or baked loader if the bake cache is also updated) and replace the entry in `AssetStore`.
- Walk the dependency graph (improvement 3) to flag dependent assets for re-prepare in `RenderAssets`.

**Scope:** Development builds only; strip the file watcher from release/distribution builds.

---

### 7. Memory Budget and LRU Eviction

**Problem:** Once loaded, assets stay in memory until all handles are dropped. For open-world or streaming scenarios, assets out of the active region hold memory indefinitely.

**Approach:**
- Add an optional `MemoryBudget` resource (bytes limit per asset type).
- Track last-access time in `AssetStoreEntry`.
- When the budget is exceeded, evict the least-recently-used assets whose handle ref-count is 1 (i.e. only the `AssetStore` itself holds a reference).
- Eviction writes a baked file if one does not exist (ties into improvement 4), so the asset can be cheaply reloaded on next access.

---

### 8. Asset Load Events

**Problem:** The only way to react to an asset finishing loading is to poll `asset_store.get(&handle)` every frame. There is no way to run a system exactly once when a specific asset (or any asset of a given type) becomes available, without writing boilerplate polling logic in every consumer.

**Approach:**
- Define an `AssetLoadedEvent<A>` ECS event carrying the `AssetHandle<A>` and the `AssetId`.
- After `handle_asset_load_events` inserts an asset into `AssetStore<A>`, emit the corresponding `AssetLoadedEvent<A>` into the ECS event queue.
- Systems that care about a specific asset can filter by handle equality:

```rust
fn on_scene_ready(
    mut events: EventReader<AssetLoadedEvent<GLTFScene>>,
    scene_handle: Res<MySceneHandle>,
) {
    for event in events.read() {
        if event.handle == scene_handle.0 {
            // spawn entities, start game, etc.
        }
    }
}
```

- For the common "wait until all assets of type X are loaded" case, provide a helper condition `all_loaded::<A>()` that returns `true` once the pending count for that type reaches zero.
- Pair with improvement 5 (status API): a `Failed` variant of the event covers load errors so error handling does not require a separate polling path.

**Tradeoffs:**
- Events are consumed once per frame; systems added after the frame an asset loads will miss the event unless a persistent "already loaded" set is also maintained (the existing `loaded_assets` HashSet already serves this role for status queries).

---

### 9. Asset Handle and Store Serialization

**Problem:** Saving and loading game state (save files, scene files, editor prefabs) requires serializing references to assets. `AssetHandle<A>` currently holds a runtime-only `Arc`; it cannot be written to disk or sent over a network without losing its identity. Similarly, `AssetStore<A>` has no persistence layer, so asset data computed at runtime (e.g. procedural meshes) cannot survive a restart without being re-generated.

**Approach:**

#### 9a. Serializable Handle Representation
Introduce a `SerializedHandle` type that captures just enough information to reconstruct the handle at load time:

```rust
#[derive(Serialize, Deserialize)]
pub enum SerializedHandle {
    /// Asset loaded from a file — reconstruct via asset_server.load(path)
    Path(AssetPath<'static>),
    /// Procedural asset baked to cache — reconstruct via bake-cache lookup
    Baked(Uuid),
}
```

- `AssetHandle<A>` gets `fn to_serialized(&self) -> SerializedHandle` (reads the path from `StrongAssetHandle` if present, falls back to the `AssetId` UUID for `add()`-inserted assets).
- Reconstruction: `asset_server.handle_from_serialized(s: SerializedHandle) -> AssetHandle<A>` calls `load(path)` or looks up the bake cache.
- For ECS scene serialization, derive `Reflect` on `AssetHandle<A>` using the serialized form so the reflection-based serializer produces stable output.

#### 9b. AssetStore Snapshot / Restore
For procedural assets that have no source file, support exporting a snapshot of `AssetStore<A>` to disk and restoring it:

- Each asset type optionally implements `Serialize + Deserialize` (gated behind a feature flag or a separate `PersistableAsset` trait to avoid forcing serde onto all asset types).
- `AssetStore<A>::snapshot() -> Vec<(Uuid, A)>` serialises all current entries.
- `AssetStore<A>::restore(snapshot, asset_server)` re-inserts them with their original `AssetId` UUIDs, preserving handle identity so existing handles in a loaded scene still resolve.
- The baked format from improvement 4 can double as the on-disk representation here — a restore is just loading a collection of baked files keyed by UUID.

#### 9c. Scene File Format
With serializable handles and a restorable store, a scene file can be a simple structure:

```
{
  "assets": [
    { "id": "<uuid>", "source": "res/sponza/sponza.gltf" },
    { "id": "<uuid>", "source": "bake-cache/<uuid>.baked" }
  ],
  "entities": [ ... ECS component data with handle UUIDs ... ]
}
```

Loading a scene becomes: deserialize asset list → issue `load()` / bake-cache lookups → wait for all assets → deserialize entity data.

**Tradeoffs:**
- Requires a stable `AssetId` across runs for baked/procedural assets (use a deterministic UUID derived from content hash rather than a random one).
- Path-based handles are naturally stable; UUID-based handles need the bake cache to act as the source of truth.

---

## Priority Order

| Priority | Improvement | Effort | Impact |
|----------|-------------|--------|--------|
| 1 | Dependency tracking | Medium | Unlocks 4c, 6, 9c |
| 2 | Baked asset format | High | Largest load-time win |
| 3 | Asset load events | Low | Eliminates polling boilerplate |
| 4 | `add()` deduplication | Low | Fixes silent duplication |
| 5 | Usage-settings-aware path key | Low | Correctness fix |
| 6 | Handle status API | Low | Ergonomics / loading screens |
| 7 | Handle and store serialization | Medium | Required for save files and scene editor |
| 8 | Hot reloading | Medium | Developer experience |
| 9 | Memory budget / LRU | High | Required for streaming |
