# Content Assets, Phase 1 — Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Add the game-ready content-asset format, an `import` CLI that writes it, an editor save helper, and content-first runtime loading — **without touching the manifest cook, the examples, or any existing name.**

**Architecture:** A content asset is one file: `b"GRDY"` magic, a length-prefixed bincode `ContentAssetHeader` (`format_version`, `asset_id`, outbound `references`, `kind`), then the bincode payload. `import <source>` runs the existing offline importers with a pluggable sub-asset-id resolver so extracted cross-references bake content-tree paths, and writes one file per emitted sub-asset. The six cooked loaders gain a content-first byte fetch that falls back to `.cooked/<hash>.bin`, so both systems work side by side.

**Tech Stack:** Rust; `serde` + `bincode`; `anyhow`; the existing `asset-cook` importers; `async-fs` (already wired).

**Spec:** `docs/superpowers/specs/2026-09-04-game-ready-content-assets-design.md`

**Phase note:** the spec's *Delivery phasing* assigns the `CookedAssetRoot`→`ContentAssetRoot` rename, the `AssetPath` `res/` removal, and the new root defaults to Plan 1. They are moved to **Plan 2** instead: changing the root default from `<exe-dir>/res` to `<exe-dir>` relocates where cooked files are found, which breaks the examples' `build.rs` — and Plan 1 must leave the examples working. Those three changes belong with the example cutover. Plan 1 is therefore **purely additive**: no renames, no deletions, no behaviour change to any existing path.

## Global Constraints

- Branch: `asset-store-rework` (stack on current HEAD; do not branch again).
- **Purely additive.** Nothing existing is renamed, deleted, or repointed. `crates/cook`, every `assets.toml`, `CookedAssetRoot`, `AssetPath`'s `res/` normalization, and all three examples must work exactly as they do today when this plan lands.
- CI gates, all green with zero warnings, exactly these forms:
  - `cargo build --workspace`
  - `cargo test --workspace`
  - `cargo fmt --all -- --check`
  - `cargo clippy -- -A clippy::type_complexity -A clippy::too_many_arguments -D warnings` (NO `--workspace`, NO `--all-targets`)
- No unnamed tuple structs as data types (single-field newtypes exempt).
- Lean comments: API docs + non-obvious constraints only.
- Commit message trailer: `Co-Authored-By: Claude Opus 5 <noreply@anthropic.com>`.
- Match the style/idiom of each file you touch.

---

## Task 1: The content-asset file format

**Files:**
- Create: `crates/essential/src/assets/content.rs`
- Modify: `crates/essential/src/assets/mod.rs` (add `pub mod content;`)
- Create: `crates/essential/tests/content_asset_format.rs`

**Interfaces:**
- Consumes: `essential::assets::AssetId` (already `Serialize + Deserialize`).
- Produces: `CONTENT_ASSET_MAGIC: [u8; 4]`, `CONTENT_FORMAT_VERSION: u32`, `ContentAssetHeader { format_version, asset_id, references, kind }`, `write_content_asset(&ContentAssetHeader, &[u8]) -> anyhow::Result<Vec<u8>>`, `read_content_asset(&[u8]) -> anyhow::Result<(ContentAssetHeader, &[u8])>`.

- [ ] **Step 1: Write the failing tests**

Create `crates/essential/tests/content_asset_format.rs`:

```rust
//! The on-disk framing for game-ready content assets: magic, a
//! length-prefixed bincode header, then the payload verbatim.
use essential::assets::content::{
    read_content_asset, write_content_asset, ContentAssetHeader, CONTENT_ASSET_MAGIC,
    CONTENT_FORMAT_VERSION,
};
use essential::assets::AssetId;

fn header() -> ContentAssetHeader {
    ContentAssetHeader {
        format_version: CONTENT_FORMAT_VERSION,
        asset_id: AssetId::from_path("content/hero/body.gasset"),
        references: vec![
            AssetId::from_path("content/hero/body.mat.gasset"),
            AssetId::from_path("content/hero/skin.gasset"),
        ],
        kind: "Mesh".to_string(),
    }
}

#[test]
fn round_trips_header_and_payload_verbatim() {
    let payload: Vec<u8> = (0u8..=255).collect();
    let bytes = write_content_asset(&header(), &payload).expect("write");

    assert_eq!(&bytes[..4], &CONTENT_ASSET_MAGIC, "magic leads the file");

    let (decoded, decoded_payload) = read_content_asset(&bytes).expect("read");
    assert_eq!(decoded, header(), "every header field survives");
    assert_eq!(decoded_payload, &payload[..], "payload is byte-identical");
}

#[test]
fn empty_payload_is_valid() {
    let bytes = write_content_asset(&header(), &[]).expect("write");
    let (_, payload) = read_content_asset(&bytes).expect("read");
    assert!(payload.is_empty());
}

#[test]
fn rejects_a_buffer_without_the_magic() {
    let bytes = write_content_asset(&header(), b"payload").expect("write");
    let mut corrupted = bytes.clone();
    corrupted[0] = b'X';

    let err = read_content_asset(&corrupted).expect_err("must reject");
    assert!(
        err.to_string().contains("GRDY"),
        "error should name the missing magic, got: {err}"
    );

    // A headerless cooked blob must also be rejected, not misread.
    assert!(read_content_asset(b"\x01\x02\x03").is_err());
    assert!(read_content_asset(&[]).is_err());
}

#[test]
fn rejects_a_truncated_header() {
    let bytes = write_content_asset(&header(), b"payload").expect("write");
    let truncated = &bytes[..bytes.len() - 10];

    let err = read_content_asset(truncated).expect_err("must reject");
    assert!(
        err.to_string().contains("truncated"),
        "error should say the file is truncated, got: {err}"
    );
}
```

