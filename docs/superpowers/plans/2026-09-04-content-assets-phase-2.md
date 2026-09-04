# Content Assets Phase 2: Cut Over and Delete — Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Delete the manifest-driven `cook` pipeline entirely, rename the surviving offline-import machinery, add an `AssetRegistry` + `ImportProvenance` so path-less loads and future editor tooling both work without the old `.cooked/<hash>.bin` naming-is-lookup trick, and migrate all three examples onto content assets as the sole asset pipeline.

**Architecture:** Content assets (Plan 1) already load in preference to the cooked layout with a fallback. This plan removes the fallback and everything it falls back to: `crates/cook`, `asset-cook`'s manifest/incremental/cook-report code, `CookedAssetRoot`'s `res/`-relative defaults, and `AssetPath`'s forced `"res/"` prefix. In their place: a renamed, slimmed `asset-import` crate holding only the `Importer` trait and `ImportContext`; a `content/.registry.toml`-backed `AssetRegistry` that `AssetServer::request_load` consults for path-less (`load_by_id`) loads; and `ContentAssetHeader.provenance`, so a content asset produced by `import` records which DCC source and sub-asset it came from. All three examples end up importing their DCC sources into a committed `content/` tree and loading exclusively by content-tree address.

**Tech Stack:** Rust workspace (2021/2024 edition mix per-crate), `serde`/`bincode` for the content-asset envelope, `toml` for the registry file, `async-fs`/`reqwest` for the native/wasm byte-read split already in place, `pollster` for blocking on async code in tests.

**Spec:** `docs/superpowers/specs/2026-09-04-game-ready-content-assets-design.md` (Plan 2 / "cut over and delete" section). This plan argues from that spec; where this plan is silent, the spec is authoritative.

## Global Constraints

