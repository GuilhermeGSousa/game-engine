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

- **No *minted* stable ids or redirectors now.** The header's `asset_id` field (seam A) stays `AssetId::from_path(address)` — deterministic, not a randomly-minted identity — through both phases. A future editor can replace it with a minted id and add rename redirectors without a format-version break. (Phase 2 *does* add an id → address registry — see *Registry* — but that registry maps today's deterministic ids, it does not require minting new ones.)
- **No re-import machinery.** `import` re-run overwrites convention paths; a file the user renamed becomes a stale duplicate. Provenance-in-header, incremental skip, skip-deleted, `--extract`, and prior-output cleanup are all deferred.
- **No incremental import.** Every `import <source>` re-runs the full importer and rewrites all of that source's content files.
- **No batch import.** `import` takes one source per invocation. A convention-walking `import --all <dir>` is a natural follow-up but is not needed for reproducibility now that the content tree is committed.
- **No whole-tree reference-integrity check.** A `content-check` tool that scans the whole content tree is a future thing that seam B enables.
- **No Unreal Asset *Manager*** (Primary Asset Ids, asset bundles, chunk assignment for packaging/streaming) — that's shipping-scale infrastructure nothing here needs. (Its metadata-index idea, scoped down to the one lookup Phase 2 actually needs, *is* in scope — see *Registry*.)
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

The per-type loaders (`texture`, `mesh`, `material`, `scene`, `skeleton`, `clip`) fetch bytes through one helper. (Named `load_asset_bytes` in the shipped Plan 1 code — the signature below matches what actually landed; this doc originally sketched it as `load_content_asset_bytes` before implementation.)

```rust
async fn load_asset_bytes(
    root: &CookedAssetRoot,   // renamed ContentAssetRoot in Phase 2 — see Rename sweep
    address: &str,            // AssetPath::address(), e.g. "content/hero/body.gasset"
    id: AssetId,               // still needed in Plan 1, for the .cooked fallback; dropped in Phase 2
    expected_kind: &str,
) -> anyhow::Result<Vec<u8>>   // payload, header stripped
```

- Resolve `<root>/<address>`; read (reusing the `async-fs` off-thread read); require the `magic` prefix; `read_content_asset`; assert `header.kind == expected_kind`; return the payload slice for the caller's existing `bincode::deserialize::<A>`.
- A missing file, absent magic, or `kind` mismatch is an error naming the resolved path and both kinds.
- **Plan 1 keeps a `.cooked/<hash>.bin` fallback** (and a short-circuit for empty / `#`-fragment addresses straight to it) so the cook and the content system coexist while the examples are still on the old pipeline. **Phase 2 deletes the fallback entirely** and resolves `load_by_id`'s path-less case a different way — see *Registry*.

`AssetLoadContext` already receives the `AssetPath` (loader first param), so no `AssetServer` signature change is needed. `AssetServer::add` / procedural `AssetStore` inserts are untouched — they never hit disk. `load_by_id` (below) changes in Phase 2.

### Registry (Phase 2)

Plan 1's loaders find bytes by *address*. `load_by_id(id)` (used by every component's `apply()` to upgrade a deserialized `Weak` handle — `MeshComponent`, `MaterialComponent`, `SkeletonComponent`, `StandardMaterial`'s textures, camera render targets) has no address, only the id — today it works because the id doubles as the `.cooked/<hex>.bin` filename, a pure function needing no lookup. Once content assets live at their literal human path instead of a hash-derived one, that trick is gone: the id is a one-way hash of the path, so a caller with only the id has no way back to it. Phase 2 needs an explicit **id → address index** — an Asset Registry, in Unreal's sense (the metadata-index sense, not the Primary-Asset-Id/bundle sense already ruled out in *Non-goals*).

**File:** `<content-root>/.registry.toml` — e.g. `content/.registry.toml`. Plain TOML (not `GRDY`-framed; it is infrastructure, not a game asset), keyed by `AssetId::simple_hex()`:

```toml
[assets]
"8a3f1c2e9b4d4a1f8e6c2b3d4f5a6b7c" = "content/hero/body.gasset"
"1c908c1a2b3d4e5f6a7b8c9d0e1f2a3b" = "content/hero/hero.scene.gasset"
```

Living *inside* the content root (not beside it at the project root) means it needs no special-casing: `build.rs` already copies `content/` next to the binary, and Trunk's `copy-dir` already ships it on wasm, so the registry rides along automatically and resolves through the same `<root>/<address>` fetch every content asset uses — `content/.registry.toml` is just another well-known, fixed address.