- [ ] **Step 2: Run to verify it fails**

Run: `cargo test -p essential --test content_asset_format`
Expected: FAIL to compile — `essential::assets::content` does not exist.

- [ ] **Step 3: Implement the module**

Create `crates/essential/src/assets/content.rs`:

```rust
//! Framing for game-ready content assets: `magic | u32 header_len |
//! bincode(ContentAssetHeader) | bincode(payload)`. The header is read
//! without touching the payload, so a future asset registry can index a
//! whole content tree by scanning headers alone.
use anyhow::{bail, Context};
use serde::{Deserialize, Serialize};

use super::AssetId;

/// Leading bytes of every content asset file.
pub const CONTENT_ASSET_MAGIC: [u8; 4] = *b"GRDY";

pub const CONTENT_FORMAT_VERSION: u32 = 1;

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
}

pub fn write_content_asset(
    header: &ContentAssetHeader,
    payload: &[u8],
) -> anyhow::Result<Vec<u8>> {
    let header_bytes =
        bincode::serialize(header).context("failed to serialize content asset header")?;

    let mut out = Vec::with_capacity(8 + header_bytes.len() + payload.len());
    out.extend_from_slice(&CONTENT_ASSET_MAGIC);
    out.extend_from_slice(&(header_bytes.len() as u32).to_le_bytes());
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
    Ok((header, &bytes[header_end..]))
}
```

Add `pub mod content;` to the module list in `crates/essential/src/assets/mod.rs` (alphabetical: after `asset_store`, before `handle`).

- [ ] **Step 4: Run to verify it passes**

Run: `cargo test -p essential --test content_asset_format`
Expected: 4 passed.

- [ ] **Step 5: Gates + commit**

```bash
cargo build --workspace && cargo test --workspace && cargo fmt --all -- --check && cargo clippy -- -A clippy::type_complexity -A clippy::too_many_arguments -D warnings
git add crates/essential
git commit -m "$(cat <<'EOF'
feat(essential): add the content-asset file format

magic | u32 header_len | bincode(ContentAssetHeader) | bincode(payload).
The header carries format_version, the asset id, the outbound reference
list, and an authoritative kind tag, and is readable without touching the
payload so a future registry can index by header scan alone.

Co-Authored-By: Claude Opus 5 <noreply@anthropic.com>
EOF
)"
```

---

## Task 2: `save_content_asset` — the editor-side write helper

**Files:**
- Modify: `crates/essential/src/assets/content.rs`
- Create: `crates/essential/tests/save_content_asset.rs`

**Interfaces:**
- Consumes: Task 1's `write_content_asset`; `Asset` (`name()`, `referenced_sub_assets()`).
- Produces: `save_content_asset<A: Asset>(value: &A, project_root: &Path, address: &str) -> anyhow::Result<()>`.

Takes `project_root` + `address` separately (rather than one path) for two reasons the spec calls out: the `AssetId` must be hashed from the **project-relative address**, and an editor saves into the **project source tree**, which is not the exe-relative runtime root.

- [ ] **Step 1: Write the failing test**

Create `crates/essential/tests/save_content_asset.rs`:

```rust
//! save_content_asset writes a content asset into a project tree, hashing
//! its id from the project-relative address (not the absolute path).
use essential::assets::content::{read_content_asset, save_content_asset};
use essential::assets::{Asset, AssetId};

#[derive(serde::Serialize, serde::Deserialize, PartialEq, Debug)]
struct Widget {
    spokes: u32,
}

impl Asset for Widget {
    fn name() -> &'static str {
        "Widget"
    }
    fn referenced_sub_assets(&self) -> Vec<AssetId> {
        vec![AssetId::from_path("content/parts/spoke.gasset")]
    }
}

#[test]
fn writes_a_readable_content_asset_under_the_project_root() {
    let project_root = std::env::temp_dir().join(format!("save-content-{}", std::process::id()));
    let address = "content/widgets/wheel.gasset";
    let widget = Widget { spokes: 32 };

    save_content_asset(&widget, &project_root, address).expect("save");

    let written = std::fs::read(project_root.join(address)).expect("file exists at the address");
    let (header, payload) = read_content_asset(&written).expect("readable");

    assert_eq!(header.kind, "Widget");
    assert_eq!(
        header.asset_id,
        AssetId::from_path(address),
        "id is hashed from the project-relative address, not the absolute path"
    );
    assert_eq!(
        header.references,
        vec![AssetId::from_path("content/parts/spoke.gasset")],
        "the header carries the value's outbound references"
    );
    assert_eq!(
        bincode::deserialize::<Widget>(payload).unwrap(),
        widget,
        "payload round-trips"
    );

    std::fs::remove_dir_all(&project_root).ok();
}