- CI gates, run in this exact form after every task: `cargo build --workspace` (must produce zero warnings), `cargo test --workspace`, `cargo fmt --all -- --check`, `cargo clippy -- -A clippy::type_complexity -A clippy::too_many_arguments -D warnings` (no `--workspace`, no `--all-targets`).
- Commit messages end with `Co-Authored-By: Claude Sonnet 5 <noreply@anthropic.com>`.
- The content tree (`examples/*/content/`) is committed to version control, not gitignored — an explicit decision accepting the repo-size cost; git LFS is deferred.
- `AssetId` stays `Uuid`-backed and path-addressed exactly as today. The only changes are additive: `AssetId::from_simple_hex` (the inverse of `simple_hex()`) and an `Ord`/`PartialOrd` derive, both needed only for `AssetRegistry`'s deterministic on-disk ordering.
- No incremental reimport in this phase. `DependencyEntry`, `hash_file_contents`, and `ImportContext::track_dependency` are retained as intentional dead plumbing — importers still call `track_dependency`, but nothing consumes the recorded dependencies until a future incremental-import feature reads them back.
- `Importer::validate`, `ValidationSeverity`, and `ValidationIssue` are retained even though nothing calls `.validate()` once `run_cook` is deleted (no real importer overrides it either). They are forward-compatible surface, not exercised by this plan.
- Examples are allowed to be visually and functionally broken (assets fail to load at runtime) between Task 1 and their own migration task (5, 6, or 7). Only `cargo build --workspace` and `cargo test --workspace` staying green is required after every task — neither exercises example asset loading at runtime.
- `CONTENT_FORMAT_VERSION` bumps from 1 to 2 in Task 2 because `ContentAssetHeader` gains a field (`provenance`). No on-disk content asset from Plan 1 persists anywhere permanent (Plan 1's own tests only ever wrote to temp directories), so this bump needs no migration — old-version files simply don't exist to migrate.
- Visual (screenshot) verification of rendering correctness is the controller's responsibility during the final whole-branch review (mirroring Plan 1, where the controller personally ran render-test under XWayland to catch a bug invisible to the automated suite). Task-level verification for the example-migration tasks (5-7) checks that no `"Failed to load asset"` line appears in stderr during a timed run — see the recipe in Task 5.

---

## Task 1: Delete the manifest-cook pipeline; rename `asset-cook` to `asset-import`

**Files:**
- Delete: `crates/cook/` (whole directory — `Cargo.toml`, `src/main.rs`)
- Delete: `crates/asset-cook/src/cook.rs`, `crates/asset-cook/src/run.rs`, `crates/asset-cook/src/manifest.rs`
- Delete: `crates/asset-cook/tests/cook.rs`, `crates/asset-cook/tests/incremental.rs`, `crates/asset-cook/tests/validation.rs`
- Rename (directory + package): `crates/asset-cook/` → `crates/asset-import/`
- Modify: `crates/asset-import/Cargo.toml` (package name), `crates/asset-import/src/lib.rs`, `crates/asset-import/src/import_context.rs`
- Modify: `crates/gltf-loader/Cargo.toml`, `crates/obj-loader/Cargo.toml`, `crates/render/Cargo.toml`, `crates/import/Cargo.toml` (dependency path + name)
- Modify: `crates/mesh/Cargo.toml`, `crates/scene/Cargo.toml`, `crates/animation/Cargo.toml` (drop the now-pointless `asset-cook` dependency line — zero real usage in any of these three crates today)
- Modify: `crates/gltf-loader/src/gltf_importer.rs`, `crates/obj-loader/src/obj_importer.rs`, `crates/render/src/importers/image_importer.rs`, `crates/import/src/lib.rs` (import path)
- Modify: `crates/gltf-loader/tests/gltf_importer.rs`, `crates/obj-loader/tests/obj_importer.rs`, `crates/render/tests/image_importer.rs` (import path)
- Modify: `crates/render/tests/texture_pipeline_e2e.rs` (rewrite off the deleted `run_cook`/`CookOptions`/`cooked_file_path_for_id`)
- Modify: `crates/ecs/src/component/mod.rs` (stale doc comment referencing the deleted `cook` binary and `COOK_FORMAT_VERSION`)

**Interfaces:**
- Produces: crate `asset_import` re-exporting `hash_file_contents, DependencyEntry, EmittedSubAsset, ImportContext, ImportError, ImportOutputs, SubAssetIdResolver` plus the `Importer` trait, `ValidationSeverity`, `ValidationIssue` — identical surface to today's `asset_cook`, minus everything `cook`/`manifest`/`run` exported.
- Consumes: nothing new from later tasks. This task is a pure rename + deletion; `essential` is untouched.

- [ ] **Step 1: Delete `crates/cook`**

```bash
git rm -r crates/cook
```

- [ ] **Step 2: Delete the cook/manifest/incremental source and test files inside `asset-cook`**

```bash
git rm crates/asset-cook/src/cook.rs crates/asset-cook/src/run.rs crates/asset-cook/src/manifest.rs
git rm crates/asset-cook/tests/cook.rs crates/asset-cook/tests/incremental.rs crates/asset-cook/tests/validation.rs
```

- [ ] **Step 3: Move `hash_file_contents` into `import_context.rs`, the only source file that survives**

Edit `crates/asset-cook/src/import_context.rs` — change the top `use` line and insert the function right after `DependencyEntry`:

```rust
use std::path::{Path, PathBuf};

use essential::assets::{Asset, AssetId};

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct DependencyEntry {
    pub path: PathBuf,
    pub content_hash: u64,
}

/// Hashes a file's contents for [`ImportContext::track_dependency`]. Kept
/// as dead plumbing: importers still record dependencies, but nothing
/// consumes them until a future incremental-import feature reads them back.
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
```

Everything else in this file (`EmittedSubAsset`, `ImportError`, `ImportOutputs`, `SubAssetIdResolver`, `ImportContext` and its impl) is unchanged.

- [ ] **Step 4: Rewrite `crates/asset-cook/src/lib.rs`**

```rust
mod import_context;

pub use import_context::{
    hash_file_contents, DependencyEntry, EmittedSubAsset, ImportContext, ImportError,
    ImportOutputs, SubAssetIdResolver,
};

use std::path::{Path, PathBuf};

pub trait Importer: Send + Sync {
    fn supported_extensions(&self) -> &'static [&'static str];

    fn import(&self, source_path: &Path, ctx: &mut ImportContext) -> Result<(), ImportError>;

    fn validate(&self, _sub_assets: &[EmittedSubAsset]) -> Vec<ValidationIssue> {
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

- [ ] **Step 5: Rename the crate directory and package**

```bash
git mv crates/asset-cook crates/asset-import
```

Edit `crates/asset-import/Cargo.toml` — change `name = "asset-cook"` to `name = "asset-import"`. Leave every dependency line (`essential`, `serde`, `bincode`, `anyhow`, `log`) as-is; drop `toml = "0.8"` from `[dependencies]` if present — it belonged to the now-deleted `manifest.rs` and nothing else in this crate parses TOML. Leave `[dev-dependencies]` (`serde`) as-is.

- [ ] **Step 6: Repoint every dependent `Cargo.toml`**

In each of `crates/gltf-loader/Cargo.toml`, `crates/obj-loader/Cargo.toml`, `crates/render/Cargo.toml`, `crates/import/Cargo.toml`, change:

```toml
asset-cook = { path = "../asset-cook" }
```

to:

```toml
asset-import = { path = "../asset-import" }
```

In each of `crates/mesh/Cargo.toml`, `crates/scene/Cargo.toml`, `crates/animation/Cargo.toml`, delete the line `asset-cook = { path = "../asset-cook" }` entirely — grep confirms zero `asset_cook`/`asset_import` references in any `.rs` file under these three crates.

- [ ] **Step 7: Repoint every `use asset_cook::...`**

In `crates/gltf-loader/src/gltf_importer.rs`, `crates/obj-loader/src/obj_importer.rs`, `crates/render/src/importers/image_importer.rs`, `crates/import/src/lib.rs`, `crates/gltf-loader/tests/gltf_importer.rs`, `crates/obj-loader/tests/obj_importer.rs`, `crates/render/tests/image_importer.rs`: change every `use asset_cook::` to `use asset_import::`. The imported item lists are unchanged (e.g. `gltf_importer.rs` keeps `use asset_import::{ImportContext, ImportError, Importer, hash_file_contents};`).

- [ ] **Step 8: Rewrite `crates/render/tests/texture_pipeline_e2e.rs`**

The old version drove the deleted `run_cook`/`CookOptions`/`cooked_file_path_for_id`. Replace the whole file:

```rust
//! End-to-end proof that an imported texture round-trips through the
//! runtime byte-loading path — the same envelope every AssetLoader reads.
use asset_import::{ImportContext, Importer};
use essential::assets::content::{write_content_asset, ContentAssetHeader, CONTENT_FORMAT_VERSION};
use essential::assets::utils::load_asset_bytes;
use essential::assets::{AssetId, CookedAssetRoot};
use render::assets::texture::{Texture, TextureFormat, TextureKind};
use render::importers::image_importer::ImageImporter;

#[test]
fn an_imported_texture_is_reachable_at_its_conventional_address() {
    let temp_dir = std::env::temp_dir().join(format!("texture-e2e-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&temp_dir);
    std::fs::create_dir_all(&temp_dir).unwrap();

    let source_path = temp_dir.join("swatch.png");
    let img = image::RgbaImage::from_pixel(1, 1, image::Rgba([9, 9, 9, 255]));
    img.save(&source_path).expect("failed to save fixture texture");

    let mut ctx = ImportContext::new(std::path::PathBuf::from("swatch.png"));
    ImageImporter
        .import(&source_path, &mut ctx)
        .expect("importing the fixture texture must succeed");
    let outputs = ctx.into_parts();
    assert_eq!(
        outputs.sub_assets.len(),
        1,
        "one image yields one sub-asset"
    );
    let sub_asset = &outputs.sub_assets[0];

    let address = "content/swatch/main.gasset";
    let header = ContentAssetHeader {
        format_version: CONTENT_FORMAT_VERSION,
        asset_id: sub_asset.asset_id,
        references: sub_asset.references.clone(),
        kind: sub_asset.type_name.to_string(),
    };
    std::fs::create_dir_all(temp_dir.join("content/swatch")).unwrap();
    std::fs::write(
        temp_dir.join(address),
        write_content_asset(&header, &sub_asset.bytes).unwrap(),
    )
    .unwrap();

    let bytes = pollster::block_on(load_asset_bytes(
        &CookedAssetRoot::Directory(temp_dir.clone()),
        address,
        AssetId::from_path(address),
        Texture::name(),
    ))
    .expect("the imported texture must load back through the runtime byte path");

    let texture: Texture = bincode::deserialize(&bytes).expect("failed to deserialize texture");
    assert_eq!(texture.width, 1);
    assert_eq!(texture.height, 1);
    assert_eq!(texture.format, TextureFormat::Rgba8UnormSrgb);
    assert_eq!(texture.kind, TextureKind::Sampled);
    assert_eq!(texture.data, vec![9, 9, 9, 255]);

    let _ = std::fs::remove_dir_all(&temp_dir);
}
```

Add `use essential::assets::Asset;` if `Texture::name()` doesn't resolve without it (the `Asset` trait must be in scope for the associated function call). This test intentionally still calls `essential::assets::utils::load_asset_bytes` with a `CookedAssetRoot`/`id` argument and a `ContentAssetHeader` without `provenance` — those are exactly right for the code as it exists after this task; Tasks 2 and 3 rename/reshape them, not this one.

- [ ] **Step 9: Fix the stale doc comment in `crates/ecs/src/component/mod.rs`**

Around line 55, change:

```rust
    /// NOTE: `std::any::type_name` output is not guaranteed stable across
    /// compiler versions. Cooked scenes embed these strings, so a toolchain
    /// upgrade may require re-running `cook` — `COOK_FORMAT_VERSION` and the
    /// fact that cooked output is git-ignored make that a rebuild, not a
    /// migration.
```

to:

```rust
    /// NOTE: `std::any::type_name` output is not guaranteed stable across
    /// compiler versions. Scenes embed these strings, so a toolchain
    /// upgrade may require re-running `import` — `CONTENT_FORMAT_VERSION`
    /// changes make that a rebuild, not a migration.
```

- [ ] **Step 10: Build and test**

```bash
cargo build --workspace
cargo test --workspace
cargo fmt --all -- --check
cargo clippy -- -A clippy::type_complexity -A clippy::too_many_arguments -D warnings
```

Expected: all four green. `cargo test -p render texture_pipeline_e2e` specifically exercises Step 8's rewrite.

- [ ] **Step 11: Commit**

```bash
git add -A
git commit -m "refactor!: delete the manifest-cook pipeline; rename asset-cook to asset-import

crates/cook and asset-cook's manifest/incremental/run machinery are gone.
The surviving Importer trait + ImportContext move to crates/asset-import.

Co-Authored-By: Claude Sonnet 5 <noreply@anthropic.com>"
```

---

## Task 2: `AssetRegistry` + `ImportProvenance` (additive to `essential`)

**Files:**
- Modify: `crates/essential/Cargo.toml` (add `toml` dependency)
- Modify: `crates/essential/src/assets/mod.rs` (`AssetId::from_simple_hex`, `Ord`/`PartialOrd` derive)
- Modify: `crates/essential/src/assets/content.rs` (`ImportProvenance`, `ContentAssetHeader.provenance`, `CONTENT_FORMAT_VERSION` bump, `AssetRegistry`, `save_content_asset` upsert)
- Modify (minimally, to keep the workspace compiling — see note below): `crates/import/src/lib.rs`, `crates/essential/tests/content_first_loading.rs`, `crates/essential/tests/content_asset_format.rs`, `crates/render/tests/texture_pipeline_e2e.rs`
- Create: `crates/essential/tests/asset_registry.rs`

**Interfaces:**
- Produces: `essential::assets::content::{AssetRegistry, ImportProvenance, REGISTRY_FILE_NAME}`; `ContentAssetHeader.provenance: Option<ImportProvenance>`; `AssetId::from_simple_hex(hex: &str) -> Result<AssetId, uuid::Error>`. `AssetRegistry` methods: `new() -> Self`, `load(project_root: &Path) -> anyhow::Result<Self>`, `parse(text: &str) -> anyhow::Result<Self>`, `save(&self, project_root: &Path) -> anyhow::Result<()>`, `get(&self, id: AssetId) -> Option<&str>`, `insert(&mut self, id: AssetId, address: impl Into<String>)`, `remove(&mut self, id: AssetId) -> Option<String>`, `iter(&self) -> impl Iterator<Item = (AssetId, &str)>`.
- Consumes: nothing from Task 1's rename (this task never touches `crates/asset-import` or `crates/cook`).
- Note on scope: adding a field to `ContentAssetHeader` breaks every existing struct-literal construction site at compile time. There are four today: `crates/import/src/lib.rs`, `crates/essential/tests/content_first_loading.rs` (four literals), `crates/essential/tests/content_asset_format.rs` (one, in its shared `header()` helper), and `crates/render/tests/texture_pipeline_e2e.rs` (one, written fresh by Task 1). This task adds `provenance: None,` to all of them to keep the workspace compiling. Task 4 gives `crates/import/src/lib.rs` its real provenance population; Task 3 gives `content_first_loading.rs` its full rewrite and renames `texture_pipeline_e2e.rs`'s `load_asset_bytes`/`CookedAssetRoot` calls. `content_asset_format.rs` is not touched again by any later task — this task's one-line patch is its final form.

- [ ] **Step 1: Add the `toml` dependency**

In `crates/essential/Cargo.toml`, under `[dependencies]`, add (after the `bincode = "1.3"` line):

```toml
toml = "0.8"
```

- [ ] **Step 2: `AssetId::from_simple_hex` + ordering**

In `crates/essential/src/assets/mod.rs`, change the `AssetId` derive:

```rust
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub struct AssetId(Uuid);
```

Add this method inside `impl AssetId`, after `simple_hex`:

```rust
    /// Inverse of `simple_hex()` — parses an AssetId back from its
    /// 32-lowercase-hex-digit form. Used by `AssetRegistry` to reconstruct
    /// ids read back from `.registry.toml`.
    pub fn from_simple_hex(hex: &str) -> Result<Self, uuid::Error> {
        Uuid::parse_str(hex).map(AssetId)
    }
```

- [ ] **Step 3: Rewrite `crates/essential/src/assets/content.rs`**

```rust
//! Framing for game-ready content assets: `magic | u32 header_len |
//! bincode(ContentAssetHeader) | payload (verbatim)`. The header is read
//! without touching the payload, so a future asset registry can index a
//! whole content tree by scanning headers alone.
use std::collections::BTreeMap;
use std::path::Path;

use anyhow::{bail, Context};
use serde::{Deserialize, Serialize};

use super::{Asset, AssetId};

/// Leading bytes of every content asset file.
pub const CONTENT_ASSET_MAGIC: [u8; 4] = *b"GRDY";

/// Bumped whenever `ContentAssetHeader`'s on-disk shape changes
/// incompatibly. `read_content_asset` rejects a mismatch outright.
pub const CONTENT_FORMAT_VERSION: u32 = 2;

/// Where `AssetRegistry` lives, relative to the same root content-asset
/// addresses resolve against (a project root at import/save time, the
/// runtime `ContentAssetRoot` at load time).
pub const REGISTRY_FILE_NAME: &str = "content/.registry.toml";

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ContentAssetHeader {
    pub format_version: u32,
    /// Identity. Today always `AssetId::from_path(<project-relative path>)`,
    /// which is recomputable from the path — it is stored so a future editor
    /// can mint a stable id here instead without a format break.
    pub asset_id: AssetId,
    /// Outbound references, so a registry scan never reads payloads.
    pub references: Vec<AssetId>,
    /// Authoritative type tag; must equal the loading type's `Asset::name()`.
    pub kind: String,
    /// Where this content asset came from, if `import` produced it from a
    /// DCC source rather than an editor saving it directly.
    pub provenance: Option<ImportProvenance>,
}

/// The offline source (and sub-asset within it) that `import` produced a
/// content asset from — lets a future editor show "re-import" provenance
/// without re-deriving it from the address string.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ImportProvenance {
    /// The source path exactly as passed to `import_source`, not otherwise
    /// normalized against any project root.
    pub source: String,
    /// The sub-asset name within that source, e.g. `"mesh/0"`.
    pub sub_asset: String,
}

pub fn write_content_asset(header: &ContentAssetHeader, payload: &[u8]) -> anyhow::Result<Vec<u8>> {
    let header_bytes =
        bincode::serialize(header).context("failed to serialize content asset header")?;

    let header_len =
        u32::try_from(header_bytes.len()).context("content asset header exceeds 4 GiB")?;

    let mut out = Vec::with_capacity(8 + header_bytes.len() + payload.len());
    out.extend_from_slice(&CONTENT_ASSET_MAGIC);
    out.extend_from_slice(&header_len.to_le_bytes());
    out.extend_from_slice(&header_bytes);
    out.extend_from_slice(payload);
    Ok(out)
}

pub fn read_content_asset(bytes: &[u8]) -> anyhow::Result<(ContentAssetHeader, &[u8])> {
    if bytes.len() < 8 || bytes[..4] != CONTENT_ASSET_MAGIC {
        bail!("not a content asset: missing the GRDY magic prefix");
    }

    let header_len = u32::from_le_bytes(
        bytes[4..8]
            .try_into()
            .expect("slice of exactly 4 bytes is always a [u8; 4]"),
    ) as usize;
    let header_end = 8usize
        .checked_add(header_len)
        .context("content asset header length overflows")?;
    if bytes.len() < header_end {
        bail!(
            "content asset truncated: header claims {header_len} bytes, only {} available",
            bytes.len() - 8
        );
    }

    let header: ContentAssetHeader = bincode::deserialize(&bytes[8..header_end])
        .context("failed to deserialize content asset header")?;
    if header.format_version != CONTENT_FORMAT_VERSION {
        bail!(
            "unsupported content asset format version {} (this build expects {CONTENT_FORMAT_VERSION})",
            header.format_version
        );
    }
    Ok((header, &bytes[header_end..]))
}

/// Writes `value` as a content asset at `project_root/address`, creating
/// parent directories as needed, and upserts the asset registry so a
/// path-less (`AssetServer::load_by_id`) load can find it later.
///
/// `address` is the project-relative path (`"content/hero/body.gasset"`) and
/// is what the asset's id is hashed from; `project_root` is the source tree
/// an editor saves into, which is deliberately *not* the exe-relative
/// runtime root — a save must land in the tree under version control, not
/// beside the binary where the next build overwrites it.
pub fn save_content_asset<A: Asset>(
    value: &A,
    project_root: &Path,
    address: &str,
) -> anyhow::Result<()> {
    let asset_id = AssetId::from_path(address);
    let header = ContentAssetHeader {
        format_version: CONTENT_FORMAT_VERSION,
        asset_id,
        references: value.referenced_sub_assets(),
        kind: A::name().to_string(),
        provenance: None,
    };
    let payload = bincode::serialize(value).context("failed to serialize content asset payload")?;
    let bytes = write_content_asset(&header, &payload)?;

    let path = project_root.join(address);
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)
            .with_context(|| format!("failed to create '{}'", parent.display()))?;
    }
    std::fs::write(&path, bytes)
        .with_context(|| format!("failed to write content asset '{}'", path.display()))?;

    let mut registry = AssetRegistry::load(project_root)?;
    registry.insert(asset_id, address);
    registry.save(project_root)
}

/// A serialized `[assets]` table in `.registry.toml`: `AssetId::simple_hex()`
/// keys to content-tree address values.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
struct RegistryFile {
    #[serde(default)]
    assets: BTreeMap<String, String>,
}

/// Maps AssetIds to their content-tree address. Backs `AssetServer`'s
/// path-less (`load_by_id`) resolution now that content assets live at
/// literal human paths instead of the old `.cooked/<hex>.bin`
/// naming-is-lookup convention: `import` and `save_content_asset` upsert it
/// directly, merge-only, never pruned.
#[derive(Debug, Clone, Default)]
pub struct AssetRegistry {
    entries: BTreeMap<AssetId, String>,
}

