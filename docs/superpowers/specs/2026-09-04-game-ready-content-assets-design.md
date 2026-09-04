# Game-Ready Content Assets — Design

**Status:** approved for planning
**Date:** 2026-09-04
**Author:** brainstormed with Claude

## Problem

Today an asset's identity is *derived* from its DCC source path: `AssetServer::load("Sponza/Sponza.gltf#scene")` computes `AssetId = uuid_v5("Sponza/Sponza.gltf#scene")`, the manifest cook writes `.cooked/<hash>.bin` keyed by that id, and `assets.toml` is a flat list of sources to cook. Consequences:

- The game references the DCC source, not a game-ready artifact. Renaming or reorganizing `assets/` breaks every reference.
- One `.gltf` fans out into fixed `source.gltf#mesh/0`, `source.gltf#material/0`, … addresses — the user cannot name or file the extracted pieces.
- There is no place for assets that have **no 1:1 source**: a `Scene` authored in an editor, a material tweaked in-engine, a mesh composed from two imports. A `Scene` can only be produced by re-exporting to glTF/FBX.

`Scene`, `AnimationGraph`, `StandardMaterial`, `Mesh`, `Texture`, etc. are all `Asset: Serialize + DeserializeOwned` now (the trait merge). That makes a direct "serialize a game-ready asset to a file" model possible.

## Goal

Replace the manifest cook with a **content asset** system:

- The user runs an explicit `import` of a DCC source; it writes game-ready files under a content tree the user organizes and names.
- Editor-authored assets (`Scene` first) save straight to the same file format — no DCC round-trip.
- The game references content assets by their project-relative path.
- The manifest cook (`assets.toml`, `crates/cook`, `run_cook`, `.cooked/*.bin`, `.index/`) is **deleted**, and the `cook`-flavored names are renamed to `content`/`import` throughout (full sweep — see *Rename sweep*).

Deliberately minimal for v1. An editor (planned) will later handle stale-reference cleanup, rename fixup, and re-import ergonomics; the format carries two forward-compat seams (below) so that layer is additive.

## Non-goals

- **No stable random ids / registry / redirectors now.** The header reserves the id field (seam A) so a future editor can mint stable ids, build a `path ↔ id` registry, redirect renames, and switch `AssetHandle` to the stable id — without a format-version break. Not built here.
- **No re-import machinery.** `import` re-run overwrites convention paths; a file the user renamed becomes a stale duplicate. Provenance-in-header, incremental skip, skip-deleted, `--extract`, and prior-output cleanup are all deferred.
- **No incremental import.** Every `import <source>` re-runs the full importer and rewrites all of that source's content files. `import` is an explicit, occasional operation; there is no `.index`-style skip.
- **No whole-tree reference-integrity check.** The old `run_cook` validated that every emitted reference id resolved across the entire manifest. `import` validates only within a single source's emitted set. A `content-check` tool that scans the whole content tree is a future thing that seam B enables.
- **No Unreal Asset *Manager*** (Primary Asset Ids, asset bundles, chunk assignment for packaging/streaming). The useful borrowed idea is the Asset *Registry* — an index built by scanning lightweight headers — which seam B enables later.
- **No rename-fixup tooling.** Renaming a content asset: fix code references (`load::<T>("…")` string literals) by hand; asset→asset references embedded in payloads go dead until the referrer is re-imported or re-saved.

## The model

### Identity & addressing

- A content asset lives at a project-relative path, e.g. `content/hero/body.<ext>`.
- `<ext>` is **one uniform engine extension for every content asset** (like `.uasset`), configured per project (see *Configuration*). It is cosmetic — the header `kind` is the authoritative type tag.
- Identity is `AssetId::from_path("content/hero/body.<ext>")` — the same `uuid_v5` hash used today. `AssetId` stays `AssetId(Uuid)`, both constructors (`from_path` v5, `new` v4) unchanged. `AssetHandle` (Strong/Weak serde enum) is unchanged.
- `#`-fragment addressing (`"source.gltf#sub"`) has no meaning anymore — nothing produces sub-asset-fragment assets. `AssetPath` becomes a plain path wrapper; its sub-asset split is removed.
- Renaming or moving a file changes its identity. No index, no redirectors in v1.