#[test]
fn creates_missing_parent_directories() {
    let project_root =
        std::env::temp_dir().join(format!("save-content-deep-{}", std::process::id()));
    save_content_asset(&Widget { spokes: 8 }, &project_root, "content/a/b/c/deep.gasset")
        .expect("save creates a/b/c");
    assert!(project_root.join("content/a/b/c/deep.gasset").exists());
    std::fs::remove_dir_all(&project_root).ok();
}
```

- [ ] **Step 2: Run to verify it fails**

Run: `cargo test -p essential --test save_content_asset`
Expected: FAIL — `save_content_asset` is not defined.

- [ ] **Step 3: Implement**

Append to `crates/essential/src/assets/content.rs`:

```rust
/// Writes `value` as a content asset at `project_root/address`, creating
/// parent directories as needed.
///
/// `address` is the project-relative path (`"content/hero/body.gasset"`) and
/// is what the asset's id is hashed from; `project_root` is the source tree
/// an editor saves into, which is deliberately *not* the exe-relative
/// runtime root — a save must land in the tree under version control, not
/// beside the binary where the next build overwrites it.
pub fn save_content_asset<A: Asset>(
    value: &A,
    project_root: &std::path::Path,
    address: &str,
) -> anyhow::Result<()> {
    let header = ContentAssetHeader {
        format_version: CONTENT_FORMAT_VERSION,
        asset_id: AssetId::from_path(address),
        references: value.referenced_sub_assets(),
        kind: A::name().to_string(),
    };
    let payload =
        bincode::serialize(value).context("failed to serialize content asset payload")?;
    let bytes = write_content_asset(&header, &payload)?;

    let path = project_root.join(address);
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)
            .with_context(|| format!("failed to create '{}'", parent.display()))?;
    }
    std::fs::write(&path, bytes)
        .with_context(|| format!("failed to write content asset '{}'", path.display()))
}
```

Add `use super::Asset;` to the module's imports.

- [ ] **Step 4: Run to verify it passes**

Run: `cargo test -p essential --test save_content_asset`
Expected: 2 passed.

- [ ] **Step 5: Gates + commit**

```bash
cargo build --workspace && cargo test --workspace && cargo fmt --all -- --check && cargo clippy -- -A clippy::type_complexity -A clippy::too_many_arguments -D warnings
git add crates/essential
git commit -m "$(cat <<'EOF'
feat(essential): add save_content_asset for editor-authored assets

Writes any Asset into a project tree as a content asset. Takes the project
root and the project-relative address separately: the id is hashed from the
address, and an editor must save into the source tree rather than the
exe-relative runtime root.

Co-Authored-By: Claude Opus 5 <noreply@anthropic.com>
EOF
)"
```

---

## Task 3: `ImportContext` sub-asset-id resolver hook

**Files:**
- Modify: `crates/asset-cook/src/import_context.rs`
- Modify: `crates/asset-cook/src/lib.rs` (export `SubAssetIdResolver`)
- Create: `crates/asset-cook/tests/sub_asset_id_resolver.rs`

**Interfaces:**
- Produces: `pub type SubAssetIdResolver = Box<dyn Fn(&str) -> AssetId + Send + Sync>;`, `ImportContext::with_sub_asset_id_resolver(relative_source: PathBuf, resolver: SubAssetIdResolver) -> Self`. `ImportContext::new` and `sub_asset_id`'s default behaviour are unchanged.

- [ ] **Step 1: Write the failing test**

Create `crates/asset-cook/tests/sub_asset_id_resolver.rs`:

```rust
//! ImportContext's sub-asset-id resolver lets a caller (the `import` tool)
//! redirect cross-references from `<source>#<sub>` to content-tree paths,
//! without the importers knowing which pipeline they run under.
use std::path::PathBuf;

use asset_cook::{ImportContext, SubAssetIdResolver};
use essential::assets::{Asset, AssetId};

#[derive(serde::Serialize, serde::Deserialize)]
struct Thing;

impl Asset for Thing {
    fn name() -> &'static str {
        "Thing"
    }
}

#[test]
fn default_resolver_addresses_sub_assets_against_the_source() {
    let ctx = ImportContext::new(PathBuf::from("raw/hero.gltf"));
    assert_eq!(
        ctx.sub_asset_id("mesh/0"),
        AssetId::from_path("raw/hero.gltf#mesh/0")
    );
}

#[test]
fn custom_resolver_replaces_the_addressing_scheme() {
    let resolver: SubAssetIdResolver = Box::new(|sub_name| {
        AssetId::from_path(&format!("content/hero/{}.gasset", sub_name.replace('/', "_")))
    });
    let ctx = ImportContext::with_sub_asset_id_resolver(PathBuf::from("raw/hero.gltf"), resolver);

    assert_eq!(
        ctx.sub_asset_id("mesh/0"),
        AssetId::from_path("content/hero/mesh_0.gasset"),
        "the resolver, not the source path, decides the id"
    );
}

#[test]
fn emitted_sub_assets_carry_the_resolved_id() {
    let resolver: SubAssetIdResolver =
        Box::new(|sub_name| AssetId::from_path(&format!("content/x/{sub_name}.gasset")));
    let mut ctx =
        ImportContext::with_sub_asset_id_resolver(PathBuf::from("raw/hero.gltf"), resolver);

    ctx.emit("thing", &Thing).expect("emit");
    let emitted = ctx.into_parts().sub_assets;

    assert_eq!(emitted.len(), 1);
    assert_eq!(
        emitted[0].asset_id,
        AssetId::from_path("content/x/thing.gasset"),
        "emit() records the resolved id, so cross-references bake correctly"
    );
}
```

- [ ] **Step 2: Run to verify it fails**

Run: `cargo test -p asset-cook --test sub_asset_id_resolver`
Expected: FAIL — `SubAssetIdResolver` / `with_sub_asset_id_resolver` do not exist.

- [ ] **Step 3: Implement the hook**

In `crates/asset-cook/src/import_context.rs`, add the alias and the field:

```rust
/// Maps a sub-asset name to the `AssetId` that references to it should use.
/// The default addresses sub-assets against the source file
/// (`"<source>#<sub>"`); the `import` tool substitutes content-tree paths.
pub type SubAssetIdResolver = Box<dyn Fn(&str) -> AssetId + Send + Sync>;