impl AssetRegistry {
    pub fn new() -> Self {
        Self::default()
    }

    /// Loads `<project_root>/content/.registry.toml`, or an empty registry
    /// if it does not exist yet (a content tree with nothing imported or
    /// saved into it has no registry file).
    pub fn load(project_root: &Path) -> anyhow::Result<Self> {
        let path = project_root.join(REGISTRY_FILE_NAME);
        match std::fs::read_to_string(&path) {
            Ok(text) => Self::parse(&text),
            Err(err) if err.kind() == std::io::ErrorKind::NotFound => Ok(Self::default()),
            Err(err) => Err(err).with_context(|| format!("failed to read '{}'", path.display())),
        }
    }

    pub fn parse(text: &str) -> anyhow::Result<Self> {
        let file: RegistryFile = toml::from_str(text).context("failed to parse asset registry")?;
        let mut entries = BTreeMap::new();
        for (hex, address) in file.assets {
            let id = AssetId::from_simple_hex(&hex)
                .map_err(|err| anyhow::anyhow!("invalid asset id '{hex}' in registry: {err}"))?;
            entries.insert(id, address);
        }
        Ok(Self { entries })
    }

    pub fn save(&self, project_root: &Path) -> anyhow::Result<()> {
        let path = project_root.join(REGISTRY_FILE_NAME);
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)
                .with_context(|| format!("failed to create '{}'", parent.display()))?;
        }
        let assets = self
            .entries
            .iter()
            .map(|(id, address)| (id.simple_hex(), address.clone()))
            .collect();
        let text = toml::to_string_pretty(&RegistryFile { assets })
            .context("failed to serialize asset registry")?;
        std::fs::write(&path, text).with_context(|| format!("failed to write '{}'", path.display()))
    }

    pub fn get(&self, id: AssetId) -> Option<&str> {
        self.entries.get(&id).map(String::as_str)
    }

    pub fn insert(&mut self, id: AssetId, address: impl Into<String>) {
        self.entries.insert(id, address.into());
    }

    pub fn remove(&mut self, id: AssetId) -> Option<String> {
        self.entries.remove(&id)
    }

    pub fn iter(&self) -> impl Iterator<Item = (AssetId, &str)> {
        self.entries.iter().map(|(id, address)| (*id, address.as_str()))
    }
}
```

- [ ] **Step 4: Patch `crates/import/src/lib.rs`'s existing header literal**

Find the `ContentAssetHeader { ... }` construction inside `import_source` and add the new field:

```rust
        let header = ContentAssetHeader {
            format_version: CONTENT_FORMAT_VERSION,
            asset_id: sub_asset.asset_id,
            references: sub_asset.references.clone(),
            kind: sub_asset.type_name.to_string(),
            provenance: None,
        };
```

(Task 4 replaces this whole function; this step exists only to keep the workspace compiling until then.)

- [ ] **Step 5: Patch the remaining `ContentAssetHeader` construction sites**

Add `provenance: None,` as a new line immediately after the `kind: "...".to_string(),` (or equivalent) field in each of these:

- `crates/essential/tests/content_first_loading.rs` — its four `ContentAssetHeader { ... }` literals (`prefers_a_content_asset_over_the_cooked_file`, `a_kind_mismatch_is_an_error`, `a_hash_fragment_address_never_probes_for_a_content_asset`, `a_real_loader_reads_a_content_asset_and_falls_back`). Task 3 rewrites this file completely; this step only keeps it compiling in the interim.
- `crates/essential/tests/content_asset_format.rs` — the single literal inside its `header()` helper (shared by all five tests in the file). No later task touches this file again; this one-line addition is its final form.
- `crates/render/tests/texture_pipeline_e2e.rs` — the single literal (written fresh by Task 1). Task 3 renames this file's `load_asset_bytes`/`CookedAssetRoot` calls to their final names; this step only adds the field.

- [ ] **Step 6: Create `crates/essential/tests/asset_registry.rs`**

```rust
//! `AssetRegistry` load/save/get/insert/remove/iter, and `save_content_asset`
//! upserting it.
use essential::assets::content::{read_content_asset, save_content_asset, AssetRegistry};
use essential::assets::{Asset, AssetId};
use serde::{Deserialize, Serialize};

fn temp_root(tag: &str) -> std::path::PathBuf {
    let dir = std::env::temp_dir().join(format!("asset-registry-{tag}-{}", std::process::id()));
    std::fs::create_dir_all(&dir).unwrap();
    dir
}

#[test]
fn insert_get_remove_round_trip_in_memory() {
    let mut registry = AssetRegistry::new();
    let id = AssetId::from_path("content/hero/scene.gasset");
    assert_eq!(registry.get(id), None);

    registry.insert(id, "content/hero/scene.gasset");
    assert_eq!(registry.get(id), Some("content/hero/scene.gasset"));

    assert_eq!(
        registry.remove(id),
        Some("content/hero/scene.gasset".to_string())
    );
    assert_eq!(registry.get(id), None);
}

#[test]
fn save_then_load_round_trips_every_entry() {
    let dir = temp_root("save-load");
    let mut registry = AssetRegistry::new();
    let a = AssetId::from_path("content/a.gasset");
    let b = AssetId::from_path("content/b.gasset");
    registry.insert(a, "content/a.gasset");
    registry.insert(b, "content/b.gasset");
    registry.save(&dir).expect("save");

    let loaded = AssetRegistry::load(&dir).expect("load");
    assert_eq!(loaded.get(a), Some("content/a.gasset"));
    assert_eq!(loaded.get(b), Some("content/b.gasset"));
    assert_eq!(loaded.iter().count(), 2);

    std::fs::remove_dir_all(&dir).ok();
}

#[test]
fn loading_a_registry_that_was_never_saved_is_empty_not_an_error() {
    let dir = temp_root("never-saved");
    let registry = AssetRegistry::load(&dir).expect("an absent registry file is not an error");
    assert_eq!(registry.iter().count(), 0);
    std::fs::remove_dir_all(&dir).ok();
}

#[test]
fn registry_file_lands_under_content_and_is_keyed_by_simple_hex() {
    let dir = temp_root("on-disk-shape");
    let mut registry = AssetRegistry::new();
    let id = AssetId::from_path("content/hero/scene.gasset");
    registry.insert(id, "content/hero/scene.gasset");
    registry.save(&dir).expect("save");

    let text = std::fs::read_to_string(dir.join("content/.registry.toml")).expect("file exists");
    assert!(
        text.contains(&id.simple_hex()),
        "the registry must key entries by simple_hex(), got: {text}"
    );

    std::fs::remove_dir_all(&dir).ok();
}

#[derive(Debug, Serialize, Deserialize)]
struct Thing;

impl Asset for Thing {
    fn name() -> &'static str {
        "Thing"
    }
}

#[test]
fn save_content_asset_upserts_the_registry() {
    let dir = temp_root("save-upserts");
    let address = "content/things/one.gasset";

    save_content_asset(&Thing, &dir, address).expect("save");

    let registry = AssetRegistry::load(&dir).expect("load");
    assert_eq!(
        registry.get(AssetId::from_path(address)),
        Some(address),
        "save_content_asset must register the asset it just wrote"
    );

    let raw = std::fs::read(dir.join(address)).unwrap();
    let (header, _) = read_content_asset(&raw).expect("readable");
    assert_eq!(
        header.provenance, None,
        "an editor save is not import-derived"
    );

    std::fs::remove_dir_all(&dir).ok();
}
```

- [ ] **Step 7: Build and test**

```bash
cargo build --workspace
cargo test --workspace
cargo fmt --all -- --check
cargo clippy -- -A clippy::type_complexity -A clippy::too_many_arguments -D warnings
```

- [ ] **Step 8: Commit**

```bash
git add -A
git commit -m "feat(essential): add AssetRegistry + ContentAssetHeader.provenance

CONTENT_FORMAT_VERSION 1 -> 2. AssetRegistry backs the runtime's future
path-less (load_by_id) resolution; save_content_asset now upserts it.

Co-Authored-By: Claude Sonnet 5 <noreply@anthropic.com>"
```

---

## Task 3: Essential cutover — delete the fallback, rename `CookedAssetRoot`, wire the registry into `request_load`

**Files:**
- Modify: `crates/essential/src/assets/mod.rs` (`CookedAssetRoot` → `ContentAssetRoot`, drop `res/` defaults; `AssetPath::new`/`address()` drop the `res/` prefix)
- Modify: `crates/essential/src/assets/utils.rs` (delete `load_cooked_asset_bytes`; rename+trim `load_asset_bytes` → `load_content_asset_bytes`; add `load_registry`)
- Modify: `crates/essential/src/assets/asset_server.rs` (rename cooked_root → content_root throughout; add registry caching + `resolve_by_id`; wire it into `request_load`)
- Modify: `crates/render/src/loaders/texture_loader.rs`, `crates/mesh/src/mesh.rs`, `crates/render/src/assets/material.rs`, `crates/scene/src/scene.rs`, `crates/mesh/src/skeleton.rs`, `crates/animation/src/clip.rs` (call the renamed helper, drop the `id` argument)
- Modify: `crates/render/tests/texture_pipeline_e2e.rs` (rename its `load_asset_bytes`/`CookedAssetRoot` calls to `load_content_asset_bytes`/`ContentAssetRoot`, dropping the `id` argument)
- Delete: `crates/essential/tests/cooked_asset_root.rs`
- Rewrite: `crates/essential/tests/asset_path_address.rs`, `crates/essential/tests/content_first_loading.rs`
- Create: `crates/essential/tests/registry_loading.rs`

**Interfaces:**
- Produces: `essential::assets::{ContentAssetRoot, AssetPath}` (address() with no prefix logic); `essential::assets::utils::{load_content_asset_bytes(root: &ContentAssetRoot, address: &str, expected_kind: &str) -> anyhow::Result<Vec<u8>>, load_registry(root: &ContentAssetRoot) -> anyhow::Result<AssetRegistry>}`; `AssetServer::{content_root(), set_content_root()}`; `AssetLoadContext::content_root()`.
- Consumes: `essential::assets::content::{AssetRegistry, REGISTRY_FILE_NAME}` from Task 2.
- This task is the reason examples 5-7 are temporarily broken at runtime: every `"<source>#<sub>"` address string still in example source now resolves nowhere (no fallback exists). That's expected — Tasks 5-7 fix it per-example.
- `crates/render/tests/texture_pipeline_e2e.rs` was written fresh by Task 1 using the pre-rename names (`load_asset_bytes`, `CookedAssetRoot`, plus an `id` argument) and had `provenance: None,` added to its header literal by Task 2 — both are still true of the version this task starts from; only the rename in the new step below changes.

- [ ] **Step 1: `ContentAssetRoot` + `AssetPath` in `mod.rs`**

Replace the `AssetPath` impl block (drop the forced `"res/"` prefix and the strip-back-off in `address()`):

```rust
impl<'a> AssetPath<'a> {
    pub fn new(path: impl AsRef<str>) -> Self {
        let mut normalized = path.as_ref().replace('\\', "/");

        // Remove any leading ./ or .\
        if normalized.starts_with("./") {
            normalized.drain(..2);
        }

        AssetPath {
            normalized_path: Cow::Owned(Path::new(&normalized).to_owned()),
        }
    }

    pub fn to_path(&self) -> &Path {
        &self.normalized_path
    }

    /// Recovers the logical asset address this path was constructed from —
    /// the string a caller wrote in a `load()` call or that `import`'s
    /// content-address convention produced (e.g.
    /// `"content/hero/scene.gasset"`) — as an owned `String`.
    ///
    /// This is what `AssetId::from_path` must be given so that a runtime
    /// `load()` call and import-time ID computation (which both derive an
    /// `AssetId` from the same address string) agree on the same `AssetId`
    /// for the same asset.
    pub fn address(&self) -> String {
        self.normalized_path.to_string_lossy().into_owned()
    }