### File format

```
offset 0   magic          b"GRDY"                     (4 bytes)
           u32   header_len                            (little-endian)
           header_len bytes:  bincode(ContentAssetHeader)
           <to EOF>:          bincode(payload: A)
```

```rust
struct ContentAssetHeader {
    format_version: u32,
    /// Seam A. v1: AssetId::from_path(<this file's project-relative path>).
    /// Redundant with the path today (from_path is pure); stored as the seam
    /// for a future editor to mint stable ids here instead.
    asset_id: AssetId,
    /// Seam B. Outbound references, so a registry/editor scan reads headers
    /// only — never the payload. Importer-produced assets: the emitted
    /// `references`. Editor-authored assets: see "Reference list caveat".
    references: Vec<AssetId>,
    /// Authoritative type tag == `A::name()` (e.g. "Mesh", "Scene").
    kind: String,
}
```

- `magic` is a raw prefix for cheap sniffing / clear "this is not a content asset" errors.
- `load::<T>(path)` reads magic + header; a `kind != T::name()` mismatch is a **hard load error**, not a warning. (`kind` shares the stable-string hazard noted for enum variant order elsewhere — engine-owned types, low risk, but the loader must compare exactly.)

### Import — `cargo run -p import -- raw/hero.gltf`

1. Select the offline importer by source extension, from the importer list the deleted `cook` binary used (`ImageImporter`, `GltfImporter`, `ObjImporter`) — moved to `crates/import`.
2. Run the importer **once**, with an `ImportContext` whose **sub-asset-id resolver** (new hook, below) is wired to content paths.
3. For every `EmittedSubAsset { name, asset_id, type_name, bytes, references }`, write `content_path_for(name)` as a content asset: `magic` + `bincode(ContentAssetHeader { format_version, asset_id, references, kind: type_name.into() })` + `bytes`. Plain overwrite.

`content_path_for(sub_name) = "<content-root>/<source-stem>/<sanitize(sub_name)>.<ext>"` where `<source-stem>` is the **source file name without extension** (`raw/a/b/hero.gltf` → `hero`, not `a/b/hero`); e.g. `content/hero/mesh_0.<ext>`, `content/hero/animation_12.<ext>`, `content/hero/scene.<ext>`. `sanitize` replaces `/` (`mesh/0` → `mesh_0`).

**Cross-references.** `ImportContext` gains:

```rust
// default resolver:
//   |sub_name| AssetId::from_path(&format!("{relative_source}#{sub_name}"))
type SubAssetIdResolver = Box<dyn Fn(&str) -> AssetId + Send + Sync>;
```