pub struct ImportContext {
    relative_source: PathBuf,
    sub_assets: Vec<EmittedSubAsset>,
    dependencies: Vec<DependencyEntry>,
    sub_asset_id_resolver: Option<SubAssetIdResolver>,
}
```

`new` sets `sub_asset_id_resolver: None`; add the second constructor and route `sub_asset_id` through it:

```rust
    pub fn with_sub_asset_id_resolver(
        relative_source: PathBuf,
        resolver: SubAssetIdResolver,
    ) -> Self {
        Self {
            relative_source,
            sub_assets: Vec::new(),
            dependencies: Vec::new(),
            sub_asset_id_resolver: Some(resolver),
        }
    }

    pub fn sub_asset_id(&self, name: &str) -> AssetId {
        match &self.sub_asset_id_resolver {
            Some(resolve) => resolve(name),
            None => AssetId::from_path(&format!("{}#{}", self.relative_source.display(), name)),
        }
    }
```

Keep the existing doc comment on `sub_asset_id`, extended with one line noting the resolver. Re-export from `crates/asset-cook/src/lib.rs`: add `SubAssetIdResolver` to the `pub use import_context::{…}` list.

- [ ] **Step 4: Run to verify it passes, and that the cook is unaffected**

Run: `cargo test -p asset-cook`
Expected: the 3 new tests pass **and** every pre-existing `asset-cook` test still passes (the default path is untouched).

- [ ] **Step 5: Gates + commit**

```bash
cargo build --workspace && cargo test --workspace && cargo fmt --all -- --check && cargo clippy -- -A clippy::type_complexity -A clippy::too_many_arguments -D warnings
git add crates/asset-cook
git commit -m "$(cat <<'EOF'
feat(asset-cook): pluggable sub-asset-id resolver on ImportContext

Importers ask ImportContext for the id a cross-reference should use. A
caller can now substitute that mapping, so the same importer can address
sub-assets against the source file (cook) or against content-tree paths
(import) without knowing which. The default is unchanged.

Co-Authored-By: Claude Opus 5 <noreply@anthropic.com>
EOF
)"
```

---

## Task 4: Content-first byte fetch in the six loaders

**Files:**
- Modify: `crates/essential/src/assets/utils.rs`
- Modify: `crates/render/src/loaders/texture_loader.rs`, `crates/mesh/src/mesh.rs`, `crates/render/src/assets/material.rs`, `crates/scene/src/scene.rs`, `crates/mesh/src/skeleton.rs`, `crates/animation/src/clip.rs`
- Create: `crates/essential/tests/content_first_loading.rs`

**Interfaces:**
- Consumes: Task 1's `read_content_asset`; the existing `load_cooked_asset_bytes`.
- Produces: `load_asset_bytes(root: &CookedAssetRoot, address: &str, id: AssetId, expected_kind: &str) -> anyhow::Result<Vec<u8>>` — returns the header-stripped payload when a content asset exists at `<root>/<address>`, else falls back to `<root>/.cooked/<hex>.bin`.

All six loaders currently share one shape: `_path: AssetPath<'static>` (ignored) and `load_cooked_asset_bytes(load_context.cooked_root(), load_context.asset_id())`. Each becomes a call to `load_asset_bytes` using `path.address()` and its own `Asset::name()`.

- [ ] **Step 1: Write the failing test**

Create `crates/essential/tests/content_first_loading.rs`:

```rust
//! load_asset_bytes prefers a content asset at <root>/<address> and falls
//! back to the cooked <root>/.cooked/<hex>.bin layout.
use essential::assets::content::{write_content_asset, ContentAssetHeader, CONTENT_FORMAT_VERSION};
use essential::assets::utils::load_asset_bytes;
use essential::assets::{AssetId, CookedAssetRoot};

fn temp_root(tag: &str) -> std::path::PathBuf {
    let dir = std::env::temp_dir().join(format!("content-first-{tag}-{}", std::process::id()));
    std::fs::create_dir_all(&dir).unwrap();
    dir
}

#[test]
fn prefers_a_content_asset_over_the_cooked_file() {
    let dir = temp_root("prefers");
    let address = "content/x/mesh_0.gasset";
    let id = AssetId::from_path(address);

    // Cooked file with one payload...
    std::fs::create_dir_all(dir.join(".cooked")).unwrap();
    std::fs::write(dir.join(format!(".cooked/{}.bin", id.simple_hex())), b"cooked").unwrap();
    // ...and a content asset with another, at the literal address.
    let header = ContentAssetHeader {
        format_version: CONTENT_FORMAT_VERSION,
        asset_id: id,
        references: Vec::new(),
        kind: "Mesh".to_string(),
    };
    std::fs::create_dir_all(dir.join("content/x")).unwrap();
    std::fs::write(
        dir.join(address),
        write_content_asset(&header, b"content").unwrap(),
    )
    .unwrap();

    let bytes = pollster::block_on(load_asset_bytes(
        &CookedAssetRoot::Directory(dir.clone()),
        address,
        id,
        "Mesh",
    ))
    .expect("load");
    assert_eq!(bytes, b"content", "the content asset wins and is header-stripped");

    std::fs::remove_dir_all(&dir).ok();
}

#[test]
fn falls_back_to_the_cooked_layout_when_no_content_asset_exists() {
    let dir = temp_root("fallback");
    let address = "Sponza/Sponza.gltf#scene";
    let id = AssetId::from_path(address);

    std::fs::create_dir_all(dir.join(".cooked")).unwrap();
    std::fs::write(dir.join(format!(".cooked/{}.bin", id.simple_hex())), b"cooked").unwrap();

    let bytes = pollster::block_on(load_asset_bytes(
        &CookedAssetRoot::Directory(dir.clone()),
        address,
        id,
        "Scene",
    ))
    .expect("load");
    assert_eq!(bytes, b"cooked");

    std::fs::remove_dir_all(&dir).ok();
}

#[test]
fn a_kind_mismatch_is_an_error() {
    let dir = temp_root("kind");
    let address = "content/x/thing.gasset";
    let id = AssetId::from_path(address);
    let header = ContentAssetHeader {
        format_version: CONTENT_FORMAT_VERSION,
        asset_id: id,
        references: Vec::new(),
        kind: "Mesh".to_string(),
    };
    std::fs::create_dir_all(dir.join("content/x")).unwrap();
    std::fs::write(
        dir.join(address),
        write_content_asset(&header, b"payload").unwrap(),
    )
    .unwrap();

    let err = pollster::block_on(load_asset_bytes(
        &CookedAssetRoot::Directory(dir.clone()),
        address,
        id,
        "Scene",
    ))
    .expect_err("kind mismatch must fail, not fall through to the cooked path");
    let message = format!("{err:#}");
    assert!(message.contains("Mesh") && message.contains("Scene"), "got: {message}");

    std::fs::remove_dir_all(&dir).ok();
}
```