    pub fn into_owned(self) -> AssetPath<'static> {
        AssetPath {
            normalized_path: Cow::Owned(self.normalized_path.into_owned()),
        }
    }

    #[cfg(target_arch = "wasm32")]
    pub fn to_string(&self) -> &str {
        self.normalized_path.to_str().unwrap()
    }
}
```

Replace `CookedAssetRoot` with `ContentAssetRoot`:

```rust
/// Where the runtime finds content asset files. Only the root differs per
/// platform; every address is a full path relative to it (e.g.
/// `"content/hero/scene.gasset"` — the `content/` segment is part of the
/// address, not injected by the root).
#[derive(Debug, Clone)]
pub enum ContentAssetRoot {
    /// Native: the directory containing the executable.
    Directory(PathBuf),
    /// wasm: the page origin, e.g. `"http://host"`.
    UrlBase(String),
}

impl ContentAssetRoot {
    /// Native: `<directory containing the executable>`, matching what each
    /// example's `build.rs` copies its `content/` tree next to. wasm:
    /// `<page origin>`, matching Trunk's `copy-dir` target.
    pub fn default_for_platform() -> Self {
        cfg_if::cfg_if! {
            if #[cfg(target_arch = "wasm32")] {
                let origin = web_sys::window()
                    .and_then(|window| window.location().origin().ok())
                    .unwrap_or_default();
                ContentAssetRoot::UrlBase(origin)
            } else {
                let exe_dir = std::env::current_exe()
                    .ok()
                    .and_then(|exe| exe.parent().map(Path::to_path_buf))
                    .unwrap_or_else(|| PathBuf::from("."));
                ContentAssetRoot::Directory(exe_dir)
            }
        }
    }
}

impl Default for ContentAssetRoot {
    fn default() -> Self {
        Self::default_for_platform()
    }
}
```

Every other item in `mod.rs` (`AssetId` and its impl from Task 2, `Asset`, `LoadableAsset`, the `From` impls for `AssetPath`) is unchanged.

- [ ] **Step 2: Rewrite `crates/essential/src/assets/utils.rs`**

```rust
use anyhow::Context;
use cfg_if::cfg_if;

use super::content::AssetRegistry;
use super::ContentAssetRoot;

/// Reads an asset's payload from the content asset at `<root>/<address>`.
pub async fn load_content_asset_bytes(
    root: &ContentAssetRoot,
    address: &str,
    expected_kind: &str,
) -> anyhow::Result<Vec<u8>> {
    let raw = try_read_relative(root, address)
        .await?
        .with_context(|| format!("no content asset at '{address}'"))?;

    let (header, payload) = crate::assets::content::read_content_asset(&raw)
        .with_context(|| format!("malformed content asset '{address}'"))?;
    if header.kind != expected_kind {
        anyhow::bail!(
            "content asset '{address}' holds a {} but a {expected_kind} was requested",
            header.kind
        );
    }
    Ok(payload.to_vec())
}

/// Reads and parses `.registry.toml` from `root`, for `AssetServer`'s
/// path-less (`load_by_id`) resolution. An absent registry (nothing has
/// ever been imported or saved into this root) is an empty registry, not
/// an error.
pub async fn load_registry(root: &ContentAssetRoot) -> anyhow::Result<AssetRegistry> {
    match try_read_relative(root, crate::assets::content::REGISTRY_FILE_NAME).await? {
        Some(bytes) => {
            let text =
                String::from_utf8(bytes).context("asset registry file is not valid UTF-8")?;
            AssetRegistry::parse(&text)
        }
        None => Ok(AssetRegistry::default()),
    }
}

/// Reads `<root>/<relative>`, returning `Ok(None)` when it simply is not
/// there (a missing file natively, a non-success status on wasm) so the
/// caller can turn absence into its own error message. Any other failure
/// is a real error.
async fn try_read_relative(
    root: &ContentAssetRoot,
    relative: &str,
) -> anyhow::Result<Option<Vec<u8>>> {
    match root {
        ContentAssetRoot::Directory(dir) => {
            let path = dir.join(relative);
            cfg_if! {
                if #[cfg(target_arch = "wasm32")] {
                    let _ = path;
                    Ok(None)
                } else {
                    match async_fs::read(&path).await {
                        Ok(bytes) => Ok(Some(bytes)),
                        Err(err)
                            if matches!(
                                err.kind(),
                                std::io::ErrorKind::NotFound
                                    | std::io::ErrorKind::IsADirectory
                                    | std::io::ErrorKind::NotADirectory
                                    | std::io::ErrorKind::InvalidInput
                            ) =>
                        {
                            Ok(None)
                        }
                        Err(err) => Err(anyhow::Error::new(err))
                            .with_context(|| format!("failed to read '{}'", path.display())),
                    }
                }
            }
        }
        ContentAssetRoot::UrlBase(base) => {
            cfg_if! {
                if #[cfg(target_arch = "wasm32")] {
                    let url = format!("{base}/{relative}");
                    let response = reqwest::get(&url)
                        .await
                        .with_context(|| format!("HTTP request for '{url}' failed"))?;
                    if !response.status().is_success() {
                        return Ok(None);
                    }
                    let bytes = response
                        .bytes()
                        .await
                        .with_context(|| format!("failed to read response body for '{url}'"))?;
                    Ok(Some(bytes.to_vec()))
                } else {
                    let _ = relative;
                    anyhow::bail!(
                        "ContentAssetRoot::UrlBase is only supported on wasm32 (base '{base}')"
                    )
                }
            }
        }
    }
}
```

- [ ] **Step 3: Rewrite `crates/essential/src/assets/asset_server.rs`**

```rust
use std::{
    any::TypeId,
    collections::{HashMap, HashSet},
    sync::{Arc, RwLock, Weak},
};

use crossbeam_channel::{Receiver, Sender};
use ecs::{resource::Resource, world};
use tasks::load_pool::LoadTaskPool;

use crate::{
    assets::{handle::StrongAssetHandle, AssetPath, LoadableAsset},
    tasks::{task_pool::TaskPool, Task},
};

use super::{
    asset_container::AssetContainer,
    asset_store::AssetStore,
    content::AssetRegistry,
    handle::{AssetHandle, AssetLifetimeEvent},
    Asset, AssetId, ContentAssetRoot,
};

struct LoadedAsset {
    pub(crate) id: AssetId,
    pub(crate) value: Box<dyn AssetContainer>,
}

impl LoadedAsset {
    pub fn new<A: Asset + 'static>(id: AssetId, value: A) -> Self {
        LoadedAsset {
            id,
            value: Box::new(value),
        }
    }
}

enum AssetLoadEvent {
    Loaded(LoadedAsset),
    LoadFailed(AssetId),
}

pub struct AssetLoadContext {
    asset_server: AssetServer,
    asset_id: AssetId,
    content_root: ContentAssetRoot,
}

impl AssetLoadContext {
    pub fn asset_server(&self) -> &AssetServer {
        &self.asset_server
    }

    pub fn asset_id(&self) -> AssetId {
        self.asset_id
    }

    pub fn content_root(&self) -> &ContentAssetRoot {
        &self.content_root
    }
}

impl AssetLoadContext {
    pub(crate) fn new(
        asset_server: AssetServer,
        asset_id: AssetId,
        content_root: ContentAssetRoot,
    ) -> Self {
        Self {
            asset_server,
            asset_id,
            content_root,
        }
    }
}

pub(crate) struct AssetInfo {
    // std::sync::Weak, used only for handle-reuse/dedup in AssetHandleProvider
    // (doesn't keep the asset alive) — unrelated to the AssetHandle::Weak
    // enum variant (an unresolved AssetId reference).
    handle: Weak<StrongAssetHandle>,
}

pub(crate) struct AssetServerData {
    pending_tasks: RwLock<HashMap<AssetId, Task<()>>>,
    loaded_assets: RwLock<HashSet<AssetId>>,
    path_to_id: RwLock<HashMap<AssetPath<'static>, AssetId>>,
    handle_provider: AssetHandleProvider,
    asset_load_event_sender: Sender<AssetLoadEvent>,
    asset_load_event_receiver: Receiver<AssetLoadEvent>,
    content_root: RwLock<ContentAssetRoot>,
    // Cached after the first path-less (`load_by_id`) resolution; see
    // `AssetServer::resolve_by_id`.
    registry: RwLock<Option<Arc<AssetRegistry>>>,
}

#[derive(Resource, Clone)]
pub struct AssetServer {
    data: Arc<AssetServerData>,
}

impl AssetServer {
    pub fn new() -> Self {
        let (asset_load_event_sender, asset_load_event_receiver) = crossbeam_channel::unbounded();
        let server_data = AssetServerData {
            pending_tasks: RwLock::new(HashMap::new()),
            loaded_assets: RwLock::new(HashSet::new()),
            path_to_id: RwLock::new(HashMap::new()),
            handle_provider: AssetHandleProvider::new(),
            asset_load_event_sender,
            asset_load_event_receiver,
            content_root: RwLock::new(ContentAssetRoot::default_for_platform()),
            registry: RwLock::new(None),
        };

        Self {
            data: Arc::new(server_data),
        }
    }

    /// The root every content-asset loader resolves its address against.
    /// Defaults to [`ContentAssetRoot::default_for_platform`]; override with
    /// [`AssetServer::set_content_root`] before triggering loads.
    pub fn content_root(&self) -> ContentAssetRoot {
        self.data.content_root.read().unwrap().clone()
    }

    pub fn set_content_root(&self, root: ContentAssetRoot) {
        *self.data.content_root.write().unwrap() = root;
    }

    pub fn register_asset<A: Asset>(&mut self, asset: &AssetStore<A>) {
        self.data
            .handle_provider
            .register_asset::<A>(asset.clone_drop_sender());
    }