`ImportContext::sub_asset_id(name)` calls the resolver. The `import` bin supplies `|sub_name| AssetId::from_path(&content_path_for(sub_name))`. Because `content_path_for` is a pure function of the sub-asset name (no importer run needed to know it), a **single importer pass** suffices: an emitted `Scene` node's `MeshComponent` handle, a material's texture handle, etc. all bake `AssetId::from_path("content/hero/body.<ext>")` directly, and `EmittedSubAsset.asset_id` / `.references` come out as content-path ids. (The default resolver remains defined for `ImportContext`'s own unit tests; no production caller uses it after the cook is deleted.)

`import` validates that every id in an emitted asset's `references` appears among *this source's* emitted set (the intra-source half of the old validation pass). No global check.

### Editor-authored save

```rust
fn save_content_asset<A: Asset>(value: &A, path: &str) -> std::io::Result<()>
```

Writes `magic` + `bincode(ContentAssetHeader { format_version, asset_id: AssetId::from_path(path), references: value.referenced_sub_assets(), kind: A::name().into() })` + `bincode(value)`.

`Scene` is `Asset` + serde, so an editor serializes the live entity/component tree into a `Scene` and calls this — no glTF/FBX export.

**Reference list caveat.** `Asset::referenced_sub_assets()` is defaulted-empty; only `Scene` and `StandardMaterial` override it, and `Scene::referenced_sub_assets` returns `self.referenced_assets`, a field the *importer* populates — not maintained as an editor mutates the scene. So for v1 the header `references` list is trustworthy only for importer-produced assets. Making it trustworthy for editor-authored assets (a real `referenced_sub_assets` on every content-authorable type, or a serde-tree walk for `AssetId`-shaped values at save time) is deferred to the editor work and noted here so seam B's consumers know the limitation.

### Runtime loading

New module `essential::assets::content`:

```rust
struct ContentAssetHeader { /* as above */ }
fn read_content_asset(bytes: &[u8]) -> anyhow::Result<(ContentAssetHeader, &[u8])>  // (header, payload slice)
fn write_content_asset(header: &ContentAssetHeader, payload: &[u8]) -> Vec<u8>
```

The per-type loaders (`texture`, `mesh`, `material`, `scene`, `skeleton`, `clip`) fetch bytes through **one** helper — **content path only, no fallback**:

```rust
async fn load_content_asset_bytes(
    root: &ContentAssetRoot,
    asset_path: &str,
    expected_kind: &str,
) -> anyhow::Result<Vec<u8>>   // returns the payload, header stripped
```

- Resolve `<root>/<asset-path>` (native: `<res-dir>/content/hero/body.<ext>`, copied next to the binary the way `res/` already is; wasm: `<origin>/content/hero/body.<ext>` over HTTP).
- Read; require the `magic` prefix; `read_content_asset`; assert `header.kind == expected_kind`; return the payload slice for the caller's existing `bincode::deserialize::<A>`.
- A missing file or absent magic is a plain error naming the resolved path. There is no `.cooked/<hash>.bin` anymore.
- Reuses the `async-fs` off-thread read already in place.

`AssetLoadContext` already receives the `AssetPath` (loader first param), so no `AssetServer` signature change is needed. `AssetServer::add` / `load_by_id` / procedural `AssetStore` inserts are untouched — they never hit disk.

### Rename sweep

Done in this plan:

| Old | New |
|---|---|
| `crates/cook` (bin) | `crates/import` (bin) |
| `crates/asset-cook` (crate) | `crates/asset-import` (crate) |
| `CookedAssetRoot` (`Directory` / `UrlBase`) | `ContentAssetRoot` (same variants) |
| `AssetLoadContext::cooked_root()` / `set_cooked_root` | `content_root()` / `set_content_root` |
| `load_cooked_asset_bytes` | `load_content_asset_bytes` (new signature above) |
| `.cooked/<hash>.bin` layout | gone (content tree replaces it) |

Deleted outright (not renamed): `run_cook`, `cook_source`, `CookReport`, `CookOptions`, `AssetManifest` / `ManifestEntry`, `SourceIndex`, `COOK_FORMAT_VERSION`, `cooked_file_path_for_id`, the `.index/` incremental machinery, the manifest-driven validation pass, every `assets.toml`.

Kept in `crates/asset-import`: `Importer` trait, `ImportContext` (+ `SubAssetIdResolver`), `EmittedSubAsset`, `ImportOutputs`, `ImportError`, `DependencyEntry`, `hash_file_contents` (still useful for a future incremental import), the per-importer `validate` hook.

### Configuration

A project config file — `content.toml` at the project root:

```toml
extension = "gasset"     # the uniform content-asset extension
root      = "content"    # content tree root, project-relative
```

Read by `import` (and any future tool). The **runtime does not read it** — `load("content/hero/body.gasset")` carries the extension in the path, and the content root is joined from `ContentAssetRoot`. `--ext` / `--content-root` flags on `import` override the config. `import` resolves `content.toml` from the directory it is run in (or its `--config` flag); each workspace example is its own "project" with its own `content.toml` + `content/` tree (`examples/tech-demo/content.toml`, `examples/tech-demo/content/…`), so `cargo run -p import -- examples/tech-demo/assets/UAL1.glb --config examples/tech-demo/content.toml` writes under `examples/tech-demo/content/`.

### Examples migration (mandatory scope)

`render-test`, `tech-demo`, `animation-test` load nothing once the manifest cook is gone. Per example:

- Move DCC sources out of the cooked `res/` into a non-shipped `assets/` (already the layout for the un-parked examples), run `cargo run -p import -- assets/<source>` per source, producing `content/<stem>/…`.
- Rewrite `asset_server.load::<T>("<source>#<sub>")` → `load::<T>("content/<stem>/<sub>.<ext>")`.
- `build.rs` copies `content/` next to the binary instead of `res/`; `.gitignore` swaps `res/` for `content/`.
- Visual-verify each (the project's XWayland screenshot recipe): Sponza textured; both characters animating.

## What changes vs. what is reused

**Reused unchanged:** `AssetId`, `AssetHandle`, `AssetStore`, `AssetServer::{load, add, load_by_id}`, `LoadTaskPool`, the `Asset` trait, every offline importer's *parsing* code (`GltfImporter`, `ObjImporter`, `ImageImporter` — they move crates but their bodies don't change), the `async-fs` off-thread read.

**New:**
- `crates/import` — the CLI bin + the importer list.
- `crates/asset-import` — `crates/asset-cook` renamed and stripped to the import primitives.
- `essential::assets::content` — `ContentAssetHeader`, `read_content_asset`, `write_content_asset`.
- `essential` — `save_content_asset::<A>` helper; `ContentAssetRoot` (renamed `CookedAssetRoot`).
- `ImportContext` — the `SubAssetIdResolver` hook.
- `content.toml` reader (in `import`).

**Deleted:** `crates/cook`, the manifest/cook/index code listed in *Rename sweep*, `.cooked/` and `.index/` on disk, every `assets.toml`.

## Testing

- `ContentAssetHeader` round-trip: `write_content_asset` → `read_content_asset` recovers header + exact payload bytes; a truncated / wrong-magic / wrong-`kind` buffer errors cleanly.
- `import` on a small glТf fixture: writes the expected `content/<stem>/<sub>.<ext>` files; the extracted `Scene`'s `MeshComponent`/`MaterialComponent` handles carry `AssetId::from_path("content/<stem>/mesh_0.<ext>")` (not `fixture.gltf#mesh/0`); each file's header `references` matches its payload's real outbound ids; an emitted reference to a sub-asset outside the source errors.
- `ImportContext` with a custom resolver: `sub_asset_id("mesh/0")` returns the resolver's id; the default (`<source>#mesh/0`) still works for `ImportContext`'s own tests.
- Runtime: `load::<Mesh>("content/x/mesh_0.<ext>")` reads a content file written by `import`; a missing path errors naming the resolved location; a `kind` mismatch (`load::<Scene>` of a `Mesh` file) is an error.
- `save_content_asset::<Scene>(&scene, "content/levels/a.<ext>")` then `load::<Scene>("content/levels/a.<ext>")` round-trips the tree.
- The three examples build, `import` cleanly, and pass their visual check.
- CI gates unchanged: `cargo build --workspace` (zero warnings), `cargo test --workspace`, `cargo fmt --all -- --check`, `cargo clippy -- -A clippy::type_complexity -A clippy::too_many_arguments -D warnings`.

## Deferred (documented for the future editor)

- **Re-import**: provenance (`{source, sub, source_hash}`) in the header; incremental skip; skip user-deleted sub-assets; `--extract sub=path`; delete prior auto-output before re-export.
- **Stable identity**: mint random ids into the `asset_id` field; a `path ↔ id` registry (built by scanning headers — seam B); `AssetHandle` serializes the stable id; redirectors + a "fix up references" batch op; rename-as-refactor.
- **Whole-content-tree reference-integrity check** (`content-check`), via seam B.
- **Trustworthy header `references` for editor-authored assets**: real `referenced_sub_assets()` on every content-authorable type, or a save-time serde-tree walk.
- **Per-asset import settings** (LOD, compression, axis fixups) in the header / a config.