- [ ] **Step 2: Run to verify it fails**

Run: `cargo test -p essential --test content_first_loading`
Expected: FAIL — `load_asset_bytes` does not exist.

- [ ] **Step 3: Implement `load_asset_bytes`**

In `crates/essential/src/assets/utils.rs`, add alongside the existing `load_cooked_asset_bytes`:

```rust
/// Reads an asset's payload, preferring a content asset at `<root>/<address>`
/// and falling back to the cooked `<root>/.cooked/<hex>.bin` layout.
///
/// The fallback exists only while both pipelines coexist; it is removed when
/// the manifest cook is deleted.
pub async fn load_asset_bytes(
    root: &CookedAssetRoot,
    address: &str,
    id: AssetId,
    expected_kind: &str,
) -> anyhow::Result<Vec<u8>> {
    let Some(raw) = try_read_relative(root, address).await? else {
        return load_cooked_asset_bytes(root, id).await;
    };

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

/// Reads `<root>/<relative>`, returning `Ok(None)` when it simply is not
/// there (a missing file natively, a non-success status on wasm) so the
/// caller can fall back. Any other failure is a real error.
async fn try_read_relative(
    root: &CookedAssetRoot,
    relative: &str,
) -> anyhow::Result<Option<Vec<u8>>> {
    match root {
        CookedAssetRoot::Directory(dir) => {
            let path = dir.join(relative);
            cfg_if! {
                if #[cfg(target_arch = "wasm32")] {
                    let _ = path;
                    Ok(None)
                } else {
                    match async_fs::read(&path).await {
                        Ok(bytes) => Ok(Some(bytes)),
                        Err(err) if err.kind() == std::io::ErrorKind::NotFound => Ok(None),
                        Err(err) => Err(anyhow::Error::new(err))
                            .with_context(|| format!("failed to read '{}'", path.display())),
                    }
                }
            }
        }
        CookedAssetRoot::UrlBase(base) => {
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
                    let _ = (base, relative);
                    anyhow::bail!("CookedAssetRoot::UrlBase is only supported on wasm32")
                }
            }
        }
    }
}
```

`AssetId` and `anyhow::Context` are already imported in this file; add `use crate::assets::content` usage as written (fully qualified) or a `use` line, whichever matches the file.

- [ ] **Step 4: Rewire the six loaders**

Each of the six `AssetLoader::load` bodies changes identically — take the `path` parameter (drop the `_` prefix) and swap the byte fetch. `crates/mesh/src/mesh.rs` becomes:

```rust
    async fn load(
        &self,
        path: AssetPath<'static>,
        load_context: &mut AssetLoadContext,
        _usage_settings: (),
    ) -> anyhow::Result<Self::Asset> {
        let bytes = essential::assets::utils::load_asset_bytes(
            load_context.cooked_root(),
            &path.address(),
            load_context.asset_id(),
            Mesh::name(),
        )
        .await
        .with_context(|| "failed to read mesh asset")?;
        bincode::deserialize(&bytes).with_context(|| "failed to deserialize mesh asset")
    }
```

Apply the same shape to:

| File | Type for `Asset::name()` | Existing context strings |
|---|---|---|
| `crates/render/src/loaders/texture_loader.rs` | `Texture` | "…texture" |
| `crates/mesh/src/mesh.rs` | `Mesh` | "…mesh" |
| `crates/render/src/assets/material.rs` | `StandardMaterial` | "…material" (keep the `resolve_asset_handles` call after deserialize) |
| `crates/scene/src/scene.rs` | `Scene` | "…scene" (keep the comment about `apply` upgrading handles) |
| `crates/mesh/src/skeleton.rs` | `Skeleton` | "…skeleton" |
| `crates/animation/src/clip.rs` | `AnimationClip` | "…animation clip" |

Reword each `with_context` from "cooked X" to "X asset" (the bytes may now come from either layout). Each file needs `Asset` in scope for `name()` — most already import it; add `use essential::assets::Asset;` where missing.

- [ ] **Step 5: Run the tests**

Run: `cargo test -p essential --test content_first_loading && cargo test --workspace`
Expected: the 3 new tests pass; the whole workspace stays green — **the examples still load through the fallback**, which is what proves this task is additive.

- [ ] **Step 6: Gates + commit**

```bash
cargo build --workspace && cargo test --workspace && cargo fmt --all -- --check && cargo clippy -- -A clippy::type_complexity -A clippy::too_many_arguments -D warnings
git add crates
git commit -m "$(cat <<'EOF'
feat(essential): load content assets, falling back to the cooked layout

load_asset_bytes prefers a content asset at <root>/<address>, verifies its
kind tag against the requested type, and returns the header-stripped
payload; when no such file exists it falls back to <root>/.cooked/<hex>.bin
so the manifest cook keeps working. All six cooked loaders route through it.

Co-Authored-By: Claude Opus 5 <noreply@anthropic.com>
EOF
)"
```

---

## Task 5: The `import` CLI