    pub fn load<'a, A>(&self, path: impl Into<AssetPath<'a>>) -> AssetHandle<A>
    where
        A: LoadableAsset + 'static,
    {
        self.load_internal::<A>(path, A::default_usage_settings())
    }

    pub fn add<A: Asset>(&self, asset: A) -> AssetHandle<A> {
        let id = AssetId::new();

        let sender = self.data.asset_load_event_sender.clone();
        let _ = sender.send(AssetLoadEvent::Loaded(LoadedAsset::new(id, asset)));
        self.data.handle_provider.request_handle(id, None)
    }

    pub fn load_with_usage_settings<'a, A>(
        &self,
        path: impl Into<AssetPath<'a>>,
        usage_settings: A::UsageSettings,
    ) -> AssetHandle<A>
    where
        A: LoadableAsset + 'static,
    {
        self.load_internal::<A>(path, usage_settings)
    }

    /// Loads (or returns a handle to an already-loading/loaded asset for) the
    /// given `AssetId` directly, with no `AssetPath` involved. Used by
    /// callers (e.g. importers building references) that only have an
    /// `AssetId` and no human-readable path; `request_load` resolves it to
    /// an address via the asset registry.
    ///
    /// This is the same per-ID dedup and request-load logic `load_internal`
    /// has always used, extracted so `load_internal` and `load_by_id` share
    /// it: if an asset for `id` isn't already loaded or loading, a load task
    /// is spawned via `request_load`; either way, a handle to `id` is
    /// returned (deduped/reused via `AssetHandleProvider.asset_handles`).
    pub fn load_by_id<A: LoadableAsset + 'static>(&self, id: AssetId) -> AssetHandle<A> {
        if !self.data.pending_tasks.read().unwrap().contains_key(&id)
            && !self.data.loaded_assets.read().unwrap().contains(&id)
        {
            self.request_load::<A>(None, id, A::default_usage_settings());
        }

        self.data.handle_provider.request_handle(id, None)
    }

    fn load_internal<'a, A: LoadableAsset>(
        &self,
        path: impl Into<AssetPath<'a>>,
        usage_settings: A::UsageSettings,
    ) -> AssetHandle<A> {
        let path = path.into().into_owned();

        let id = match self.data.path_to_id.write().unwrap().entry(path.clone()) {
            std::collections::hash_map::Entry::Occupied(occupied_entry) => *occupied_entry.get(),
            std::collections::hash_map::Entry::Vacant(vacant_entry) => {
                *vacant_entry.insert(AssetId::from_path(&path.address()))
            }
        };

        if !self.data.pending_tasks.read().unwrap().contains_key(&id)
            && !self.data.loaded_assets.read().unwrap().contains(&id)
        {
            self.request_load::<A>(Some(path.clone()), id, usage_settings);
        }

        self.data.handle_provider.request_handle(id, Some(path))
    }

    pub fn process_handle_drop(&mut self, id: &AssetId, path: Option<AssetPath<'static>>) {
        self.data.loaded_assets.write().unwrap().remove(id);

        if let Some(path) = path {
            self.data.path_to_id.write().unwrap().remove(&path);
        }
    }

    /// Resolves `id` to its content-tree address via the asset registry, for
    /// a path-less (`load_by_id`) load. Loads and caches the registry from
    /// `content_root()` on first use; concurrent first-use callers may each
    /// load it once — a benign race, not a correctness issue, since the
    /// registry is read-only from the runtime's perspective.
    async fn resolve_by_id(&self, id: AssetId) -> Option<String> {
        let cached = self.data.registry.read().unwrap().clone();
        if let Some(registry) = cached {
            return registry.get(id).map(str::to_owned);
        }

        let root = self.content_root();
        let registry = match crate::assets::utils::load_registry(&root).await {
            Ok(registry) => Arc::new(registry),
            Err(error) => {
                log::error!("failed to load asset registry: {error:#}");
                return None;
            }
        };
        let address = registry.get(id).map(str::to_owned);
        *self.data.registry.write().unwrap() = Some(registry);
        address
    }

    /// Spawns the async load task for `id`. `path` is the human-readable
    /// asset path used both to locate/parse the file and for error logging;
    /// `load_by_id` callers have no path (they only have an `AssetId`), so
    /// they pass `None` and this resolves one via the asset registry before
    /// the loader ever runs — a registry miss fails the load outright rather
    /// than calling the loader with a placeholder path.
    fn request_load<A: LoadableAsset>(
        &self,
        path: Option<AssetPath<'static>>,
        id: AssetId,
        usage_settings: A::UsageSettings,
    ) {
        let asset_loader = A::loader();

        let sender = self.data.asset_load_event_sender.clone();

        let server = self.clone();
        // No profiling scope around the async body: a scope guard must not be
        // held across .await (tasks can migrate between worker threads).
        // Load costs show up on the named "asset-load-N" threads instead.
        let task =
            LoadTaskPool::get_or_init(|| TaskPool::with_name("asset-load")).spawn(async move {
                let path = match path {
                    Some(path) => path,
                    None => match server.resolve_by_id(id).await {
                        Some(address) => AssetPath::new(address),
                        None => {
                            log::error!(
                                "no content asset registered for AssetId {id:?} (type {})",
                                std::any::type_name::<A>()
                            );
                            sender.send(AssetLoadEvent::LoadFailed(id)).unwrap();
                            return;
                        }
                    },
                };
                let log_path = path.clone();
                let content_root = server.content_root();
                let asset = asset_loader
                    .load(
                        path,
                        &mut AssetLoadContext::new(server, id, content_root),
                        usage_settings,
                    )
                    .await;
                match asset {
                    Ok(asset) => {
                        sender
                            .send(AssetLoadEvent::Loaded(LoadedAsset::new(id, asset)))
                            .unwrap();
                    }
                    Err(error) => {
                        log::error!(
                            "Failed to load asset '{}' (type {}): {:#}",
                            log_path.to_path().display(),
                            std::any::type_name::<A>(),
                            error
                        );
                        sender.send(AssetLoadEvent::LoadFailed(id)).unwrap();
                    }
                }
            });

        self.data.pending_tasks.write().unwrap().insert(id, task);
    }
}

impl Default for AssetServer {
    fn default() -> Self {
        Self::new()
    }
}

// TODO: This shouldn't need to be public
pub fn handle_asset_load_events(world: &mut world::World) {
    let server = world.remove_resource::<AssetServer>().unwrap();

    server
        .data
        .asset_load_event_receiver
        .try_iter()
        .for_each(|event| match event {
            AssetLoadEvent::Loaded(loaded_asset) => {
                server
                    .data
                    .pending_tasks
                    .write()
                    .unwrap()
                    .remove(&loaded_asset.id);
                server
                    .data
                    .loaded_assets
                    .write()
                    .unwrap()
                    .insert(loaded_asset.id);
                loaded_asset.value.insert(loaded_asset.id, world);
            }
            AssetLoadEvent::LoadFailed(id) => {
                server.data.pending_tasks.write().unwrap().remove(&id);
                server.data.loaded_assets.write().unwrap().remove(&id);
            }
        });
    world.insert_resource(server);
}

struct AssetHandleProvider {
    asset_handles: RwLock<HashMap<AssetId, AssetInfo>>,
    asset_lifetime_send_map: RwLock<HashMap<TypeId, Sender<AssetLifetimeEvent>>>,
}

impl AssetHandleProvider {
    pub fn new() -> Self {
        Self {
            asset_handles: RwLock::new(HashMap::new()),
            asset_lifetime_send_map: RwLock::new(HashMap::new()),
        }
    }

    pub fn register_asset<A: Asset>(&self, lifetime_sender: Sender<AssetLifetimeEvent>) {
        let type_id = TypeId::of::<A>();
        self.asset_lifetime_send_map
            .write()
            .unwrap()
            .insert(type_id, lifetime_sender);
    }

