# Merging `Asset` and `CookedAsset` — Design

**Status:** approved for planning
**Date:** 2026-09-03
**Author:** brainstormed with Claude

## Problem

The engine has two overlapping asset traits:

- `essential::assets::Asset` — `Send + Sync + 'static` with `fn name() -> &'static str`. The runtime marker: `AssetStore<A: Asset>`, `AssetServer::add<A: Asset>`, `App::register_asset<A: Asset>` are all generic over it.
- `asset_cook::CookedAsset` — `Serialize + DeserializeOwned` with `const TYPE_NAME: &'static str` and `fn referenced_sub_assets(&self) -> Vec<AssetId>`. What the offline cook pipeline serializes and what the cooked-bytes loaders deserialize.

For almost every asset the two are implemented on the *same* type (`Mesh`, `Skeleton`, `AnimationClip`, `Scene`, `StandardMaterial`). The split forces a parallel DTO exactly once — `CookedTexture` shadows `Texture` — because `Texture` embeds a `wgpu_types::TextureDescriptor<Option<&'static str>, &'static [TextureFormat]>` (via `TextureUsageSettings`) whose `&'static` references cannot `Deserialize`.

Result: `TYPE_NAME` duplicates `name()`, importers and loaders juggle two vocabularies, and `CookedTexture`/`Texture::from_cooked` exist purely to bridge a gap that is one struct field wide.

## Goal

One trait. `CookedAsset` is deleted; `Asset` absorbs its responsibilities and gains a `Serialize + DeserializeOwned` supertrait bound, so every asset is uniformly serializable and cook-able. The cook `emit` path and the cooked loaders then need no extra trait vocabulary — a bare `T: Asset` bound carries serialization.

Non-goal for this plan: making `AnimationGraph` genuinely serializable. It is closure-laden (see below) and its redesign is a separate, later project. This plan keeps the build green with a placeholder.

## Scope

### 1. The merged trait

`crates/essential/src/assets/mod.rs`:

```rust
pub trait Asset:
    Send + Sync + 'static + serde::Serialize + serde::de::DeserializeOwned
{
    fn name() -> &'static str;

    /// AssetIds of every sub-asset this one references — consumed by the
    /// cook tool's reference-integrity pass. Empty for leaf assets.
    fn referenced_sub_assets(&self) -> Vec<AssetId> {
        Vec::new()
    }
}
```

- `LoadableAsset: Asset` is unchanged in shape; it inherits the serde bound transitively.
- `essential` already depends on `serde` with `derive`. No new dependency.

`crates/asset-cook/src/lib.rs`:

- Delete the `CookedAsset` trait and its `pub use` (it is not re-exported today, but the `use serde::{de::DeserializeOwned, Serialize}` import it needed can go).
- `asset-cook` already depends on `essential`; bring `essential::assets::Asset` into scope where `CookedAsset` was used.

`crates/asset-cook/src/import_context.rs`:

- `emit<T: CookedAsset>` → `emit<T: Asset>`. Body unchanged except `type_name: T::TYPE_NAME` → `type_name: T::name()` and `value.referenced_sub_assets()` still resolves (now an `Asset` method).
- `bincode::serialize(value)` still compiles: `T: Asset` implies `T: Serialize`.

`TYPE_NAME` → `name()` equivalence is verified: every hand-written `TYPE_NAME` (`"Mesh"`, `"Texture"`, `"Skeleton"`, `"AnimationClip"`) already equals `stringify!` of the type, which is exactly what `#[derive(Asset)]` emits for `name()`. The one consumer of `type_name` is the diagnostic label on `EmittedSubAsset`/`SubAssetEntry`; behaviour is identical.

`asset-cook` tests (`import_context.rs`, `cook.rs`, `incremental.rs`, `validation.rs`) each define a fake `impl CookedAsset for FakeThing { const TYPE_NAME = ...; }`. Convert each to `#[derive(Serialize, Deserialize)] struct FakeThing {…}` + `impl Asset for FakeThing { fn name() -> &'static str { "FakeThing" } }` (plus `referenced_sub_assets` override where the test exercises references). These fakes are not `Send + Sync + 'static` concerns — plain structs already satisfy that.

### 2. `Texture` rework

`crates/render/src/assets/texture.rs` — new shape:

```rust
#[derive(Asset, serde::Serialize, serde::Deserialize)]
pub struct Texture {
    pub width: u32,
    pub height: u32,
    pub format: wgpu_types::TextureFormat,
    pub kind: TextureKind,
    /// RGBA8 (or block-compressed) pixels matching `format`. Empty for
    /// `TextureKind::RenderTarget` — the camera system allocates those.
    pub data: Vec<u8>,
}

#[derive(serde::Serialize, serde::Deserialize, Clone, Copy, PartialEq, Eq, Debug)]
pub enum TextureKind {
    Sampled,
    RenderTarget,
}
```