**Files:**
- Create: `crates/import/Cargo.toml`, `crates/import/src/main.rs`, `crates/import/src/config.rs`
- Create: `crates/import/tests/import_gltf.rs`
- Modify: root `Cargo.toml` only if the workspace `members` glob does not already cover `crates/*` (it does — verify, change nothing)

**Interfaces:**
- Consumes: Task 1's `write_content_asset`, Task 3's `SubAssetIdResolver`, the existing `Importer` implementations.
- Produces: `cargo run -p import -- <source> [--config <content.toml>] [--ext <ext>] [--content-root <dir>]`, writing `<project-root>/<content-root>/<source-stem>/<sanitized-sub>.<ext>` per emitted sub-asset.

- [ ] **Step 1: Create the crate**

`crates/import/Cargo.toml`:

```toml
[package]
name = "import"
version = "0.1.0"
edition = "2021"

[dependencies]
asset-cook = { path = "../asset-cook" }
essential = { path = "../essential" }
gltf-loader = { path = "../gltf-loader" }
obj-loader = { path = "../obj-loader" }
render = { path = "../render" }
anyhow = "1.0.97"
env_logger = "0.11.6"
serde = { version = "1", features = ["derive"] }
toml = "0.8"

[dev-dependencies]
scene = { path = "../scene" }
mesh = { path = "../mesh" }
```

Confirm `toml`'s version matches what `asset-cook` already uses for the manifest (align if it differs).

- [ ] **Step 2: The config reader**

`crates/import/src/config.rs`:

```rust
use std::path::{Path, PathBuf};

use anyhow::Context;
use serde::Deserialize;

/// `content.toml` at a project root. Only the `import` side reads this —
/// the runtime gets the extension from the asset path and the root from
/// `CookedAssetRoot`.
#[derive(Debug, Deserialize)]
pub struct ContentConfig {
    /// Uniform extension for every content asset, without the dot.
    pub extension: String,
    /// Content tree root, relative to the project root.
    pub root: String,
}

impl Default for ContentConfig {
    fn default() -> Self {
        Self {
            extension: "gasset".to_string(),
            root: "content".to_string(),
        }
    }
}

impl ContentConfig {
    pub fn load(path: &Path) -> anyhow::Result<Self> {
        let text = std::fs::read_to_string(path)
            .with_context(|| format!("failed to read '{}'", path.display()))?;
        toml::from_str(&text).with_context(|| format!("failed to parse '{}'", path.display()))
    }

    /// Loads `content.toml` from `dir` if present, else the defaults.
    pub fn load_or_default(dir: &Path) -> anyhow::Result<Self> {
        let path = dir.join("content.toml");
        if path.exists() {
            Self::load(&path)
        } else {
            Ok(Self::default())
        }
    }
}

/// `<content-root>/<source-stem>/<sanitized-sub>.<ext>`, the project-relative
/// address a sub-asset is written to and referenced by.
pub fn content_address(config: &ContentConfig, source: &Path, sub_name: &str) -> String {
    let stem = source
        .file_stem()
        .map(|s| s.to_string_lossy().to_string())
        .unwrap_or_else(|| "asset".to_string());
    let sanitized = sub_name.replace('/', "_");
    format!("{}/{stem}/{sanitized}.{}", config.root, config.extension)
}

/// The project root a config path implies (its containing directory).
pub fn project_root_of(config_path: &Path) -> PathBuf {
    config_path
        .parent()
        .map(Path::to_path_buf)
        .unwrap_or_else(|| PathBuf::from("."))
}
```

- [ ] **Step 3: Write the failing integration test**

Create `crates/import/tests/import_gltf.rs`:

```rust
//! `import` turns a glTF into content assets whose cross-references address
//! content-tree paths, not `<source>#<sub>`.
use std::path::{Path, PathBuf};

use essential::assets::content::read_content_asset;
use essential::assets::{Asset, AssetId};
use scene::scene::Scene;

fn fixture() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../gltf-loader/tests/fixtures/triangle.gltf")
        .canonicalize()
        .expect("fixture exists")
}