    pub fn request_handle<A: Asset>(
        &self,
        id: AssetId,
        path: Option<AssetPath<'static>>,
    ) -> AssetHandle<A> {
        let lifetime_sender = self
            .asset_lifetime_send_map
            .read()
            .unwrap()
            .get(&TypeId::of::<A>())
            .expect("Asset lifetime sender not found, make sure to register it")
            .clone();

        let mut binding = self.asset_handles.write().unwrap();

        let info = binding.entry(id).or_insert_with(|| AssetInfo {
            handle: Weak::new(),
        });

        if let Some(strong_handle) = info.handle.upgrade() {
            AssetHandle::strong(strong_handle)
        } else {
            let handle = Arc::new(StrongAssetHandle {
                id,
                lifetime_sender,
                path,
            });

            info.handle = Arc::downgrade(&handle);

            AssetHandle::strong(handle)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::assets::content::AssetRegistry;

    fn temp_root(tag: &str) -> std::path::PathBuf {
        let dir = std::env::temp_dir().join(format!("asset-server-{tag}-{}", std::process::id()));
        std::fs::create_dir_all(dir.join("content")).unwrap();
        dir
    }

    #[test]
    fn resolve_by_id_finds_a_registered_asset() {
        let dir = temp_root("resolve-hit");
        let id = AssetId::from_path("content/hero/scene.gasset");
        let mut registry = AssetRegistry::new();
        registry.insert(id, "content/hero/scene.gasset");
        registry.save(&dir).expect("save registry");

        let server = AssetServer::new();
        server.set_content_root(ContentAssetRoot::Directory(dir.clone()));
        let resolved = pollster::block_on(server.resolve_by_id(id));

        assert_eq!(resolved.as_deref(), Some("content/hero/scene.gasset"));
        std::fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn resolve_by_id_returns_none_for_an_unregistered_id() {
        let dir = temp_root("resolve-miss");
        let server = AssetServer::new();
        server.set_content_root(ContentAssetRoot::Directory(dir.clone()));
        let resolved = pollster::block_on(server.resolve_by_id(AssetId::new()));
        assert_eq!(resolved, None);
        std::fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn resolve_by_id_caches_the_registry_after_first_load() {
        let dir = temp_root("resolve-cache");
        let id = AssetId::from_path("content/hero/scene.gasset");
        let mut registry = AssetRegistry::new();
        registry.insert(id, "content/hero/scene.gasset");
        registry.save(&dir).expect("save registry");

        let server = AssetServer::new();
        server.set_content_root(ContentAssetRoot::Directory(dir.clone()));
        let first = pollster::block_on(server.resolve_by_id(id));
        assert_eq!(first.as_deref(), Some("content/hero/scene.gasset"));

        // Removing the on-disk registry must not affect a cached lookup.
        std::fs::remove_file(dir.join("content/.registry.toml")).unwrap();
        let second = pollster::block_on(server.resolve_by_id(id));
        assert_eq!(
            second.as_deref(),
            Some("content/hero/scene.gasset"),
            "the registry is cached after first use, so a since-deleted file must not affect the second lookup"
        );

        std::fs::remove_dir_all(&dir).ok();
    }
}
```

- [ ] **Step 4: Update all six loaders**

In each of `crates/render/src/loaders/texture_loader.rs`, `crates/mesh/src/mesh.rs`, `crates/render/src/assets/material.rs`, `crates/scene/src/scene.rs`, `crates/mesh/src/skeleton.rs`, `crates/animation/src/clip.rs`, change:

```rust
        let bytes = essential::assets::utils::load_asset_bytes(
            load_context.cooked_root(),
            &path.address(),
            load_context.asset_id(),
            <Type>::name(),
        )
```

to:

```rust
        let bytes = essential::assets::utils::load_content_asset_bytes(
            load_context.content_root(),
            &path.address(),
            <Type>::name(),
        )
```

where `<Type>` is `Texture`, `Mesh`, `StandardMaterial`, `Scene`, `Skeleton`, `AnimationClip` respectively. The rest of each `load` function (everything after the `.with_context(...)` line) is unchanged — this includes `material.rs`'s post-deserialize `resolve_asset_handles` call and `scene.rs`'s "each component upgrades its own Weak handle" comment.

- [ ] **Step 5: Rename the same call in `crates/render/tests/texture_pipeline_e2e.rs`**

This file (written by Task 1, given `provenance: None,` by Task 2) still calls the pre-rename API. Change:

```rust
use essential::assets::utils::load_asset_bytes;
use essential::assets::{AssetId, CookedAssetRoot};
```

to:

```rust
use essential::assets::utils::load_content_asset_bytes;
use essential::assets::ContentAssetRoot;
```

(`AssetId` is no longer used by this file once the `id` argument below is dropped — remove its import rather than leave it unused) and change the call itself:

```rust
    let bytes = pollster::block_on(load_asset_bytes(
        &CookedAssetRoot::Directory(temp_dir.clone()),
        address,
        AssetId::from_path(address),
        Texture::name(),
    ))
```

to:

```rust
    let bytes = pollster::block_on(load_content_asset_bytes(
        &ContentAssetRoot::Directory(temp_dir.clone()),
        address,
        Texture::name(),
    ))
```

Nothing else in the file changes.

- [ ] **Step 6: Delete `crates/essential/tests/cooked_asset_root.rs`**

```bash
git rm crates/essential/tests/cooked_asset_root.rs
```

- [ ] **Step 7: Rewrite `crates/essential/tests/asset_path_address.rs`**

```rust
//! `AssetPath::address()` recovers the exact address a caller passed to
//! `load()` (or that `import`'s content-address convention produced), so
//! `AssetId::from_path(path.address())` — what `AssetServer::load_internal`
//! computes — agrees with an `AssetId::from_path` computed directly from the
//! same string with no `AssetPath` involved (e.g. `import`'s
//! `ImportContext::sub_asset_id`, or a hand-written test id).
use essential::assets::{AssetId, AssetPath};

#[test]
fn address_returns_the_normalized_path_unchanged() {
    let path = AssetPath::new("content/hero/scene.gasset");
    assert_eq!(path.address(), "content/hero/scene.gasset");
}

#[test]
fn address_normalizes_backslashes_and_a_leading_dot_slash() {
    let path = AssetPath::new("./content\\hero\\scene.gasset");
    assert_eq!(path.address(), "content/hero/scene.gasset");
}

#[test]
fn address_agrees_with_import_time_id_computation() {
    let raw_address = "content/hero/scene.gasset";

    let runtime_id = AssetId::from_path(&AssetPath::new(raw_address).address());
    let import_time_id = AssetId::from_path(raw_address);

    assert_eq!(
        runtime_id, import_time_id,
        "a runtime load() call and import-time ID computation must agree on the AssetId for the same asset address"
    );
}
```

- [ ] **Step 8: Rewrite `crates/essential/tests/content_first_loading.rs`**

```rust
//! load_content_asset_bytes reads a content asset at <root>/<address>.
use essential::assets::content::{write_content_asset, ContentAssetHeader, CONTENT_FORMAT_VERSION};
use essential::assets::utils::load_content_asset_bytes;
use essential::assets::{AssetId, ContentAssetRoot};
use serde::{Deserialize, Serialize};

fn temp_root(tag: &str) -> std::path::PathBuf {
    let dir = std::env::temp_dir().join(format!("content-first-{tag}-{}", std::process::id()));
    std::fs::create_dir_all(&dir).unwrap();
    dir
}

#[test]
fn reads_a_content_asset_at_its_literal_address() {
    let dir = temp_root("reads");
    let address = "content/x/mesh_0.gasset";
    let id = AssetId::from_path(address);

    let header = ContentAssetHeader {
        format_version: CONTENT_FORMAT_VERSION,
        asset_id: id,
        references: Vec::new(),
        kind: "Mesh".to_string(),
        provenance: None,
    };
    std::fs::create_dir_all(dir.join("content/x")).unwrap();
    std::fs::write(
        dir.join(address),
        write_content_asset(&header, b"payload").unwrap(),
    )
    .unwrap();

    let bytes = pollster::block_on(load_content_asset_bytes(
        &ContentAssetRoot::Directory(dir.clone()),
        address,
        "Mesh",
    ))
    .expect("load");
    assert_eq!(bytes, b"payload", "the payload comes back header-stripped");

    std::fs::remove_dir_all(&dir).ok();
}

#[test]
fn a_kind_mismatch_is_an_error() {
    let dir = temp_root("kind");
    let address = "content/x/thing.gasset";
    let header = ContentAssetHeader {
        format_version: CONTENT_FORMAT_VERSION,
        asset_id: AssetId::from_path(address),
        references: Vec::new(),
        kind: "Mesh".to_string(),
        provenance: None,
    };
    std::fs::create_dir_all(dir.join("content/x")).unwrap();
    std::fs::write(
        dir.join(address),
        write_content_asset(&header, b"payload").unwrap(),
    )
    .unwrap();

    let err = pollster::block_on(load_content_asset_bytes(
        &ContentAssetRoot::Directory(dir.clone()),
        address,
        "Scene",
    ))
    .expect_err("kind mismatch must be an error");
    let message = format!("{err:#}");
    assert!(
        message.contains("Mesh") && message.contains("Scene"),
        "got: {message}"
    );

    std::fs::remove_dir_all(&dir).ok();
}

#[test]
fn a_missing_content_asset_is_an_error() {
    let dir = temp_root("missing");

    let err = pollster::block_on(load_content_asset_bytes(
        &ContentAssetRoot::Directory(dir.clone()),
        "content/x/absent.gasset",
        "Mesh",
    ))
    .expect_err("a missing content asset must be an error, not empty bytes");
    assert!(
        format!("{err:#}").contains("content/x/absent.gasset"),
        "the error must name the address so a missing asset is traceable"
    );

    std::fs::remove_dir_all(&dir).ok();
}

#[test]
fn a_directory_at_the_address_is_an_error() {
    let dir = temp_root("addr-is-dir");
    let address = "content/x";
    std::fs::create_dir_all(dir.join(address)).unwrap();

    let err = pollster::block_on(load_content_asset_bytes(
        &ContentAssetRoot::Directory(dir.clone()),
        address,
        "Mesh",
    ))
    .expect_err("a directory at the joined path is not a readable content asset");
    assert!(
        format!("{err:#}").contains(address),
        "the error must name the address"
    );

    std::fs::remove_dir_all(&dir).ok();
}

#[derive(Debug, PartialEq, Serialize, Deserialize)]
struct StandInMesh {
    vertices: Vec<[f32; 3]>,
    indices: Vec<u32>,
}

#[test]
fn a_real_loader_reads_a_content_asset() {
    // `MeshLoader::load` is exactly `load_content_asset_bytes(.., "Mesh")`
    // then `bincode::deserialize`. `essential` cannot depend on `mesh` (that
    // is a dependency cycle) and `AssetLoadContext::new` is crate-private, so
    // this exercises the identical bytes-then-decode path with a stand-in
    // payload.
    let dir = temp_root("real-loader");
    let address = "content/x/m.gasset";
    let id = AssetId::from_path(address);
    let mesh = StandInMesh {
        vertices: vec![[0.0, 0.0, 0.0], [1.0, 0.0, 0.0], [0.0, 1.0, 0.0]],
        indices: vec![0, 1, 2],
    };

    let header = ContentAssetHeader {
        format_version: CONTENT_FORMAT_VERSION,
        asset_id: id,
        references: Vec::new(),
        kind: "Mesh".to_string(),
        provenance: None,
    };
    std::fs::create_dir_all(dir.join("content/x")).unwrap();
    let payload = bincode::serialize(&mesh).unwrap();
    std::fs::write(
        dir.join(address),
        write_content_asset(&header, &payload).unwrap(),
    )
    .unwrap();

    let bytes = pollster::block_on(load_content_asset_bytes(
        &ContentAssetRoot::Directory(dir.clone()),
        address,
        "Mesh",
    ))
    .expect("the content asset loads through the loader byte path");
    let decoded: StandInMesh = bincode::deserialize(&bytes).expect("payload is the mesh type");
    assert_eq!(
        decoded, mesh,
        "the loader path returns a decodable Mesh payload"
    );

    std::fs::remove_dir_all(&dir).ok();
}
```

- [ ] **Step 9: Create `crates/essential/tests/registry_loading.rs`**

```rust
//! `utils::load_registry` — the async, ContentAssetRoot-aware counterpart to
//! `AssetRegistry::load` that `AssetServer::resolve_by_id` uses at runtime.
use essential::assets::content::AssetRegistry;
use essential::assets::utils::load_registry;
use essential::assets::{AssetId, ContentAssetRoot};

#[test]
fn loads_a_registry_written_by_asset_registry_save() {
    let dir = std::env::temp_dir().join(format!("registry-loading-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();

    let id = AssetId::from_path("content/hero/scene.gasset");
    let mut registry = AssetRegistry::new();
    registry.insert(id, "content/hero/scene.gasset");
    registry.save(&dir).expect("save");

    let loaded = pollster::block_on(load_registry(&ContentAssetRoot::Directory(dir.clone())))
        .expect("load");
    assert_eq!(loaded.get(id), Some("content/hero/scene.gasset"));

    std::fs::remove_dir_all(&dir).ok();
}

#[test]
fn an_absent_registry_file_is_an_empty_registry() {
    let dir =
        std::env::temp_dir().join(format!("registry-loading-absent-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();

    let loaded = pollster::block_on(load_registry(&ContentAssetRoot::Directory(dir.clone())))
        .expect("an absent registry file is not an error");
    assert_eq!(loaded.iter().count(), 0);

    std::fs::remove_dir_all(&dir).ok();
}
```

- [ ] **Step 10: Build and test**

```bash
cargo build --workspace
cargo test --workspace
cargo fmt --all -- --check
cargo clippy -- -A clippy::type_complexity -A clippy::too_many_arguments -D warnings
```

Expected: all four green. The three examples still compile (their load-address string literals are unaffected by this task) but will fail at runtime — expected per Global Constraints.

- [ ] **Step 11: Commit**

```bash
git add -A
git commit -m "refactor!: rename CookedAssetRoot to ContentAssetRoot; delete the cooked-asset fallback

AssetPath no longer forces a res/ prefix. load_asset_bytes is now
load_content_asset_bytes with no id param and no fallback. AssetServer's
path-less (load_by_id) loads resolve through AssetRegistry instead.

Co-Authored-By: Claude Sonnet 5 <noreply@anthropic.com>"
```

---

## Task 4: `import_source` returns structured results, populates provenance, upserts the registry

**Files:**
- Modify: `crates/import/src/lib.rs`, `crates/import/src/main.rs`
- Modify: `crates/import/tests/import_gltf.rs`, `crates/import/tests/end_to_end.rs`

**Interfaces:**
- Produces: `import::ImportedAsset { sub_asset_name: String, address: String, kind: String }`; `import::import_source(source: &Path, project_root: &Path, config: &ContentConfig) -> anyhow::Result<Vec<ImportedAsset>>`.
- Consumes: `essential::assets::content::{AssetRegistry, ImportProvenance, ContentAssetHeader, CONTENT_FORMAT_VERSION}` (Task 2), `essential::assets::{ContentAssetRoot, utils::load_content_asset_bytes}` (Task 3), `asset_import::{ImportContext, Importer, SubAssetIdResolver}` (Task 1).

- [ ] **Step 1: Rewrite `crates/import/src/lib.rs`**

```rust
//! The content-asset import driver: runs the offline importers with a
//! content-path sub-asset-id resolver and writes one content asset per
//! emitted sub-asset. The binary (`src/main.rs`) is a thin CLI over
//! [`import_source`].
use std::collections::HashSet;
use std::path::Path;

use anyhow::{bail, Context};
use asset_import::{ImportContext, Importer, SubAssetIdResolver};
use essential::assets::content::{
    write_content_asset, AssetRegistry, ContentAssetHeader, ImportProvenance,
    CONTENT_FORMAT_VERSION,
};
use essential::assets::AssetId;

pub mod config;

use config::{content_address, ContentConfig};

fn registered_importers() -> Vec<Box<dyn Importer>> {
    vec![
        Box::new(render::importers::image_importer::ImageImporter),
        Box::new(gltf_loader::gltf_importer::GltfImporter),
        Box::new(obj_loader::obj_importer::ObjImporter),
    ]
}

/// One content asset `import_source` wrote, returned so an in-process caller
/// (a future editor) gets structured results instead of parsing addresses
/// back out of printed strings.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ImportedAsset {
    /// The sub-asset name within the source, e.g. `"mesh/0"`.
    pub sub_asset_name: String,
    /// The project-relative content-tree address it was written to.
    pub address: String,
    /// The asset type tag (`Asset::name()`), e.g. `"Mesh"`.
    pub kind: String,
}

/// Imports one source file into `project_root`, returning the content
/// assets written, and upserts `project_root`'s asset registry so each one
/// is reachable by `AssetServer::load_by_id`.
pub fn import_source(
    source: &Path,
    project_root: &Path,
    config: &ContentConfig,
) -> anyhow::Result<Vec<ImportedAsset>> {
    let extension = source
        .extension()
        .and_then(|e| e.to_str())
        .unwrap_or_default()
        .to_ascii_lowercase();
    let importers = registered_importers();
    let Some(importer) = importers
        .iter()
        .find(|i| i.supported_extensions().contains(&extension.as_str()))
    else {
        bail!(
            "no importer handles '.{extension}' (source '{}')",
            source.display()
        );
    };

    // Cross-references resolve to content-tree addresses. `content_address`
    // is a pure function of the sub-asset name, so one importer pass is
    // enough — no need to discover the sub-asset set first.
    let owned_source = source.to_path_buf();
    let owned_config = config.clone();
    let resolver: SubAssetIdResolver = Box::new(move |sub_name| {
        AssetId::from_path(&content_address(&owned_config, &owned_source, sub_name))
    });

    let mut ctx = ImportContext::with_sub_asset_id_resolver(source.to_path_buf(), resolver);
    importer
        .import(source, &mut ctx)
        .map_err(|err| anyhow::anyhow!("{err:?}"))
        .with_context(|| format!("importing '{}'", source.display()))?;
    let outputs = ctx.into_parts();

    let emitted: HashSet<AssetId> = outputs.sub_assets.iter().map(|s| s.asset_id).collect();
    let mut registry = AssetRegistry::load(project_root)?;
    let mut written = Vec::with_capacity(outputs.sub_assets.len());

    for sub_asset in &outputs.sub_assets {
        for reference in &sub_asset.references {
            // A reference outside this source's emitted set is a cross-source
            // link the content-path resolver never produced (e.g. an OBJ's
            // `<tex>#main` texture id). This phase has no cross-source import
            // story, so warn and keep going rather than aborting the import.
            if !emitted.contains(reference) {
                log::warn!(
                    "'{}' references {reference:?}, which this source does not emit; \
                     leaving it unresolved (cross-source imports land in a later phase)",
                    sub_asset.name
                );
            }
        }

        let address = content_address(config, source, &sub_asset.name);
        let kind = sub_asset.type_name.to_string();
        let header = ContentAssetHeader {
            format_version: CONTENT_FORMAT_VERSION,
            asset_id: sub_asset.asset_id,
            references: sub_asset.references.clone(),
            kind: kind.clone(),
            provenance: Some(ImportProvenance {
                source: source.display().to_string(),
                sub_asset: sub_asset.name.clone(),
            }),
        };
        let bytes = write_content_asset(&header, &sub_asset.bytes)?;

        let path = project_root.join(&address);
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)
                .with_context(|| format!("failed to create '{}'", parent.display()))?;
        }
        std::fs::write(&path, bytes)
            .with_context(|| format!("failed to write '{}'", path.display()))?;

        registry.insert(sub_asset.asset_id, address.clone());
        written.push(ImportedAsset {
            sub_asset_name: sub_asset.name.clone(),
            address,
            kind,
        });
    }

    registry.save(project_root)?;

    Ok(written)
}
```

- [ ] **Step 2: Update `crates/import/src/main.rs`'s print loop**

```rust
    let written = import_source(&source, &project_root, &config)?;
    println!("imported {} -> {} assets", source.display(), written.len());
    for asset in &written {
        println!("  {} -> {}", asset.sub_asset_name, asset.address);
    }
    Ok(())
```

(everything above this in `main.rs` is unchanged)

- [ ] **Step 3: Update `crates/import/tests/import_gltf.rs`**

Change the top-of-file `use` for `content` items to:

```rust
use essential::assets::content::{read_content_asset, AssetRegistry, ImportProvenance};
```

Replace the test body's address-membership checks and add a provenance assertion plus a new registry test:

```rust
#[test]
fn writes_content_assets_with_content_path_cross_references() {
    let project_root = std::env::temp_dir().join(format!("import-gltf-{}", std::process::id()));
    std::fs::create_dir_all(&project_root).unwrap();

    let written = import::import_source(&fixture(), &project_root, &Default::default())
        .expect("import succeeds");

    for expected in [
        "content/triangle/mesh_0.gasset",
        "content/triangle/scene.gasset",
    ] {
        assert!(
            written.iter().any(|a| a.address == expected),
            "expected {expected} among {written:?}"
        );
        assert!(project_root.join(expected).exists(), "{expected} on disk");
    }

    let raw = std::fs::read(project_root.join("content/triangle/scene.gasset")).unwrap();
    let (header, payload) = read_content_asset(&raw).expect("readable");
    assert_eq!(header.kind, Scene::name());
    assert_eq!(
        header.asset_id,
        AssetId::from_path("content/triangle/scene.gasset")
    );
    assert_eq!(
        header.provenance,
        Some(ImportProvenance {
            source: fixture().display().to_string(),
            sub_asset: "scene".to_string(),
        }),
        "import must record where the content asset came from"
    );

    let scene: Scene = bincode::deserialize(payload).expect("scene payload");
    let mesh_id = AssetId::from_path("content/triangle/mesh_0.gasset");
    let component = scene.nodes[0]
        .components
        .iter()
        .find(|c| c.type_name == MeshComponent::name())
        .expect("the node carries a MeshComponent");
    let referenced = serde_json::from_str::<MeshComponent>(&component.data)
        .expect("a MeshComponent payload must deserialize")
        .handle
        .id();
    assert_eq!(
        referenced, mesh_id,
        "the MeshComponent must address the content path, not triangle.gltf#mesh/0"
    );
    assert!(
        header.references.contains(&mesh_id),
        "header references list carries the content-path mesh id"
    );

    std::fs::remove_dir_all(&project_root).ok();
}

#[test]
fn import_upserts_the_registry_for_every_written_asset() {
    let project_root =
        std::env::temp_dir().join(format!("import-gltf-registry-{}", std::process::id()));
    std::fs::create_dir_all(&project_root).unwrap();

    let written = import::import_source(&fixture(), &project_root, &Default::default())
        .expect("import succeeds");

    let registry = AssetRegistry::load(&project_root).expect("registry loads");
    for asset in &written {
        assert_eq!(
            registry.get(AssetId::from_path(&asset.address)),
            Some(asset.address.as_str()),
            "import must register every content asset it writes"
        );
    }

    std::fs::remove_dir_all(&project_root).ok();
}
```

The `fixture()` helper and the rest of the file's imports (`ecs::component::Component`, `essential::assets::{Asset, AssetId}`, `mesh::mesh::MeshComponent`, `scene::scene::Scene`) are unchanged.

- [ ] **Step 4: Rewrite `crates/import/tests/end_to_end.rs`**

```rust
//! The full import loop: import writes content assets, the runtime byte
//! path reads them back as the right type, and an editor-saved Scene
//! round-trips.
//!
//! These drive `load_content_asset_bytes` — the exact helper every
//! AssetLoader calls — rather than `AssetServer::load`, because completing
//! an AssetServer load needs the LoadTaskPool plus a World to pump
//! `handle_asset_load_events`, and no such harness exists in the test
//! suite today. This covers the same code path minus the task-pool wrapper.
use std::path::{Path, PathBuf};

use essential::assets::content::{read_content_asset, save_content_asset};
use essential::assets::utils::load_content_asset_bytes;
use essential::assets::{Asset, AssetId, ContentAssetRoot};
use mesh::mesh::Mesh;
use scene::scene::{Scene, SceneNode};

fn fixture() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../gltf-loader/tests/fixtures/triangle.gltf")
        .canonicalize()
        .expect("fixture exists")
}

#[test]
fn imported_content_assets_load_back_as_their_type() {
    let root = std::env::temp_dir().join(format!("content-e2e-{}", std::process::id()));
    std::fs::create_dir_all(&root).unwrap();
    import::import_source(&fixture(), &root, &Default::default()).expect("import");

    let address = "content/triangle/mesh_0.gasset";
    let bytes = pollster::block_on(load_content_asset_bytes(
        &ContentAssetRoot::Directory(root.clone()),
        address,
        Mesh::name(),
    ))
    .expect("the imported content asset loads");

    let mesh: Mesh = bincode::deserialize(&bytes).expect("payload is a Mesh");
    assert_eq!(
        mesh.vertices.len(),
        3,
        "the triangle fixture's mesh survives import -> load"
    );

    let err = pollster::block_on(load_content_asset_bytes(
        &ContentAssetRoot::Directory(root.clone()),
        address,
        Scene::name(),
    ))
    .expect_err("kind tag must reject a Scene load of a Mesh file");
    assert!(format!("{err:#}").contains("Mesh"));

    std::fs::remove_dir_all(&root).ok();
}

#[test]
fn an_editor_saved_scene_round_trips() {
    let root = std::env::temp_dir().join(format!("content-save-e2e-{}", std::process::id()));
    let address = "content/levels/intro.gasset";

    let scene = Scene {
        nodes: vec![SceneNode {
            name: "root".to_string(),
            children: Vec::new(),
            components: Vec::new(),
        }],
        referenced_assets: Vec::new(),
    };
    save_content_asset(&scene, &root, address).expect("save");

    let bytes = pollster::block_on(load_content_asset_bytes(
        &ContentAssetRoot::Directory(root.clone()),
        address,
        Scene::name(),
    ))
    .expect("an editor-saved asset loads through the runtime path");
    let restored: Scene = bincode::deserialize(&bytes).expect("payload is a Scene");
    assert_eq!(restored.nodes.len(), scene.nodes.len());

    let raw = std::fs::read(root.join(address)).unwrap();
    let (header, _) = read_content_asset(&raw).expect("readable");
    assert_eq!(header.kind, Scene::name());
    assert_eq!(header.asset_id, AssetId::from_path(address));
    assert_eq!(
        header.provenance, None,
        "an editor save is not import-derived"
    );

    std::fs::remove_dir_all(&root).ok();
}
```

- [ ] **Step 5: Build and test**

```bash
cargo build --workspace
cargo test --workspace
cargo fmt --all -- --check
cargo clippy -- -A clippy::type_complexity -A clippy::too_many_arguments -D warnings
```

- [ ] **Step 6: Commit**

```bash
git add -A
git commit -m "feat(import): return structured ImportedAsset results; populate provenance; upsert the registry

Co-Authored-By: Claude Sonnet 5 <noreply@anthropic.com>"
```

---

## Task 5: Migrate `render-test` onto content assets

**Files:**
- Modify: `examples/render-test/build.rs`, `examples/render-test/src/main.rs`, `.gitignore`
- Delete: `examples/render-test/assets.toml`, the stale `examples/render-test/res/` directory
- New (generated by Step 1, then committed): `examples/render-test/content/Sponza/...`

**Interfaces:**
- Consumes: `import::import_source` (Task 4), `ContentAssetRoot::default_for_platform`'s new exe-dir default (Task 3).

- [ ] **Step 1: Import Sponza**

```bash
cd examples/render-test
cargo run --release -p import -- assets/Sponza/Sponza.gltf
cd -
```

Expected output ends with `imported assets/Sponza/Sponza.gltf -> N assets` and a `content/Sponza/` directory now exists under `examples/render-test/` containing `scene.gasset` plus one file per mesh/material/texture sub-asset.

- [ ] **Step 2: Delete `assets.toml` and the stale `res/` output**

```bash
git rm examples/render-test/assets.toml
rm -rf examples/render-test/res
```

- [ ] **Step 3: Update `.gitignore`**

Remove these two lines (and the blank line that follows, so the file stays tidy):

```
# render-test cook output (regenerate with: cargo run -p cook -- examples/render-test/assets.toml examples/render-test/assets examples/render-test/res)
/examples/render-test/res/
```

- [ ] **Step 4: Update `examples/render-test/build.rs`**

```rust
use std::env;
use std::path::{Path, PathBuf};

use fs_extra::copy_items;
use fs_extra::dir::CopyOptions;

/// Copies the `content/` directory next to the built binary, so the
/// executable-relative `ContentAssetRoot::Directory` default finds it.
fn main() -> anyhow::Result<()> {
    let manifest_dir = PathBuf::from(env::var("CARGO_MANIFEST_DIR")?).canonicalize()?;
    let content_path = manifest_dir.join("content");
    println!("cargo:rerun-if-changed={}", content_path.display());

    if !content_path.exists() {
        return Ok(());
    }

    // OUT_DIR is target/<profile>/build/<pkg>-<hash>/out, so the profile
    // directory — where the binary lands — is three levels up.
    let out_dir = PathBuf::from(env::var("OUT_DIR")?);
    let output_path = out_dir.ancestors().nth(3).unwrap().to_path_buf();

    copy_items(
        &[content_path],
        Path::new(&output_path),
        &CopyOptions {
            overwrite: true,
            ..Default::default()
        },
    )?;

    Ok(())
}
```

- [ ] **Step 5: Rewrite the load address in `examples/render-test/src/main.rs`**

```rust
const SPONZA_PATH: &str = "content/Sponza/scene.gasset";
```

(replaces `const SPONZA_PATH: &str = "Sponza/Sponza.gltf#scene";`; nothing else in `main.rs` changes)

- [ ] **Step 6: Build**

```bash
cargo build --workspace
cargo test --workspace
```

- [ ] **Step 7: Runtime verification (no fallback exists — this must actually work)**

Build the `xshot` screenshot helper once (reused by Tasks 6 and 7 too) if this environment has an X11/XWayland display available:

```bash
mkdir -p /tmp/xshot && cd /tmp/xshot
cat > Cargo.toml <<'EOF'
[package]
name = "xshot"
version = "0.1.0"
edition = "2021"
[dependencies]
x11rb = "0.13"
image = "0.25"
EOF
mkdir -p src
cat > src/main.rs <<'EOF'
// Finds the top-level window whose WM_NAME contains a substring, GetImages
// it, converts BGRX to RGBA, and saves a PNG. Usage: xshot <name-substring> <out.png>
use x11rb::connection::Connection;
use x11rb::protocol::xproto::*;

fn find_window(conn: &impl Connection, root: Window, needle: &str) -> Option<Window> {
    let tree = conn.query_tree(root).ok()?.reply().ok()?;
    for &win in &tree.children {
        if let Ok(prop) = conn
            .get_property(false, win, AtomEnum::WM_NAME, AtomEnum::STRING, 0, 1024)
            .and_then(|c| c.reply())
        {
            if let Ok(name) = String::from_utf8(prop.value) {
                if name.contains(needle) {
                    return Some(win);
                }
            }
        }
        if let Some(found) = find_window(conn, win, needle) {
            return Some(found);
        }
    }
    None
}

fn main() {
    let args: Vec<String> = std::env::args().collect();
    let needle = &args[1];
    let out_path = &args[2];

    let (conn, screen_num) = x11rb::connect(None).expect("connect");
    let root = conn.setup().roots[screen_num].root;
    let win = find_window(&conn, root, needle).expect("window not found");

    let geom = conn.get_geometry(win).unwrap().reply().unwrap();
    let image = conn
        .get_image(
            ImageFormat::Z_PIXMAP,
            win,
            0,
            0,
            geom.width,
            geom.height,
            !0,
        )
        .unwrap()
        .reply()
        .unwrap();

    let mut rgba = Vec::with_capacity((geom.width as usize) * (geom.height as usize) * 4);
    for chunk in image.data.chunks_exact(4) {
        rgba.extend_from_slice(&[chunk[2], chunk[1], chunk[0], 255]);
    }
    image::save_buffer(
        out_path,
        &rgba,
        geom.width as u32,
        geom.height as u32,
        image::ColorType::Rgba8,
    )
    .unwrap();
}
EOF
cargo build --release
cd -
```

Then, from the repo root:

```bash
env -u WAYLAND_DISPLAY DISPLAY=:1 ./target/debug/render-test > /tmp/render-test.log 2>&1 &
PID=$!
sleep 60
/tmp/xshot/target/release/xshot "winit example" /tmp/render-test-shot.png
kill $PID
grep -c "Failed to load asset" /tmp/render-test.log || true
```

Expected: the grep prints `0` (no load failures) and `/tmp/render-test-shot.png`, read with the Read tool, shows the Sponza atrium (not just the clear color / world grid — per the visual-verification recipe, a debug build of Sponza needs 40-70s+ before geometry appears; if the screenshot looks empty, wait longer before concluding anything is broken). If no `DISPLAY`/XWayland is available in this environment, skip the screenshot and rely on the stderr grep alone, noting in the task report that visual confirmation is deferred to the controller's final review.

- [ ] **Step 8: Commit**

```bash
git add -A
git commit -m "feat(render-test): migrate onto content assets

Sponza is imported into content/Sponza/; build.rs copies content/ instead
of res/; assets.toml is gone.

Co-Authored-By: Claude Sonnet 5 <noreply@anthropic.com>"
```

---

## Task 6: Migrate `tech-demo` onto content assets

**Files:**
- Modify: `examples/tech-demo/build.rs`, `examples/tech-demo/index.html`, `examples/tech-demo/src/scene.rs`, `examples/tech-demo/src/character.rs`, `.gitignore`
- Delete: `examples/tech-demo/assets.toml`, the stale `examples/tech-demo/res/` directory
- New (generated by Step 1, then committed): `examples/tech-demo/content/forest/...`, `examples/tech-demo/content/UAL1/...`

**Interfaces:**
- Consumes: `import::import_source` (Task 4).

- [ ] **Step 1: Import both sources**

```bash
cd examples/tech-demo
cargo run -p import -- assets/forest.glb
cargo run -p import -- assets/UAL1.glb
cd -
```

Expected: `content/forest/scene.gasset` and `content/UAL1/scene.gasset` plus one `content/UAL1/animation_<N>.gasset` per animation UAL1.glb contains.

- [ ] **Step 2: Delete `assets.toml` and the stale `res/` output**

```bash
git rm examples/tech-demo/assets.toml
rm -rf examples/tech-demo/res
```

- [ ] **Step 3: Update `.gitignore`**

Remove:

```
# tech-demo cook output (regenerate with: cargo run -p cook -- examples/tech-demo/assets.toml examples/tech-demo/assets examples/tech-demo/res)
/examples/tech-demo/res/
```

- [ ] **Step 4: Update `examples/tech-demo/build.rs`**

Same rewrite as Task 5 Step 4 (res→content), applied to `examples/tech-demo/build.rs`.

- [ ] **Step 5: Update `examples/tech-demo/index.html`**

Change:

```html
    <link data-trunk rel="copy-dir" href="res" data-target-path="res" />
```

to:

```html
    <link data-trunk rel="copy-dir" href="content" data-target-path="content" />
```

- [ ] **Step 6: Rewrite the load address in `examples/tech-demo/src/scene.rs`**

```rust
const FOREST_SCENE: &str = "content/forest/scene.gasset";
```

- [ ] **Step 7: Rewrite the load addresses in `examples/tech-demo/src/character.rs`**

```rust
const CHAR_SCENE: &str = "content/UAL1/scene.gasset";

// TODO(asset-import-pipeline): magic indices — a cooked animation-name manifest would replace these.
const IDLE_LOOP: &str = "content/UAL1/animation_53.gasset";
const JOG_FWD_LOOP: &str = "content/UAL1/animation_67.gasset";
const JOG_FWD_L_LOOP: &str = "content/UAL1/animation_64.gasset";
const JOG_FWD_R_LOOP: &str = "content/UAL1/animation_68.gasset";
const JOG_LEFT_LOOP: &str = "content/UAL1/animation_69.gasset";
const JOG_RIGHT_LOOP: &str = "content/UAL1/animation_70.gasset";
const JOG_BWD_LOOP: &str = "content/UAL1/animation_62.gasset";
const JOG_BWD_L_LOOP: &str = "content/UAL1/animation_61.gasset";
const JOG_BWD_R_LOOP: &str = "content/UAL1/animation_63.gasset";
const JUMP_START: &str = "content/UAL1/animation_73.gasset";
const JUMP_LOOP: &str = "content/UAL1/animation_72.gasset";
const JUMP_LAND: &str = "content/UAL1/animation_71.gasset";
```

(replaces the equivalent `"UAL1.glb#..."` block; every other line in the file is unchanged)

- [ ] **Step 8: Build**

```bash
cargo build --workspace
cargo test --workspace
```

- [ ] **Step 9: Runtime verification**

```bash
env -u WAYLAND_DISPLAY DISPLAY=:1 ./target/debug/tech-demo > /tmp/tech-demo.log 2>&1 &
PID=$!
sleep 20
/tmp/xshot/target/release/xshot "winit example" /tmp/tech-demo-shot.png
kill $PID
grep -c "Failed to load asset" /tmp/tech-demo.log || true
```

Expected: grep prints `0`; the screenshot shows the forest scene with the animated character. If `DISPLAY`/XWayland is unavailable, skip the screenshot and note it in the task report — the stderr grep is still required.

- [ ] **Step 10: Commit**

```bash
git add -A
git commit -m "feat(tech-demo): migrate onto content assets

forest.glb and UAL1.glb are imported into content/; build.rs and
index.html copy content/ instead of res/; assets.toml is gone.

Co-Authored-By: Claude Sonnet 5 <noreply@anthropic.com>"
```

---

## Task 7: Migrate `animation-test` onto content assets

**Files:**
- Modify: `examples/animation-test/build.rs`, `examples/animation-test/index.html`, `examples/animation-test/src/movement_animation.rs`, `.gitignore`
- Delete: `examples/animation-test/assets.toml`, the stale `examples/animation-test/res/` directory
- New (generated by Step 1, then committed): `examples/animation-test/content/ninja/...`, `content/idle/...`, `content/walk/...`, `content/strafe_left/...`, `content/strafe_right/...`

**Interfaces:**
- Consumes: `import::import_source` (Task 4).

- [ ] **Step 1: Import all five sources**

```bash
cd examples/animation-test
cargo run -p import -- assets/ninja/ninja.glb
cargo run -p import -- assets/ninja/idle.glb
cargo run -p import -- assets/ninja/walk.glb
cargo run -p import -- assets/ninja/strafe_left.glb
cargo run -p import -- assets/ninja/strafe_right.glb
cd -
```

Note: `assets/girl/` and `assets/ninja/walk_backwards.glb` exist on disk but were never in `assets.toml` and are not imported here — they were unused before this migration too; leave them as unimported DCC sources, unchanged.

Expected: `content/ninja/scene.gasset`, and for each of `idle`/`walk`/`strafe_left`/`strafe_right`: `content/<name>/scene.gasset` + `content/<name>/animation_0.gasset`.

- [ ] **Step 2: Delete `assets.toml` and the stale `res/` output**

```bash
git rm examples/animation-test/assets.toml
rm -rf examples/animation-test/res
```

- [ ] **Step 3: Update `.gitignore`**

Remove:

```
# animation-test cook output (regenerate with: cargo run -p cook -- examples/animation-test/assets.toml examples/animation-test/assets examples/animation-test/res)
/examples/animation-test/res/
```

- [ ] **Step 4: Update `examples/animation-test/build.rs`**

Same rewrite as Task 5 Step 4 (res→content), applied to `examples/animation-test/build.rs`.

- [ ] **Step 5: Update `examples/animation-test/index.html`**

Change:

```html
    <link data-trunk rel="rust" href="Cargo.toml" data-wasm-opt="z" data-target-path="res"/>
    <link data-trunk rel="copy-dir" href="res" data-target-path="res"/>
```

to:

```html
    <link data-trunk rel="rust" href="Cargo.toml" data-wasm-opt="z"/>
    <link data-trunk rel="copy-dir" href="content" data-target-path="content"/>
```

(the `rust` link's stray `data-target-path="res"` is dropped, not renamed — it was never meaningful there, only on the `copy-dir` link)

- [ ] **Step 6: Rewrite the load addresses in `examples/animation-test/src/movement_animation.rs`**

```rust
const NINJA_SCENE: &str = "content/ninja/scene.gasset";
const IDLE_ANIM: &str = "content/idle/animation_0.gasset";
const WALK_ANIM: &str = "content/walk/animation_0.gasset";
const STRAFE_LEFT_ANIM: &str = "content/strafe_left/animation_0.gasset";
const STRAFE_RIGHT_ANIM: &str = "content/strafe_right/animation_0.gasset";
```

(replaces the equivalent `"ninja/....glb#..."` block; the rest of the file is unchanged)

- [ ] **Step 7: Build**

```bash
cargo build --workspace
cargo test --workspace
```

- [ ] **Step 8: Runtime verification**

```bash
env -u WAYLAND_DISPLAY DISPLAY=:1 ./target/debug/animation-test > /tmp/animation-test.log 2>&1 &
PID=$!
sleep 15
/tmp/xshot/target/release/xshot "winit example" /tmp/animation-test-shot.png
kill $PID
grep -c "Failed to load asset" /tmp/animation-test.log || true
```

Expected: grep prints `0`; the screenshot shows the ninja character in a non-T-pose (idle animation posed, per the visual-verification recipe's "known-good skinned reference" note). If `DISPLAY`/XWayland is unavailable, skip the screenshot and note it in the task report — the stderr grep is still required.

- [ ] **Step 9: Commit**

```bash
git add -A
git commit -m "feat(animation-test): migrate onto content assets

ninja/idle/walk/strafe_left/strafe_right are imported into content/;
build.rs and index.html copy content/ instead of res/; assets.toml is gone.

Co-Authored-By: Claude Sonnet 5 <noreply@anthropic.com>"
```