**Written by `import` and `save_content_asset`, directly, on every write.** Each reads the existing registry (empty if absent), upserts an entry per content asset it just wrote (`id → address`), writes the file back. No pruning of stale entries — an old sub-asset the source no longer emits stays registered pointing at a file that may no longer be written, which surfaces as a load error rather than silent corruption (ties to the *No re-import machinery* non-goal: nothing currently detects "this sub-asset is gone"). Because every content asset's header independently carries its own `asset_id` (seam A) and `kind`, the registry is *derived data*, not a second source of truth — a future `content-check`/rebuild tool can always regenerate it from scratch by scanning the tree's headers (cheap: header-only, no payload read), which is the disaster-recovery path if it's ever suspected to have drifted. That tool is not built now.

**Read by `AssetServer::request_load`, only for path-less loads.** Today `request_load`'s spawned async task calls `loader.load(path, ctx, ...)` where a path-less call gets `path = AssetPath::new("")`. In Phase 2, before that call, when `path` was `None` the task instead: loads the registry once (first use; native via the same off-thread read every content fetch uses, wasm via one `reqwest::get`; cached in `AssetServer`'s data for subsequent calls), looks up `id`, and builds a real `AssetPath` from the address it finds — **or errors** if `id` is not registered (there is no `.cooked` left to fall back to). The loader then runs exactly as it does for a normal `load()` call, with a genuine address; **loaders never see an empty address in Phase 2** and gain no registry-awareness of their own. `load_asset_bytes`'s empty/`#` short-circuit and its `id`/`.cooked` fallback (Plan 1) are deleted along with the fallback they protected — nothing produces a `#`-fragment address once the manifest cook is gone, and nothing calls it with an empty one once `request_load` resolves `load_by_id` upstream. The helper's signature loses its `id` parameter entirely (renamed `load_content_asset_bytes` per *Rename sweep*, taking only `root` and `address`).

### Rename sweep