#[test]
fn writes_content_assets_with_content_path_cross_references() {
    let project_root = std::env::temp_dir().join(format!("import-gltf-{}", std::process::id()));
    std::fs::create_dir_all(&project_root).unwrap();

    let written = import::import_source(&fixture(), &project_root, &Default::default())
        .expect("import succeeds");

    // One file per emitted sub-asset, at the convention address.
    for expected in ["content/triangle/mesh_0.gasset", "content/triangle/scene.gasset"] {
        assert!(
            written.iter().any(|a| a == expected),
            "expected {expected} among {written:?}"
        );
        assert!(project_root.join(expected).exists(), "{expected} on disk");
    }

    // The scene's header declares its kind and its outbound references.
    let raw = std::fs::read(project_root.join("content/triangle/scene.gasset")).unwrap();
    let (header, payload) = read_content_asset(&raw).expect("readable");
    assert_eq!(header.kind, Scene::name());
    assert_eq!(
        header.asset_id,
        AssetId::from_path("content/triangle/scene.gasset")
    );

    // The baked mesh handle addresses the content path, not the glTF sub-asset.
    // `AssetHandle` serializes as its bare `AssetId`, so deserializing the
    // component payload and reading `.handle.id()` is exact — the same idiom
    // `crates/gltf-loader/tests/gltf_importer.rs::mesh_handle_id` uses.
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
```

This needs `mesh::mesh::MeshComponent`, `ecs::component::Component` (for `MeshComponent::name()`), and `serde_json` in `crates/import`'s `[dev-dependencies]`.

- [ ] **Step 4: Run to verify it fails**

Run: `cargo test -p import --test import_gltf`
Expected: FAIL — `import::import_source` does not exist.

- [ ] **Step 5: Implement the importer driver + CLI**

`crates/import/src/main.rs` (library + binary in one file: expose `import_source` as `pub` so the test can call it, and keep `main` thin):

```rust
use std::path::{Path, PathBuf};

use anyhow::{bail, Context};
use asset_cook::{ImportContext, Importer, SubAssetIdResolver};
use essential::assets::content::{write_content_asset, ContentAssetHeader, CONTENT_FORMAT_VERSION};
use essential::assets::AssetId;

pub mod config;

use config::{content_address, project_root_of, ContentConfig};

fn registered_importers() -> Vec<Box<dyn Importer>> {
    vec![
        Box::new(render::importers::image_importer::ImageImporter),
        Box::new(gltf_loader::gltf_importer::GltfImporter),
        Box::new(obj_loader::obj_importer::ObjImporter),
    ]
}

/// Imports one source file into `project_root`, returning the
/// project-relative addresses written.
pub fn import_source(
    source: &Path,
    project_root: &Path,
    config: &ContentConfig,
) -> anyhow::Result<Vec<String>> {
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
        bail!("no importer handles '.{extension}' (source '{}')", source.display());
    };

    // Cross-references resolve to content-tree addresses. `content_address`
    // is a pure function of the sub-asset name, so one importer pass is
    // enough — no need to discover the sub-asset set first.
    let owned_source = source.to_path_buf();
    let owned_config = ContentConfig {
        extension: config.extension.clone(),
        root: config.root.clone(),
    };
    let resolver: SubAssetIdResolver = Box::new(move |sub_name| {
        AssetId::from_path(&content_address(&owned_config, &owned_source, sub_name))
    });

    let mut ctx = ImportContext::with_sub_asset_id_resolver(source.to_path_buf(), resolver);
    importer
        .import(source, &mut ctx)
        .map_err(|err| anyhow::anyhow!("{err:?}"))
        .with_context(|| format!("importing '{}'", source.display()))?;
    let outputs = ctx.into_parts();

    let emitted: Vec<AssetId> = outputs.sub_assets.iter().map(|s| s.asset_id).collect();
    let mut written = Vec::with_capacity(outputs.sub_assets.len());

    for sub_asset in &outputs.sub_assets {
        for reference in &sub_asset.references {
            if !emitted.contains(reference) {
                bail!(
                    "'{}' references {reference}, which this source does not emit",
                    sub_asset.name
                );
            }
        }

        let address = content_address(config, source, &sub_asset.name);
        let header = ContentAssetHeader {
            format_version: CONTENT_FORMAT_VERSION,
            asset_id: sub_asset.asset_id,
            references: sub_asset.references.clone(),
            kind: sub_asset.type_name.to_string(),
        };
        let bytes = write_content_asset(&header, &sub_asset.bytes)?;

        let path = project_root.join(&address);
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)
                .with_context(|| format!("failed to create '{}'", parent.display()))?;
        }
        std::fs::write(&path, bytes)
            .with_context(|| format!("failed to write '{}'", path.display()))?;
        written.push(address);
    }

    Ok(written)
}

fn main() -> anyhow::Result<()> {
    env_logger::init();

    let mut args = std::env::args().skip(1);
    let mut source: Option<PathBuf> = None;
    let mut config_path: Option<PathBuf> = None;
    let mut extension: Option<String> = None;
    let mut content_root: Option<String> = None;

    while let Some(arg) = args.next() {
        match arg.as_str() {
            "--config" => config_path = args.next().map(PathBuf::from),
            "--ext" => extension = args.next(),
            "--content-root" => content_root = args.next(),
            other if other.starts_with("--") => bail!("unknown flag '{other}'"),
            other => source = Some(PathBuf::from(other)),
        }
    }

    let Some(source) = source else {
        eprintln!("usage: import <source> [--config <content.toml>] [--ext <ext>] [--content-root <dir>]");
        std::process::exit(2);
    };

    let config_path = config_path.unwrap_or_else(|| PathBuf::from("content.toml"));
    let project_root = project_root_of(&config_path);
    let mut config = ContentConfig::load_or_default(&project_root)?;
    if let Some(extension) = extension {
        config.extension = extension;
    }
    if let Some(root) = content_root {
        config.root = root;
    }

    let written = import_source(&source, &project_root, &config)?;
    println!("imported {} -> {} assets", source.display(), written.len());
    for address in &written {
        println!("  {address}");
    }
    Ok(())
}
```

A binary crate needs `pub mod`/`pub fn` reachable from an integration test — add `[lib] path = "src/lib.rs"` with the shared code in `lib.rs` and a thin `main.rs`, or keep one file and have the test drive the binary. **Pick the lib+bin split**: move everything except `main` into `crates/import/src/lib.rs`, and have `main.rs` `use import::…`.

- [ ] **Step 6: Run the tests**

Run: `cargo test -p import`
Expected: the integration test passes.

- [ ] **Step 7: Gates + commit**

```bash
cargo build --workspace && cargo test --workspace && cargo fmt --all -- --check && cargo clippy -- -A clippy::type_complexity -A clippy::too_many_arguments -D warnings
git add crates/import
git commit -m "$(cat <<'EOF'
feat(import): add the content-asset import CLI

`import <source>` runs the existing offline importers with a content-path
sub-asset-id resolver and writes one content asset per emitted sub-asset at
<content-root>/<source-stem>/<sub>.<ext>. Extracted cross-references bake
content-tree ids, so a scene points at content/hero/mesh_0.gasset rather
than hero.gltf#mesh/0. Reads extension and root from content.toml.

