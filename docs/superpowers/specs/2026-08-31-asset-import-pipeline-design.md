# Asset Import Pipeline Design

## Problem

The engine's asset system (`crates/essential/src/assets/`, `AssetServer`/`AssetStore`/`AssetLoader`) loads DCC source formats (`.gltf`, `.obj`/`.mtl`, `.png`) directly at runtime, parsing them on every load. This has two problems:

1. **Runtime deserialization cost.** Parsing a `.gltf` document, decoding vertex/image data, etc. happens every time an asset is loaded, with no offline validation step to catch malformed source data before it reaches the running game.
2. **No sub-asset addressing.** A single DCC file can logically contain multiple engine assets (e.g. a `.gltf` containing several meshes, materials, and textures — some of which may be reusable by other assets). Today, loading any part of a `.gltf` requires loading and parsing the entire file (`crates/gltf-loader/src/loader.rs`), even to reach a single texture.

## Goals

- Introduce an offline **import ("cook") step** that converts DCC source assets into engine-ready, directly-deserializable binary assets ahead of runtime.
- Allow a single DCC source file to be split into multiple independently-addressable **engine assets** ("sub-assets"), so that loading one sub-asset (e.g. a texture) does not require touching unrelated sibling data (e.g. an entire `.gltf` document).
- Validate imported assets at cook time — both generic structural/reference checks and pluggable per-importer engine-specific constraints.
- Preserve existing behavior for consumers: ECS components still hold `AssetHandle<T>`, `AssetServer::load` remains the runtime entry point, and the existing Blender-glTF-extras-driven component injection (from PR #56) continues to work.

## Non-Goals

- Runtime fallback to parsing DCC source files directly. Once this design lands, the runtime never reads `.gltf`/`.obj`/`.png` again — only cooked output.
- Asset streaming, compression, or platform-specific cooked variants. Out of scope for this pass; the cooked format is a straightforward `bincode` serialization of the same in-memory structs used today.
- A GUI asset browser or hot-reload-on-source-change workflow. `cook` is a CLI step; re-running it is the only way to pick up source changes.

## Architecture Overview

Three new pieces, one repurposed:

1. **`Importer` trait** (new, offline-only) — one implementation per source format (glTF, OBJ/MTL, image). Given a source file path, produces a set of typed, named sub-assets, plus the source-side dependency files it read and the outbound asset-path references each sub-asset makes.
2. **`cook` tool** (new binary) — reads an explicit manifest of source assets to import (`assets.toml`), matches each to an `Importer` by extension, runs the import, serializes each sub-asset to a mirrored `cooked/` directory tree (one file per sub-asset), and writes a small per-source index file used only at cook time for incremental rebuilds and cross-source reference validation.
3. **Sub-asset addressing via a stable `AssetId`** — `AssetPath` gains an optional fragment (e.g. `models/character.gltf#texture/albedo`), used at human-facing entry points (the manifest resolves sources by path; `asset_server.load(path)` calls in game code). Internally, this string is hashed deterministically into an `AssetId` (`AssetId::from_path`, a UUID v5 hash — the same technique the existing `paths_to_uuid` helper in `gltf-loader` already uses for bone/animation IDs). The cooked file location is a pure function of the `AssetId` alone (e.g. `res/.cooked/<id>.bin`), so it can be computed both at cook time (which knows the path) and at runtime from a bare `AssetId` with no path in hand (which is the common case once one asset references another) — no index read at load time either way.
4. **`AssetHandle<T>` becomes serializable, Bevy-style** — `AssetHandle<A>` becomes an enum, `Strong(Arc<StrongAssetHandle>)` (today's only variant, behavior unchanged) or `Weak(AssetId)`. It implements `Serialize` (writes just the `AssetId`) and `Deserialize` (produces `Weak`). Because `AssetHandle<T>`'s public API (`.id()`) behaves identically for both variants, no code that only reads IDs — including the existing `#[derive(AsBindGroup)]` macro's generated bind-group code — needs to change.
5. **Runtime `AssetLoader` trait** (simplified) — becomes a thin `bincode` deserializer per asset type, reading directly from the cooked file addressed by `AssetId`. Because `AssetHandle<T>` is now directly serializable, asset types with cross-references (`StandardMaterial`, `SceneNode`) derive `Serialize`/`Deserialize` **directly on their existing definitions** — no separate "cooked" DTO types anywhere in this design. A `bincode::deserialize` on such a type produces `Weak` handles in its reference fields; each such type defines a `pub(crate) fn resolve_asset_handles(&mut self, asset_server: &AssetServer)` (in its own module, so it can touch private fields) that upgrades every `Weak(id)` field to `Strong` via `AssetServer::load_by_id`. A loader's `load()` is just: deserialize, call `resolve_asset_handles`, return — so loading a material transparently loads its textures, while loading a texture alone touches only that one cooked file.

## Core Types

Per project convention, all data types below use named struct fields — no unnamed tuples.

### `Importer` trait

```rust
trait Importer {
    fn supported_extensions(&self) -> &[&str];
    fn import(&self, source_path: &Path, ctx: &mut ImportContext) -> Result<(), ImportError>;
    fn validate(&self, output: &ImportOutput) -> Vec<ValidationIssue> {
        Vec::new() // default: no additional checks
    }
}
```

`ImportContext` is the write side used during `import()`:

- `ctx.emit(name: &str, sub_asset: &impl CookedAsset)` — records a sub-asset by name (e.g. `"mesh/0"`, `"texture/albedo"`, `"scene"`). `CookedAsset` is implemented directly on the real asset type being emitted (e.g. `StandardMaterial`, `Scene`, `Mesh` — no separate DTO), requiring `Serialize + DeserializeOwned`, a `TYPE_NAME`, and `fn referenced_sub_assets(&self) -> Vec<AssetId>` (default: empty) that reports the `AssetId` of every `AssetHandle<T>` field it holds. This is explicit per type, not generic reflection — YAGNI ruled out a reflection-based field walker since every type that holds references is small enough to list them by hand.
- `ctx.track_dependency(path: &Path)` — records a source-side file read (e.g. a `.gltf`'s referenced `.bin`/image files), used for incremental-rebuild hashing.

### Per-source index (cook-time only, never read by the runtime)

```rust
struct SourceIndex {
    source_path: PathBuf,
    source_hash: u64,
    sub_assets: Vec<SubAssetEntry>,
    dependencies: Vec<DependencyEntry>,
}

struct SubAssetEntry {
    name: String,
    asset_id: AssetId,          // AssetId::from_path("<relative source>#<name>")
    type_name: String,
    cooked_path: PathBuf,
    references: Vec<AssetId>,   // AssetIds of other sub-assets this one points to
}

struct DependencyEntry {
    path: PathBuf,
    content_hash: u64,
}
```

### Validation types

```rust
struct ValidationIssue {
    severity: ValidationSeverity, // Warning | Error
    message: String,
    source_path: PathBuf,
    sub_asset_name: Option<String>,
}
```

### Import manifest (`assets.toml`, project root)

```toml
[[assets]]
path = "models/character.gltf"

[[assets]]
path = "textures/skybox.png"
```

`cook` imports only the files listed here — no directory walking, no implicit discovery.

### `Scene` asset type (replaces `GLTFScene`)

```rust
struct SceneNode {
    name: String,
    transform: Transform,
    children: Vec<usize>,
    mesh: Option<AssetHandle<Mesh>>,
    material: Option<AssetHandle<StandardMaterial>>,
    skeleton: Option<AssetHandle<Skeleton>>,
    camera: Option<CameraData>,
    light: Option<LightData>,
    extra_components: Vec<ExtraComponentData>, // Blender-extras-driven, see PR #56
}

struct Scene {
    nodes: Vec<SceneNode>,
}
```

This is the same struct used both cooked and live — no separate DTO. `Scene`/`SceneNode` derive `Serialize`/`Deserialize` directly (their `AssetHandle<T>` fields serialize as bare `AssetId`s per the handle design above). `GltfImporter` emits this as the `"scene"` sub-asset. Any future DCC importer (e.g. FBX) targets the same `Scene` type, so the ECS spawn system (`spawn_scene_components`, renamed from `spawn_gltf_components`) is format-agnostic.

### Runtime `AssetLoader` trait (simplified)

```rust
trait AssetLoader {
    type Asset: Asset + DeserializeOwned;
    fn load(bytes: &[u8], asset_server: &AssetServer) -> Result<Self::Asset, LoadError> {
        let mut asset: Self::Asset = bincode::deserialize(bytes)?;
        asset.resolve_asset_handles(asset_server); // upgrades Weak -> Strong handles
        Ok(asset)
    }
}
```

Every asset type with `AssetHandle<T>` fields (`StandardMaterial`, `SceneNode`/`Scene`) implements `resolve_asset_handles` itself, in its own module, so it can reach private fields. `Mesh`/`Texture` have no handle fields, so their `resolve_asset_handles` is a no-op (or the trait gives it a default empty body and only overriding types implement it).

## Data Flow

### Cook time

1. `cook` reads `assets.toml`.
2. For each listed source asset, finds the matching `Importer` by file extension.
3. Checks the existing `SourceIndex` for that source (if any): if `source_hash` and every `DependencyEntry.content_hash` are unchanged, skip re-importing. If hashing any dependency fails (e.g. file missing), treat as dirty and re-import.
4. Otherwise runs `Importer::import`, collecting emitted sub-assets and dependencies into a new `SourceIndex`. Each sub-asset is serialized via `bincode` to its cooked file path (mirrored source directory structure + sanitized sub-asset name).
5. After all sources are processed, a global validation pass cross-checks every `SubAssetEntry.references` entry (an `AssetId`), across all `SourceIndex`es, against the full set of produced sub-assets' `AssetId`s. Any reference that doesn't resolve fails the cook run.
6. Each `Importer::validate()` hook runs per produced `ImportOutput`; `ValidationIssue`s with `severity: Error` fail the run, `Warning` ones are printed but don't.

### glTF importer specifically

The existing `GLTFLoader` parsing logic (`crates/gltf-loader/src/loader.rs`) — node graph, materials, meshes, texture cache/dedup, glTF `extras`-driven component injection via the `facet` reflection registry from PR #56 — moves into `GltfImporter`, reused near-verbatim. It emits: one `"mesh/N"` sub-asset per primitive, one `"material/N"` per material, one `"texture/<name>"` per unique decoded image (preserving today's sRGB/linear dedup-by-usage behavior), and one `"scene"` sub-asset holding the node hierarchy. Cross-sub-asset references (a material's texture, a node's mesh/material) are built as `AssetHandle::Weak(AssetId::from_path(...))` values directly on the real `StandardMaterial`/`SceneNode` structs being emitted — there is no intermediate string or DTO representation at any point.

### Runtime load (e.g. `asset_server.load::<Texture>("models/character.gltf#texture/albedo")`)

1. The input string is parsed for its optional `#fragment`, then hashed into a stable `AssetId` via `AssetId::from_path`.
2. `AssetServer::load_by_id::<Texture>(id)` computes the cooked file location as a pure function of `id` alone (no path, no index read) and reads it.
3. The file is deserialized via the asset type's `AssetLoader`, which also calls `resolve_asset_handles` to upgrade any nested `Weak` handles to `Strong` (recursively triggering further loads exactly like `load_by_id` does for the top-level asset — a material's texture references resolve this way, transparently).

## Error Handling

**Cook time:**
- `Importer::import` returning `Err(ImportError)` aborts cooking that one source file; `cook` logs the error, continues with the rest of the manifest, and exits non-zero if anything failed. `ImportError` variants (`SourceUnreadable`, `MalformedSource`, `MissingRequiredData`, etc.) each carry a `source_path` and `message` field.
- Reference-integrity failures are collected across the whole run and reported together, each naming the offending source asset, the missing reference name, and the sub-asset that declared it. This is build-breaking.
- `ValidationIssue`s with `severity: Warning` are printed but don't fail the run; `severity: Error` does.

**Runtime:**
- A missing cooked file for a requested `AssetId` is `LoadError::AssetNotFound`, carrying the `AssetId` (and, when the load originated from a human-facing path string rather than a nested reference, that original string too, for debuggability) — same failure shape as today's "file not found," just pointing at the cooked path.
- A cooked file that fails to deserialize (format/version mismatch, corruption) is a distinct `LoadError::CorruptCookedAsset` — a new failure mode this design introduces, since today's loaders had no separate binary format to desync from.
- There is no runtime fallback to parsing source DCC files. Stale or missing cooked assets are a build-time problem, fixed by re-running `cook`.

## Migration of Existing Loaders

Per the "full pipeline" scope decision, all three existing source-parsing crates are ported in this pass:

- `crates/gltf-loader` → `GltfImporter` (offline) + thin runtime `AssetLoader` impls for `Mesh`, `StandardMaterial`, `Texture`, `Scene`.
- `crates/obj-loader` (`obj_loader.rs`, `mtl_loader.rs`) → `ObjImporter`, emitting `Mesh`/`Material` sub-assets analogously.
- `crates/render/src/loaders/texture_loader.rs` → `ImageImporter`, emitting a single `Texture` sub-asset per source image (the trivial single-sub-asset case).

`AssetId` changes from a per-load random UUID to a deterministic one: `AssetId::from_path(&str)` (UUID v5 hash of the full `path#fragment` string) is used for every path-addressable asset, while `AssetId::new()` (today's random `v4`) remains for assets created purely in-memory via `AssetServer::add()`, which have no stable path to hash. `AssetHandle<T>` becomes an enum (`Strong`/`Weak`, see Architecture Overview) with `Serialize`/`Deserialize` writing/reading just the `AssetId`; `AssetServer` gains `load_by_id::<A>(id: AssetId) -> AssetHandle<A>`, and `load(path)` becomes `AssetId::from_path(path) → load_by_id(id)`. Today's `path_to_id` dedup map keyed by path string is no longer needed for path-addressable assets, since the same path always hashes to the same `AssetId` and the existing per-ID handle dedup (`AssetHandleProvider.asset_handles: HashMap<AssetId, ...>`) already provides reuse.

## Testing Strategy

- **Per-`Importer` unit tests**: fixture source file → `import()` → assert emitted sub-assets, recorded dependencies, and references match expectations.
- **Cook-tool integration tests**: fixture `assets.toml` + source tree → run `cook` against a temp output dir → assert the cooked file tree and a sample sub-asset round-trip through `bincode`.
- **Incremental-skip test**: cook, touch nothing, cook again → assert no re-import happened; modify a dependency file → assert it does re-import.
- **Reference-integrity negative test**: a fixture with a dangling reference → assert `cook` fails with the expected error.
- **Runtime loader tests**: cook a fixture asset, then `AssetServer::load` end-to-end, including a dedicated regression test asserting that loading a single texture sub-asset reads only that one cooked file.
- **ECS spawn test**: cook a fixture glTF with a node hierarchy and Blender-extras custom component, run `spawn_scene_components`, assert resulting entities/components — protects PR #56 behavior through the refactor.

## Open Items for the Implementation Plan

- Where the `cook` binary lives (new `crates/cook` vs an `xtask`-style dev-tool crate) and how it's invoked (manual step vs wired into a build script).
- `AssetId` collision risk under UUID v5 hashing is treated as negligible (standard assumption for content-addressed/deterministic-ID systems) and not otherwise mitigated in this pass.
- Whether `AssetHandle<T>`'s `Weak`/`Strong` split needs `PartialEq`/`Hash` impls beyond what exists today (some call sites may compare handles); audit existing `AssetHandle` usage for this during implementation.
- `resolve_asset_handles` is hand-written per type (not derived) — acceptable for the small number of types with reference fields in this pass (`StandardMaterial`, `SceneNode`), but worth revisiting if that count grows significantly.