| Old | New |
|---|---|
| `crates/cook` (bin) | `crates/import` (bin) |
| `crates/asset-cook` (crate) | `crates/asset-import` (crate) |
| `CookedAssetRoot` (`Directory` / `UrlBase`) | `ContentAssetRoot` (same variants, new defaults — see *Path normalization*) |
| `AssetLoadContext::cooked_root()` / `set_cooked_root` | `content_root()` / `set_content_root` |
| `load_asset_bytes` (Plan 1's actual name; `load_cooked_asset_bytes` deleted outright, see below) | `load_content_asset_bytes` — drops the `id` parameter, drops the empty/`#`/fallback branches (see *Registry*) |
| `.cooked/<hash>.bin` layout | gone (content tree + registry replace it) |
| `.gitignore` cook-regeneration comments (3×) | removed with their entries |

Deleted outright: `run_cook`, `cook_source`, `CookReport`, `CookOptions`, `AssetManifest` / `ManifestEntry`, `SourceIndex`, `COOK_FORMAT_VERSION`, `cooked_file_path_for_id`, `load_cooked_asset_bytes`, the `.index/` machinery, the manifest-driven validation pass, every `assets.toml`, and `AssetPath`'s `res/` normalization.

New in Phase 2 (not a rename): `content/.registry.toml` per project and the registry read/write code — see *Registry*.

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

**Plan 1 — content assets alongside the cook** (`docs/superpowers/plans/2026-09-04-content-assets-phase-1.md`). `essential::assets::content` (header + read/write), `save_content_asset`, the `ImportContext` resolver hook, `crates/import` + `content.toml`, and loaders switched to content-first **with the `.cooked` fallback retained**. **Purely additive** — nothing is renamed, deleted, or repointed; the cook, `assets.toml`, and all three examples keep working exactly as they do today.

**Plan 2 — cut over and delete.** Import each example's sources, rewrite its `load()` calls, `build.rs` + `index.html` + `.gitignore`, commit the content trees, visual-verify; build the registry (*Registry*) and move `load_by_id`'s resolution into `AssetServer::request_load`; then delete `crates/cook`, `load_cooked_asset_bytes`, and Plan 1's content-first fallback (including its empty/`#`-address short-circuit, now unreachable), and do the whole rename sweep — including `AssetPath`'s `res/` removal and the new `ContentAssetRoot` defaults.

> The `res/` removal and the root-default change (`<exe-dir>/res` → `<exe-dir>`) belong to **Plan 2**, not Plan 1: moving the root relocates where cooked files are found and breaks the examples' `build.rs`, so they must land together with the example cutover.

## Testing

- `ContentAssetHeader` round-trip: `write_content_asset` → `read_content_asset` recovers header + exact payload; truncated / wrong-magic / wrong-`kind` buffers error cleanly.
- `AssetPath`: `new("content/x.gasset").address() == "content/x.gasset"` (no `res/`), and `AssetId::from_path` of that equals the id `import` wrote into the header.
- `import` on a small glTF fixture: writes the expected `content/<stem>/<sub>.<ext>` files; the extracted `Scene`'s `MeshComponent`/`MaterialComponent` handles carry `AssetId::from_path("content/<stem>/mesh_0.<ext>")`, not `fixture.gltf#mesh/0`.
- `ImportContext` with a custom resolver returns the resolver's id; the default still works for its own tests.
- Runtime: `load::<Mesh>("content/x/mesh_0.<ext>")` reads an `import`-written file; missing path errors naming the resolved location; `load::<Scene>` of a `Mesh` file is an error.
- `save_content_asset::<Scene>` → `load::<Scene>` round-trips the tree.
- The three examples build, import cleanly, and pass their visual check.
- CI gates unchanged: `cargo build --workspace` (zero warnings), `cargo test --workspace`, `cargo fmt --all -- --check`, `cargo clippy -- -A clippy::type_complexity -A clippy::too_many_arguments -D warnings`.
- **Phase 2 additions:** `import` (and `save_content_asset`) upserts a real entry into `content/.registry.toml`, preserving pre-existing unrelated entries; a second import of a different source doesn't clobber the first's entries. `AssetServer::load_by_id(id)` for a registered id resolves and loads the right asset (proves the `request_load`-level lookup, not just the registry file format). `load_by_id` for an unregistered id errors, naming the id, not a `.cooked` path (there is none). A scene spawn that exercises `MeshComponent`/`MaterialComponent`/`SkeletonComponent` handle-upgrade (i.e. `load_by_id` in practice) still works end-to-end — this is the exact class of bug Plan 1's final review found invisible to unit tests, so Phase 2's test plan must include a real `AssetServer` + spawn path, not only `load_asset_bytes`-level tests.

## Risks

- **Repo size.** Committing the three examples' content trees adds roughly **700 MB** of binary to git history (today's cooked output: 292 MB render-test, 215 MB tech-demo, 194 MB animation-test). Accepted by explicit decision, with git-LFS deferred. Note that adding LFS *after* the fact requires a history rewrite to actually shrink the repo — cheap on this branch (history is rewritten routinely here), expensive once merged and shared.
- **Renames are unguarded.** Path is identity and there is no fixup tooling; renaming a referenced content asset silently dead-references its referrers until re-import or re-save. In Phase 2 this also silently orphans that asset's registry entry (stale, pointing at a since-moved file) until the next `import`/`save` of *something* touches that id again — nothing currently detects or reports it.
- **No reproducibility check.** With `assets.toml` gone, nothing verifies the committed content tree actually matches what `import` would produce from the committed sources. A CI "re-import and diff" step is a natural follow-up.
- **The registry can drift from the tree** (an entry pointing at a deleted/moved file, or a file whose id isn't registered) with no automatic detection — recoverable only by the not-yet-built rebuild tool (*Deferred*). Because it's derived from header data that already exists, this is a bounded, fixable risk rather than data loss, but Phase 2 ships without the tool that fixes it.

## Deferred (documented for the future editor)

- **Re-import**: provenance in the header; incremental skip; skip user-deleted sub-assets (including pruning their stale registry entries); `--extract sub=path`; delete prior auto-output before re-export.
- **Batch import** (`import --all <dir>` walking a source directory).
- **Minted stable identity**: replace `asset_id`'s deterministic `from_path` value with a randomly-minted id at import/save time; `AssetHandle` serializes the minted id instead of (or alongside) the path-derived one; rename redirectors + a "fix up references" batch op. (The Phase-2 *registry* — id → address — ships without this; it indexes today's deterministic ids.)
- **Registry rebuild tool** (`content-check` or similar): regenerate `content/.registry.toml` from a full header scan of the tree, and separately validate the whole tree's reference graph (seam B) — the registry's disaster-recovery path and a whole-content-tree reference-integrity check, in one tool.
- **Trustworthy header `references` for editor-authored assets.**
- **Per-asset import settings** (LOD, compression, axis fixups).
- **git-LFS** for the content trees.