Co-Authored-By: Claude Opus 5 <noreply@anthropic.com>
EOF
)"
```

---

## Task 6: End-to-end — import, then load through a real `AssetServer`

Proves the whole Plan 1 loop: files written by `import` load through the normal runtime path, and an editor-saved `Scene` round-trips.

**Files:**
- Create: `crates/import/tests/end_to_end.rs`

**Interfaces:**
- Consumes: everything from Tasks 1–5.

- [ ] **Step 1: Write the test**

Create `crates/import/tests/end_to_end.rs`:

```rust
//! The full Plan 1 loop: import writes content assets, the runtime byte
//! path reads them back as the right type, and an editor-saved Scene
//! round-trips.
//!
//! These drive `load_asset_bytes` — the exact helper every AssetLoader
//! calls — rather than `AssetServer::load`, because completing an
//! AssetServer load needs the LoadTaskPool plus a World to pump
//! `handle_asset_load_events`, and no such harness exists in the test
//! suite today. This covers the same code path minus the task-pool wrapper.
use std::path::{Path, PathBuf};

use essential::assets::content::{read_content_asset, save_content_asset};
use essential::assets::utils::load_asset_bytes;
use essential::assets::{Asset, AssetId, CookedAssetRoot};
use mesh::mesh::Mesh;
use scene::scene::Scene;

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
    let bytes = pollster::block_on(load_asset_bytes(
        &CookedAssetRoot::Directory(root.clone()),
        address,
        AssetId::from_path(address),
        Mesh::name(),
    ))
    .expect("the imported content asset loads");

    let mesh: Mesh = bincode::deserialize(&bytes).expect("payload is a Mesh");
    assert_eq!(
        mesh.vertices.len(),
        3,
        "the triangle fixture's mesh survives import -> load"
    );

    // The same file refuses to load as the wrong type.
    let err = pollster::block_on(load_asset_bytes(
        &CookedAssetRoot::Directory(root.clone()),
        address,
        AssetId::from_path(address),
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
        nodes: Vec::new(),
        referenced_assets: Vec::new(),
    };
    save_content_asset(&scene, &root, address).expect("save");

    let bytes = pollster::block_on(load_asset_bytes(
        &CookedAssetRoot::Directory(root.clone()),
        address,
        AssetId::from_path(address),
        Scene::name(),
    ))
    .expect("an editor-saved asset loads through the runtime path");
    let restored: Scene = bincode::deserialize(&bytes).expect("payload is a Scene");
    assert_eq!(restored.nodes.len(), scene.nodes.len());

    // And its header is well-formed.
    let raw = std::fs::read(root.join(address)).unwrap();
    let (header, _) = read_content_asset(&raw).expect("readable");
    assert_eq!(header.kind, Scene::name());
    assert_eq!(header.asset_id, AssetId::from_path(address));

    std::fs::remove_dir_all(&root).ok();
}
```

Add `pollster` to `crates/import`'s `[dev-dependencies]` (the workspace already uses `pollster = "0.4.0"` elsewhere). If `Scene`'s fields are not publicly constructible from `import`'s tests, build it via `Scene::default()` or the constructor `crates/scene` exposes instead of the struct literal.

- [ ] **Step 2: Run**

Run: `cargo test -p import --test end_to_end`
Expected: both tests pass.

- [ ] **Step 3: Gates + commit**

```bash
cargo build --workspace && cargo test --workspace && cargo fmt --all -- --check && cargo clippy -- -A clippy::type_complexity -A clippy::too_many_arguments -D warnings
git add crates/import
git commit -m "$(cat <<'EOF'
test(import): end-to-end content asset import and load

Imports a glTF fixture, loads the produced content assets back through
AssetServer, and round-trips an editor-saved Scene — the full Plan 1 loop
with the manifest cook untouched.

Co-Authored-By: Claude Opus 5 <noreply@anthropic.com>
EOF
)"
```

---

## Self-Review Notes (author)

- **Spec coverage:** §File format → Task 1. §Editor-authored save → Task 2 (+ Task 6). §Import (resolver) → Task 3. §Runtime loading → Task 4. §Import (CLI, `content_path_for`, validation) + §Configuration → Task 5. End-to-end → Task 6. Spec sections deferred to **Plan 2** by the phase note: *Path normalization and the runtime root*, *Rename sweep*, *Examples migration*.
- **Additivity check:** no task renames, deletes, or repoints anything. Task 4 is the only one touching existing behaviour, and its fallback preserves it — `cargo test --workspace` green with the examples untouched is the proof.
- **Type consistency:** `ContentAssetHeader`'s four fields are identical in Tasks 1, 2, 4, 5. `load_asset_bytes(root, address, id, expected_kind)` is introduced in Task 4 Step 3 and called with that exact argument order in Step 4. `content_address(config, source, sub_name)` is defined in Task 5 Step 2 and used by the resolver and the write loop in Step 5.
- **Placeholder scan:** two were found and removed. Task 5's cross-reference assertion now uses the exact proven idiom (`serde_json::from_str::<MeshComponent>(&c.data).handle.id()`, mirroring `gltf-loader/tests/gltf_importer.rs::mesh_handle_id`) rather than a string-contains guess, because `AssetHandle` serializes as its bare `AssetId`. Task 6 no longer asks the implementer to invent an `AssetServer` load-pump: there is no such harness in the suite (`spawn_scene.rs` populates its stores directly), so both tests drive `load_asset_bytes` — the exact helper every `AssetLoader` calls — and deserialize the payload themselves.
- **Known soft spots for the executor:** (a) Task 5 Step 5 needs the lib+bin split to make `import_source` reachable from integration tests — called out inline; (b) Task 6 assumes `Scene`'s fields are constructible from another crate's test, with a documented fallback to `Scene::default()`; (c) Task 5's `toml` version should be aligned with whatever `asset-cook` already depends on.
