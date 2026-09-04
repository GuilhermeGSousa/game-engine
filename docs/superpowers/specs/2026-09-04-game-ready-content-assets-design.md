# Game-Ready Content Assets — Design

**Status:** approved for planning
**Date:** 2026-09-04
**Author:** brainstormed with Claude (Sonnet 5), reviewed and revised with Claude (Opus 5)

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
- **The content tree is committed to VCS.** It is the project's authored data, not derived output — the same stance Unreal and Godot take. This is what makes a fresh clone runnable without a manifest.
- The manifest cook (`assets.toml`, `crates/cook`, `run_cook`, `.cooked/*.bin`, `.index/`) is **deleted**, and the `cook`-flavored names are renamed to `content`/`import` throughout (full sweep — see *Rename sweep*).

Deliberately minimal for v1. An editor (planned) will later handle stale-reference cleanup, rename fixup, and re-import ergonomics; the format carries two forward-compat seams (below) so that layer is additive.

## Non-goals

- **No stable random ids / registry / redirectors now.** The header reserves the id field (seam A) so a future editor can mint stable ids, build a `path ↔ id` registry, redirect renames, and switch `AssetHandle` to the stable id — without a format-version break. Not built here.
- **No re-import machinery.** `import` re-run overwrites convention paths; a file the user renamed becomes a stale duplicate. Provenance-in-header, incremental skip, skip-deleted, `--extract`, and prior-output cleanup are all deferred.
- **No incremental import.** Every `import <source>` re-runs the full importer and rewrites all of that source's content files.
- **No batch import.** `import` takes one source per invocation. A convention-walking `import --all <dir>` is a natural follow-up but is not needed for reproducibility now that the content tree is committed.
- **No whole-tree reference-integrity check.** A `content-check` tool that scans the whole content tree is a future thing that seam B enables.
- **No Unreal Asset *Manager*** (Primary Asset Ids, asset bundles, chunk assignment). The useful borrowed idea is the Asset *Registry* — an index built by scanning lightweight headers — which seam B enables later.
- **No rename-fixup tooling.** Renaming a content asset: fix code references (`load::<T>("…")` string literals) by hand; asset→asset references embedded in payloads go dead until the referrer is re-imported or re-saved.
- **No git-LFS setup.** Deferred by explicit decision; see *Risks*.

## The model

### Identity & addressing

- A content asset lives at a project-relative path, e.g. `content/hero/body.<ext>`.
- `<ext>` is **one uniform engine extension for every content asset** (like `.uasset`), configured per project (see *Configuration*). It is cosmetic — the header `kind` is the authoritative type tag.
- Identity is `AssetId::from_path("content/hero/body.<ext>")` — the same `uuid_v5` hash used today. `AssetId` stays `AssetId(Uuid)`, both constructors (`from_path` v5, `new` v4) unchanged. `AssetHandle` (Strong/Weak serde enum) is unchanged.
- `#`-fragment addresses are simply never produced any more. (`AssetPath` never parsed them apart — the fragment was only ever part of the path *string* — so nothing needs "un-splitting".)
- Renaming or moving a file changes its identity. No index, no redirectors in v1.

### Path normalization and the runtime root

`AssetPath::new` currently **force-prefixes `"res/"`** and `address()` strips it back off — a vestige of the pre-cook loader that would double-prefix against a content root. This plan removes it:

- `AssetPath::new` normalizes separators and strips a leading `./` only. `address()` returns the normalized path verbatim. `AssetId::from_path(&path.address())` therefore hashes exactly the string the user wrote: `content/hero/body.gasset`.
- `ContentAssetRoot::Directory` defaults to **`<exe-dir>`** (was `<exe-dir>/res`); `ContentAssetRoot::UrlBase` defaults to **`<page origin>`** (was `<origin>/res`).
- Resolution is `<root>/<address>` → `<exe-dir>/content/hero/body.gasset` natively, `<origin>/content/hero/body.gasset` on wasm. No `res` segment anywhere.
- `crates/essential/tests/asset_path_address.rs` asserts the old `res/` behaviour and is rewritten accordingly.

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
    /// only — never the payload. See "Reference list caveat".
    references: Vec<AssetId>,
    /// Authoritative type tag == `A::name()` (e.g. "Mesh", "Scene").
    kind: String,
}
```

- `magic` is a raw prefix for cheap sniffing / clear "this is not a content asset" errors.
- `load::<T>(path)` reads magic + header; a `kind != T::name()` mismatch is a **hard load error**, not a warning.

### Import — `cargo run -p import -- raw/hero.gltf`

1. Select the offline importer by source extension, from the importer list the deleted `cook` binary used (`ImageImporter`, `GltfImporter`, `ObjImporter`) — moved to `crates/import`.
2. Run the importer **once**, with an `ImportContext` whose **sub-asset-id resolver** (new hook, below) is wired to content paths.
3. For every `EmittedSubAsset { name, asset_id, type_name, bytes, references }`, write `content_path_for(name)` as a content asset: `magic` + `bincode(ContentAssetHeader { format_version, asset_id, references, kind: type_name.into() })` + `bytes`. Plain overwrite.

`content_path_for(sub_name) = "<content-root>/<source-stem>/<sanitize(sub_name)>.<ext>"` where `<source-stem>` is the **source file name without extension** (`raw/a/b/hero.gltf` → `hero`); e.g. `content/hero/mesh_0.<ext>`, `content/hero/animation_12.<ext>`, `content/hero/scene.<ext>`. `sanitize` replaces `/` (`mesh/0` → `mesh_0`).

**Cross-references.** `ImportContext` gains:

```rust
// default resolver:
//   |sub_name| AssetId::from_path(&format!("{relative_source}#{sub_name}"))
type SubAssetIdResolver = Box<dyn Fn(&str) -> AssetId + Send + Sync>;
```

`ImportContext::sub_asset_id(name)` calls the resolver. The `import` bin supplies `|sub_name| AssetId::from_path(&content_path_for(sub_name))`. Because `content_path_for` is a pure function of the sub-asset name, a **single importer pass** suffices: an emitted `Scene` node's `MeshComponent` handle, a material's texture handle, etc. all bake `AssetId::from_path("content/hero/body.<ext>")` directly. (The default resolver stays defined for `ImportContext`'s own unit tests; no production caller uses it after the cook is deleted.)

`import` checks that every id in an emitted asset's `references` appears among *this source's* emitted set. **This check is weak in practice** — `Asset::referenced_sub_assets()` is defaulted-empty for every type except `Scene` and `StandardMaterial`, so it exercises little today. It is kept because it costs nothing and becomes meaningful as more types implement the method.

### Editor-authored save

```rust
fn save_content_asset<A: Asset>(value: &A, project_path: &Path) -> std::io::Result<()>
```

Writes `magic` + `bincode(ContentAssetHeader { format_version, asset_id: AssetId::from_path(<project-relative form>), references: value.referenced_sub_assets(), kind: A::name().into() })` + `bincode(value)`.

**Two roots, deliberately.** The runtime `ContentAssetRoot` is *exe-relative* (`<exe-dir>`, populated by the example's `build.rs` copy). An editor must save into the **project source tree** — the committed one — or the save is clobbered by the next `cargo build` and never reaches VCS. So `save_content_asset` takes a real filesystem path supplied by the caller and does **not** resolve through `ContentAssetRoot`. The practical dev workflow is for an editor to also point its runtime root at the project tree (`set_content_root(ContentAssetRoot::Directory(project_dir))`) so it loads and saves the same files; the spec does not build that, it just keeps the two roots distinct.

`Scene` is `Asset` + serde, so an editor serializes the live entity/component tree into a `Scene` and calls this — no glTF/FBX export.

**Reference list caveat.** `Asset::referenced_sub_assets()` is defaulted-empty; only `Scene` and `StandardMaterial` override it, and `Scene::referenced_sub_assets` returns `self.referenced_assets`, a field the *importer* populates — not maintained as an editor mutates the scene. So for v1 the header `references` list is trustworthy only for importer-produced assets. Making it trustworthy for editor-authored assets is deferred to the editor work and noted here so seam B's consumers know the limitation.

### Runtime loading

New module `essential::assets::content`:

```rust
struct ContentAssetHeader { /* as above */ }
fn read_content_asset(bytes: &[u8]) -> anyhow::Result<(ContentAssetHeader, &[u8])>
fn write_content_asset(header: &ContentAssetHeader, payload: &[u8]) -> Vec<u8>
```

The per-type loaders (`texture`, `mesh`, `material`, `scene`, `skeleton`, `clip`) fetch bytes through one helper:

```rust
async fn load_content_asset_bytes(
    root: &ContentAssetRoot,
    address: &str,          // AssetPath::address(), e.g. "content/hero/body.gasset"
    expected_kind: &str,
) -> anyhow::Result<Vec<u8>>   // payload, header stripped
```

- Resolve `<root>/<address>`; read (reusing the `async-fs` off-thread read); require the `magic` prefix; `read_content_asset`; assert `header.kind == expected_kind`; return the payload slice for the caller's existing `bincode::deserialize::<A>`.
- A missing file, absent magic, or `kind` mismatch is an error naming the resolved path and both kinds.
- **Phase 1 keeps a `.cooked/<hash>.bin` fallback** so the cook and the content system coexist while the examples are still on the old pipeline; **phase 2 deletes it** (see *Delivery phasing*).

`AssetLoadContext` already receives the `AssetPath` (loader first param), so no `AssetServer` signature change is needed. `AssetServer::add` / `load_by_id` / procedural `AssetStore` inserts are untouched — they never hit disk.

### Rename sweep

| Old | New |
|---|---|
| `crates/cook` (bin) | `crates/import` (bin) |
| `crates/asset-cook` (crate) | `crates/asset-import` (crate) |
| `CookedAssetRoot` (`Directory` / `UrlBase`) | `ContentAssetRoot` (same variants, new defaults — see *Path normalization*) |
| `AssetLoadContext::cooked_root()` / `set_cooked_root` | `content_root()` / `set_content_root` |
| `load_cooked_asset_bytes` | `load_content_asset_bytes` (new signature above) |
| `.cooked/<hash>.bin` layout | gone (content tree replaces it) |
| `.gitignore` cook-regeneration comments (3×) | removed with their entries |

Deleted outright: `run_cook`, `cook_source`, `CookReport`, `CookOptions`, `AssetManifest` / `ManifestEntry`, `SourceIndex`, `COOK_FORMAT_VERSION`, `cooked_file_path_for_id`, the `.index/` machinery, the manifest-driven validation pass, every `assets.toml`, and `AssetPath`'s `res/` normalization.

Kept in `crates/asset-import`: `Importer` trait, `ImportContext` (+ `SubAssetIdResolver`), `EmittedSubAsset`, `ImportOutputs`, `ImportError`, the per-importer `validate` hook. `DependencyEntry` / `hash_file_contents` / `ImportContext::track_dependency` are **retained as dead plumbing** — the importer bodies still call `track_dependency` and nothing consumes it until incremental import exists. Recorded here so a reviewer doesn't flag it as an oversight.

### Configuration

`content.toml` at each project root:

```toml
extension = "gasset"     # the uniform content-asset extension
root      = "content"    # content tree root, project-relative
```

Read by `import`; the **runtime does not read it** (the path carries the extension; the root comes from `ContentAssetRoot`). `--ext` / `--content-root` / `--config` flags override. Each workspace example is its own project: `examples/tech-demo/content.toml` + `examples/tech-demo/content/`, so `cargo run -p import -- examples/tech-demo/assets/UAL1.glb --config examples/tech-demo/content.toml` writes under `examples/tech-demo/content/`.

### Examples migration

`render-test`, `tech-demo`, `animation-test`. Per example:

- Keep DCC sources in the existing committed `assets/`; run `import` per source, producing `content/<stem>/…`.
- Rewrite `asset_server.load::<T>("<source>#<sub>")` → `load::<T>("content/<stem>/<sub>.<ext>")`.
- `build.rs` copies `content/` → `<exe-dir>/content/` (was `res/` → `<exe-dir>/res/`).
- **wasm** (`tech-demo`, `animation-test` only — `render-test` has no wasm harness): `index.html`'s `<link data-trunk rel="copy-dir" href="res" data-target-path="res"/>` becomes `href="content" data-target-path="content"`; also drop the stray `data-target-path="res"` on `animation-test/index.html`'s `rel="rust"` link.
- **`.gitignore`**: remove the three `examples/*/res/` entries and their comments. The `content/` trees are **committed**.
- Visual-verify each (XWayland screenshot recipe): Sponza textured; both characters animating.

## Delivery phasing

Two plans, so neither leaves the workspace red for long:

**Plan 1 — content assets alongside the cook.** `essential::assets::content` (header + read/write), `ContentAssetRoot` rename + `AssetPath` `res/` removal + new root defaults, `ImportContext` resolver hook, `crates/import` + `content.toml`, `save_content_asset`, loaders switched to content-first **with the `.cooked` fallback retained**. The cook, `assets.toml`, and the examples keep working untouched. Ships green and independently testable.

**Plan 2 — cut over and delete.** Import each example's sources, rewrite its `load()` calls, `build.rs` + `index.html` + `.gitignore`, commit the content trees, visual-verify; then delete `crates/cook` and the fallback, and finish the rename sweep (`crates/asset-cook` → `crates/asset-import`, strip the manifest/cook/index code).

## Testing

- `ContentAssetHeader` round-trip: `write_content_asset` → `read_content_asset` recovers header + exact payload; truncated / wrong-magic / wrong-`kind` buffers error cleanly.
- `AssetPath`: `new("content/x.gasset").address() == "content/x.gasset"` (no `res/`), and `AssetId::from_path` of that equals the id `import` wrote into the header.
- `import` on a small glTF fixture: writes the expected `content/<stem>/<sub>.<ext>` files; the extracted `Scene`'s `MeshComponent`/`MaterialComponent` handles carry `AssetId::from_path("content/<stem>/mesh_0.<ext>")`, not `fixture.gltf#mesh/0`.
- `ImportContext` with a custom resolver returns the resolver's id; the default still works for its own tests.
- Runtime: `load::<Mesh>("content/x/mesh_0.<ext>")` reads an `import`-written file; missing path errors naming the resolved location; `load::<Scene>` of a `Mesh` file is an error.
- `save_content_asset::<Scene>` → `load::<Scene>` round-trips the tree.
- The three examples build, import cleanly, and pass their visual check.
- CI gates unchanged: `cargo build --workspace` (zero warnings), `cargo test --workspace`, `cargo fmt --all -- --check`, `cargo clippy -- -A clippy::type_complexity -A clippy::too_many_arguments -D warnings`.

## Risks

- **Repo size.** Committing the three examples' content trees adds roughly **700 MB** of binary to git history (today's cooked output: 292 MB render-test, 215 MB tech-demo, 194 MB animation-test). Accepted by explicit decision, with git-LFS deferred. Note that adding LFS *after* the fact requires a history rewrite to actually shrink the repo — cheap on this branch (history is rewritten routinely here), expensive once merged and shared.
- **Renames are unguarded.** Path is identity and there is no fixup tooling; renaming a referenced content asset silently dead-references its referrers until re-import or re-save.
- **No reproducibility check.** With `assets.toml` gone, nothing verifies the committed content tree actually matches what `import` would produce from the committed sources. A CI "re-import and diff" step is a natural follow-up.

## Deferred (documented for the future editor)

- **Re-import**: provenance in the header; incremental skip; skip user-deleted sub-assets; `--extract sub=path`; delete prior auto-output before re-export.
- **Batch import** (`import --all <dir>` walking a source directory).
- **Stable identity**: mint random ids into the `asset_id` field; a `path ↔ id` registry (seam B); `AssetHandle` serializes the stable id; redirectors + "fix up references".
- **Whole-content-tree reference-integrity check** (`content-check`), via seam B.
- **Trustworthy header `references` for editor-authored assets.**
- **Per-asset import settings** (LOD, compression, axis fixups).
- **git-LFS** for the content trees.