- **Enable the `serde` feature on `wgpu-types`.** `crates/render/Cargo.toml`: `wgpu-types = { version = "24", default-features = false, features = ["serde"] }`. Cargo feature unification means `crates/ui` and `crates/skybox` (which also depend on `wgpu-types`) get the feature too; it is purely additive (adds `#[derive(Serialize, Deserialize)]` to wgpu-types enums).
- **Delete** `CookedTexture` (`crates/render/src/assets/cooked_texture.rs`, and the `mod cooked_texture` / re-export), `TextureUsageSettings`, `Texture::from_cooked`, `Texture::from_bytes`, `Texture::from_dynamic_image`, `_linear`, `_with_format`. Verified: `TextureUsageSettings`, `from_bytes`, and the `from_dynamic_image*` family have zero callers outside `texture.rs`.
- `Texture::render_target(width, height)` → `Texture { width, height, format: wgpu_types::TextureFormat::Rgba8UnormSrgb, kind: TextureKind::RenderTarget, data: Vec::new() }`.
- Keep accessors: `size() -> wgpu::Extent3d` (built from `width`/`height`), `data() -> &[u8]`. Callers: `crates/render/src/components/camera.rs` (2), `crates/render/src/render_asset/render_texture.rs` (2).
- `impl LoadableAsset for Texture` — `type UsageSettings = ()`; `default_usage_settings()` returns `()`. `TextureLoader` already ignores usage settings for cooked textures.

`crates/render/src/render_asset/render_texture.rs`:

- `RenderTexture::from_texture` builds the `wgpu::TextureDescriptor` and `TextureViewDescriptor` inline from `texture.width/height/format/kind`: `usage = TEXTURE_BINDING | COPY_DST` for `Sampled`, `RENDER_ATTACHMENT | TEXTURE_BINDING` for `RenderTarget`; `label: Some("texture")`; `view_formats: &[]`; default view descriptor. `&'static` labels are unrestricted in render-world code.
- The pixel upload keeps `bytes_per_row = 4 * width` (all cooked formats are 4-byte RGBA8 today). A `TODO` notes that a block-compressed `format` would need `format.block_copy_size(..)`; out of scope now.
- `prepare_asset`'s render-target skip: replace the `usage.contains(RENDER_ATTACHMENT)` sniff with `matches!(source_asset.kind, TextureKind::RenderTarget)`.

`crates/render/src/importers/image_importer.rs` and `crates/gltf-loader/src/gltf_importer.rs`:

- Both currently build `CookedTexture { width, height, srgb, pixels }` and `ctx.emit(name, &cooked)`. Replace with `Texture { width, height, format: if srgb { Rgba8UnormSrgb } else { Rgba8Unorm }, kind: TextureKind::Sampled, data: pixels }`.

`crates/render/src/loaders/texture_loader.rs`:

- `bincode::deserialize::<Texture>(&bytes)` directly; drop `Texture::from_cooked`. The `usage_settings` NOTE comment goes with `from_cooked`.

Tests: `crates/render/tests/image_importer.rs` and `texture_pipeline_e2e.rs` reference `Texture::from_cooked` / `CookedTexture`; update to deserialize `Texture` directly.

### 3. Materials — serde derives

Add `#[derive(serde::Serialize, serde::Deserialize)]` to:

- `crates/ui/src/material.rs` `UIMaterial` (fields: `LinearRgba` ×2, `[f32; 4]`, `f32` — all serde-ready; `LinearRgba` gained serde in the prior plan).
- `crates/skybox/src/material.rs` `SkyboxMaterial` (one `Option<AssetHandle<Texture>>` — identical to `StandardMaterial`'s texture fields).
- `crates/world-grid/src/material.rs` `WorldGridMaterial` and its `WorldGridUniform` field struct (`#[repr(C)] Pod` — adding the derives is compatible).

This is required (they are `#[derive(Asset)]`, so they must satisfy the new supertrait) and also makes them scene-authorable — a bonus, not a goal.

### 4. `AnimationGraph` placeholder

`crates/animation/src/graph.rs` `AnimationGraph` is `#[derive(Asset)]` and lives in `AssetStore<AnimationGraph>`, so it must be `Serialize + DeserializeOwned`. Its field is `DiGraph<Box<dyn AnimationNode>, ()>`, and the node types embed closures:

- `BlendSpace2DNode.sampler: Arc<dyn Fn(&AnimationBlackboard) -> Vec2 + Send + Sync>`
- `AnimationFSMTrigger::Condition(Arc<dyn Fn(&AnimationBlackboard) -> bool + Send + Sync>)`, plus `from_condition()` for arbitrary user predicates

A real serializable form is a standalone redesign (Plan 2). For now, hand-write impls that fail loudly:

```rust
impl serde::Serialize for AnimationGraph {
    fn serialize<S: serde::Serializer>(&self, _: S) -> Result<S::Ok, S::Error> {
        Err(serde::ser::Error::custom(
            "AnimationGraph is not serializable yet — see the Plan 2 follow-up",
        ))
    }
}

impl<'de> serde::Deserialize<'de> for AnimationGraph {
    fn deserialize<D: serde::Deserializer<'de>>(_: D) -> Result<Self, D::Error> {
        Err(serde::de::Error::custom(
            "AnimationGraph is not deserializable yet (tracked: <link>)",
        ))
    }
}
```

Safe in practice: no importer emits an `AnimationGraph`, no loader reads one from cooked bytes, and the arena path (`server.add(graph)`) never serializes. If a future code path *does* try, it errors with a clear message rather than corrupting data. A `// TODO(asset-trait-merge)` on the impls points at the Plan 2 follow-up.

### 5. Call-site sweep

Every `#[derive(Asset)]` / `impl Asset for` site must now also be `Serialize + DeserializeOwned`:

| Type | Crate | Status after this plan |
|---|---|---|
| `Mesh` | mesh | already serde ✓ |
| `Skeleton` | mesh | already serde ✓ |
| `AnimationClip` | animation | already serde ✓ |
| `Scene` | scene | already serde ✓ (overrides `referenced_sub_assets`) |
| `StandardMaterial` | render | already serde ✓ |
| `Texture` | render | serde via §2 |
| `UIMaterial` | ui | serde via §3 |
| `SkyboxMaterial` | skybox | serde via §3 |
| `WorldGridMaterial` | world-grid | serde via §3 |
| `AnimationGraph` | animation | placeholder via §4 |
| `FakeAsset` | essential test | add serde to the test struct |
| cook test fakes | asset-cook tests | convert `impl CookedAsset` → `impl Asset` + serde (§1) |

The `#[derive(Asset)]` proc-macro (`crates/essential/macros/src/lib.rs`) is unchanged — the supertrait bound is enforced at each impl site by the compiler.

## Data flow (unchanged in shape)

Import: `Importer::import` → `ImportContext::emit::<T: Asset>(name, &value)` → `bincode::serialize` → `.cooked/<id>.bin`.
Runtime: `AssetServer::load` → loader → `load_cooked_asset_bytes` → `bincode::deserialize::<T>()` where `T: LoadableAsset` (hence `Asset`, hence `DeserializeOwned`).
Arena: `AssetServer::add::<T: Asset>(value)` → `AssetStore<T>` — never serializes; unaffected except `T` now *could* be serialized.

## Testing

- **New:** `Texture` ↔ `bincode` round-trip — one `Sampled` RGBA8 texture and one `RenderTarget`; assert `width/height/format/kind/data` survive.
- **New:** `AnimationGraph::serialize` and `deserialize` each return `Err` with the placeholder message (documents intent; guards against a silent `#[serde(skip)]`-style regression).
- **Updated:** `crates/render/tests/image_importer.rs`, `texture_pipeline_e2e.rs` — assert against a directly-deserialized `Texture`.
- **Updated:** `asset-cook` tests — fakes now `impl Asset`.
- **Unchanged, must still pass:** all `scene`, `mesh`, `animation` (clip), `gltf-loader` cook/spawn tests — their asset types were already `Serialize + Deserialize`.
- CI gates unchanged: `cargo build --workspace` (zero warnings), `cargo test --workspace`, `cargo fmt --all -- --check`, `cargo clippy -- -A clippy::type_complexity -A clippy::too_many_arguments -D warnings`.

## Risks

- **wgpu-types `serde` feature unification** touches `ui`/`skybox` builds. Mitigation: additive-only feature; a `cargo build --workspace` in the plan's first task confirms.
- **`COOK_FORMAT_VERSION`.** The cooked byte layout of `Texture` changes (`CookedTexture{width,height,srgb,pixels}` → `Texture{width,height,format,kind,data}` — `srgb: bool` becomes a `TextureFormat` enum, plus a `kind` field). Bump `COOK_FORMAT_VERSION` (currently `3` → `4`, in `crates/asset-cook/src/cook.rs`) so any stale `.cooked/` regenerates. Re-cook the three examples (`render-test`, `tech-demo`, `animation-test`) as part of the plan and confirm `errors: 0`.
- **`AnimationGraph` placeholder** could mask a genuine need to serialize it if a future feature assumes all assets round-trip. Mitigation: the loud `Err`, the tracking TODO, and Plan 2 scoped below.

## Plan 2 — serializable `AnimationGraph` (deferred, scoped only)

Its own brainstorm → spec → plan. Sketch:

- Replace `DiGraph<Box<dyn AnimationNode>, ()>` node payloads with a serializable node-kind enum (or `typetag` on `AnimationNode`).
- `BlendSpace2DNode.sampler` → `BlendInput::BlackboardVec2(String)` (the only form used in practice).
- `AnimationFSMTrigger::Condition(Arc<dyn Fn>)` → data variants: `Instant`, `OnAnimationEnd`, `BoolEquals { param: String, value: bool }`, `Vec2NonZero { param: String }`.
- Consequence: the arbitrary-closure escape hatch (`AnimationFSMTrigger::from_condition`, custom samplers) is removed or gated behind a non-serializable, non-cooked builder-only path.
- Then: delete the placeholder impls from §4.
