# One `Asset` Trait + Serializable `AnimationGraph` — Design

**Status:** approved for planning
**Date:** 2026-09-03
**Author:** brainstormed with Claude

## Problem

The engine has two overlapping asset traits:

- `essential::assets::Asset` — `Send + Sync + 'static` with `fn name() -> &'static str`. The runtime marker: `AssetStore<A: Asset>`, `AssetServer::add<A: Asset>`, `App::register_asset<A: Asset>` are generic over it.
- `asset_cook::CookedAsset` — `Serialize + DeserializeOwned` with `const TYPE_NAME: &'static str` and `fn referenced_sub_assets(&self) -> Vec<AssetId>`. What the offline cook pipeline serializes and the cooked-bytes loaders deserialize.

For almost every asset the two are implemented on the *same* type (`Mesh`, `Skeleton`, `AnimationClip`, `Scene`, `StandardMaterial`). Two things stand in the way of one trait:

1. **`Texture`** forces a parallel DTO (`CookedTexture`) because it embeds a `wgpu_types::TextureDescriptor<Option<&'static str>, &'static [TextureFormat]>` (via `TextureUsageSettings`) whose `&'static` references cannot `Deserialize`.
2. **`AnimationGraph`** is `#[derive(Asset)]` (lives in `AssetStore<AnimationGraph>`) but is closure-laden — `DiGraph<Box<dyn AnimationNode>, ()>` with `Arc<dyn Fn>` fields in `BlendSpace2DNode` and `AnimationFSMTrigger`.

## Goal

One trait. `CookedAsset` is deleted; `Asset` absorbs its responsibilities and gains a `Serialize + DeserializeOwned` supertrait bound, so every asset is uniformly serializable and cook-able. The cook `emit` path and the cooked loaders then need no extra trait vocabulary — a bare `T: Asset` bound carries serialization.

To get there, both blockers are removed for real (no placeholders): `Texture` serializes directly, and `AnimationGraph` becomes a data structure.

## Order

The plan runs in two phases. **Phase A first** (serializable `AnimationGraph`) — it is self-contained in the `animation` crate plus two example call sites and needs nothing from Phase B. **Then Phase B** (the trait merge) — by which point every `Asset` type already derives serde, so the supertrait bound lands clean.

---

# Phase A — Serializable `AnimationGraph`

`AnimationGraph` today: `{ graph: DiGraph<Box<dyn AnimationNode>, ()>, result_node: NodeIndex }`. Five node-definition types implement `AnimationNode` (`AnimationResultNode`, `AnimationClipNode`, `AnimationBlendNode`, `BlendSpace2DNode`, `AnimationStateMachine`); two closure surfaces exist, both with trivial real usage:

- `BlendSpace2DNode.sampler: Arc<dyn Fn(&AnimationBlackboard) -> Vec2 + Send + Sync>` — every caller passes `|bb| bb.get_vec2("movement").unwrap_or(ZERO)`.
- `AnimationFSMTrigger::Condition(Arc<dyn Fn(&AnimationBlackboard) -> bool + Send + Sync>)` — only reached through `on_bool(param, value)` / `on_non_zero_vec(param)`. `from_condition` (arbitrary closure) has **zero callers**.

There are **zero `AnimationNode` impls outside the `animation` crate**, so a closed enum loses no real extensibility.

## A1. Node definitions become data (`crates/animation/src/node/`)

Delete the **`AnimationNode` trait**. Node definitions become one enum whose variants wrap small serde structs (keeps the existing builder ergonomics — `AnimationClipNode::new(h).with_play_mode(..)` still works):

```rust
#[derive(Serialize, Deserialize, Clone)]
pub enum AnimationNodeKind {
    Result,
    Clip(AnimationClipNode),                 // { clip: AssetHandle<AnimationClip>, play_mode, start_time }
    Blend,                                    // was AnimationBlendNode
    BlendSpace2D(BlendSpace2DDef),            // { points: Vec<Vec2>, input: BlendInput }
    StateMachine(AnimationStateMachine),
}

#[derive(Serialize, Deserialize, Clone)]
pub struct BlendInput { pub param: String }  // blackboard.get_vec2(&param).unwrap_or(Vec2::ZERO)
```

- `AnimationClipNode` keeps its fields + `new`/`with_start_time`/`with_play_mode`, drops `#[derive(AsAny)]` and its `impl AnimationNode`, gains `#[derive(Serialize, Deserialize, Clone)]`.
- `AnimationPlayMode` gains `Serialize, Deserialize` (plain `enum { Loop, PlayOnce }`).
- `BlendSpace2DDef` replaces `BlendSpace2DNode`'s *definition* role: `{ points: Vec<Vec2>, input: BlendInput }`. The derived `Triangulation2D` is **not** stored on the definition — it moves onto `BlendSpace2DInstanceNode`, built from `points` in `create_instance`.
- `AnimationStateMachine` becomes a plain serde struct (see A2); its `create_instance` logic moves into the enum's dispatch.
- Delete dead `AnimationStateMachineNode` / `AnimationStateMachineNodeState`.

Instance creation moves onto the enum:

```rust
impl AnimationNodeKind {
    pub(crate) fn create_instance(&self, ctx: &AnimationGraphContext) -> Box<dyn AnimationNodeInstance> { /* match */ }
}
```

The **`AnimationNodeInstance` trait stays** (it holds per-playback runtime state — `time`, `blend_stack`, `current_triangulated_point` — that never serializes and is rebuilt each init). Its `evaluate`/`update` signatures change `node: &dyn AnimationNode` → `node: &AnimationNodeKind`; the internal downcasts (`node.as_any().downcast_ref::<AnimationClipNode>()`) become `let AnimationNodeKind::Clip(def) = node else { return; }`.

## A2. Triggers become data (`crates/animation/src/node/state_machine.rs`)

```rust
#[derive(Serialize, Deserialize, Clone)]
pub enum AnimationFSMTrigger {
    Instant,
    OnAnimationEnd,
    BoolEquals { param: String, value: bool },
    Vec2NonZero { param: String },
}
```

- Keep `on_bool(param, value) -> Self::BoolEquals { .. }` and `on_non_zero_vec(param) -> Self::Vec2NonZero { .. }` constructors (call sites unchanged).
- Delete `from_condition` and the `Condition(Arc<dyn Fn>)` variant.
- `AnimationStateMachineInstance::update`'s `match &transition.trigger`: `BoolEquals { param, value } => ctx.blackboard().get_bool(param).is_some_and(|v| v == *value)`, `Vec2NonZero { param } => ctx.blackboard().get_vec2(param).is_some_and(|v| v.length_squared() > f32::EPSILON)`.
- `AnimationStateMachine`, `AnimationFSMState`, `AnimationStateMachineTransition`, `StateId` all derive `Serialize, Deserialize, Clone`. Fields are `String` / `AssetHandle<AnimationGraph>` / `usize` / `f32` / the trigger enum — all serde-ready. State-machine *builders* (`from_initial_state`, `state`, `TransitionBuilder`) keep their `FnOnce` args — those are build-time sugar, never stored.

## A3. Graph becomes serde (`crates/animation/src/graph.rs`)

- `type AnimationDirectedGraph = DiGraph<AnimationNodeKind, ()>`.
- `#[derive(Asset, Serialize, Deserialize)] pub struct AnimationGraph { graph: AnimationDirectedGraph, result_node: AnimationNodeIndex }`.
- `AnimationNodeIndex(NodeIndex)` derives `Serialize, Deserialize` (petgraph's `NodeIndex` is serde under `serde-1`).
- `add_node<T: AnimationNode>` / `add_boxed_node(Box<dyn AnimationNode>)` → `add_node(AnimationNodeKind)`. `get_node(idx) -> Option<&AnimationNodeKind>`. `from_node(kind: AnimationNodeKind)`.
- `crates/animation/Cargo.toml`: `petgraph = { version = "0.8", features = ["serde-1"] }` (the serde feature for petgraph 0.8; `serde` + `derive` are already deps).

## A4. Builder API (`graph.rs`, `node/blend_space.rs`)

- `AnimationNodeContext::with_blend_space_2d_input(param: &str, f: impl FnOnce(&mut BlendSpace2DBuilderContext))` — the sampler closure is gone; `param` builds `BlendInput { param: param.to_string() }`.
- `BlendSpace2DBuilderContext`: drop `sampler: Arc<dyn Fn>`, add `input: BlendInput`; `nodes: Vec<Box<dyn AnimationNode>>` → `Vec<AnimationNodeKind>`; `input(node: AnimationNodeKind, point)` and `animation_clip_input(handle, point)` (builds `AnimationNodeKind::Clip(..)`).
- `AnimationNodeContext::with_input<T: AnimationNode>(node, f)` → `with_input(node: AnimationNodeKind, f)`.

## A5. Call sites

- `examples/tech-demo/src/character.rs`: `with_blend_space_2d_input(|bb| bb.get_vec2("movement")…, |ctx| …)` → `with_blend_space_2d_input("movement", |ctx| …)`; `graph.result_node().with_input(AnimationStateMachine::…build(), …)` → wrap in `AnimationNodeKind::StateMachine(…)`; `AnimationClipNode::new(h).with_play_mode(PlayOnce)` still constructs, now wrapped `AnimationNodeKind::Clip(..)` (or via `From`).
- `examples/animation-test/src/movement_animation.rs`: same `with_blend_space_2d_input` change.
- `crates/animation/src/lib.rs` tests (FSM/graph tests around lines 185–405): update `from_node(AnimationClipNode::new(..))` → `from_node(AnimationNodeKind::Clip(..))`; triggers unchanged (constructors kept).

## A6. Runtime handle note (out of scope, recorded)

When an `AnimationGraph` is built at runtime via `server.add`, its `AnimationClip` / sub-`AnimationGraph` handles are `Strong`. A cooked-then-loaded graph would carry `Weak` handles needing an upgrade pass (as scene components do). No graph is cooked today, so this is deferred; note it where `AnimationNodeKind::Clip` is defined.

## A7. Tests

- **New:** round-trip a non-trivial `AnimationGraph` through `bincode` — a `BlendSpace2D` with three `Clip` inputs feeding a `Result`, plus a nested `StateMachine` with `BoolEquals` / `OnAnimationEnd` transitions. Assert node count, edge set, `result_node`, and trigger payloads survive.
- **Unchanged, must pass:** the existing `crates/animation/src/lib.rs` FSM/graph/blend tests, adapted to the new constructors.

---

# Phase B — Merge `Asset` and `CookedAsset`

## B1. The merged trait

`crates/essential/src/assets/mod.rs`:

```rust
pub trait Asset:
    Send + Sync + 'static + serde::Serialize + serde::de::DeserializeOwned
{
    fn name() -> &'static str;

    /// AssetIds of every sub-asset this one references — consumed by the
    /// cook tool's reference-integrity pass. Empty for leaf assets.
    fn referenced_sub_assets(&self) -> Vec<AssetId> { Vec::new() }
}
```

- `LoadableAsset: Asset` unchanged in shape; inherits the serde bound transitively.
- `essential` already depends on `serde` with `derive`.

`crates/asset-cook/src/lib.rs`: delete the `CookedAsset` trait; use `essential::assets::Asset` where it was referenced (`asset-cook` already depends on `essential`).

`crates/asset-cook/src/import_context.rs`: `emit<T: CookedAsset>` → `emit<T: Asset>`; `type_name: T::TYPE_NAME` → `T::name()`; `value.referenced_sub_assets()` still resolves. `bincode::serialize(value)` compiles from the supertrait.

`TYPE_NAME` → `name()` is safe: every hand-written `TYPE_NAME` (`"Mesh"`, `"Texture"`, `"Skeleton"`, `"AnimationClip"`) equals `stringify!` of the type, which is exactly what `#[derive(Asset)]` emits. The one consumer is the diagnostic label on `EmittedSubAsset`.

`asset-cook` tests (`import_context.rs`, `cook.rs`, `incremental.rs`, `validation.rs`) define fake `impl CookedAsset` — convert each to a `#[derive(Serialize, Deserialize)]` struct + `impl Asset` (with `referenced_sub_assets` where exercised).

## B2. `Texture` rework

`crates/render/src/assets/texture.rs`:

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
pub enum TextureKind { Sampled, RenderTarget }
```

- **Enable the `serde` feature on `wgpu-types`**: `crates/render/Cargo.toml` → `wgpu-types = { version = "24", default-features = false, features = ["serde"] }`. Feature unification means `crates/ui` and `crates/skybox` get it too; it is purely additive.
- **Delete** `CookedTexture` (+ its module and re-export), `TextureUsageSettings`, `Texture::from_cooked`, `from_bytes`, `from_dynamic_image` / `_linear` / `_with_format`. Verified: `TextureUsageSettings`, `from_bytes`, and the `from_dynamic_image*` family have **zero callers** outside `texture.rs`.
- `Texture::render_target(w, h)` → `Texture { width: w, height: h, format: wgpu_types::TextureFormat::Rgba8UnormSrgb, kind: TextureKind::RenderTarget, data: Vec::new() }`.
- Keep `size() -> wgpu::Extent3d` (from `width`/`height`) and `data() -> &[u8]`. Callers: `camera.rs` (2), `render_texture.rs` (2).
- `impl LoadableAsset for Texture` → `type UsageSettings = ()`; `TextureLoader` already ignores usage settings.

`crates/render/src/render_asset/render_texture.rs`:
- `RenderTexture::from_texture` builds `wgpu::TextureDescriptor` / `TextureViewDescriptor` inline from `width/height/format/kind`: `usage = TEXTURE_BINDING | COPY_DST` (`Sampled`) or `RENDER_ATTACHMENT | TEXTURE_BINDING` (`RenderTarget`); `label: Some("texture")`; default view descriptor. `&'static` labels are unrestricted in render-world code.
- Pixel upload keeps `bytes_per_row = 4 * width` (all cooked formats are 4-byte RGBA8 today); `TODO` for block-compressed formats.
- `prepare_asset` render-target skip: `matches!(source_asset.kind, TextureKind::RenderTarget)` instead of the `usage.contains(RENDER_ATTACHMENT)` sniff.

`crates/render/src/importers/image_importer.rs` and `crates/gltf-loader/src/gltf_importer.rs`: build `Texture { .., format: if srgb { Rgba8UnormSrgb } else { Rgba8Unorm }, kind: TextureKind::Sampled, data: pixels }` and `ctx.emit(name, &texture)` instead of `CookedTexture`.

`crates/render/src/loaders/texture_loader.rs`: `bincode::deserialize::<Texture>(&bytes)` directly.

Tests: `crates/render/tests/image_importer.rs`, `texture_pipeline_e2e.rs` — update to the new `Texture`.

## B3. Materials — serde derives

Add `#[derive(serde::Serialize, serde::Deserialize)]` to `UIMaterial` (`crates/ui`), `SkyboxMaterial` (`crates/skybox`), `WorldGridMaterial` + `WorldGridUniform` (`crates/world-grid`). All fields are already serde-ready (`LinearRgba`, `f32`, `[f32; 4]`, `Option<AssetHandle<Texture>>`). Required (they are `#[derive(Asset)]`); also makes them scene-authorable.

## B4. Call-site sweep

Every `#[derive(Asset)]` / `impl Asset for` site must be `Serialize + DeserializeOwned`:

| Type | Crate | After |
|---|---|---|
| `Mesh`, `Skeleton` | mesh | already serde ✓ |
| `AnimationClip` | animation | already serde ✓ |
| `AnimationGraph` | animation | serde via Phase A ✓ |
| `Scene` | scene | already serde ✓ (overrides `referenced_sub_assets`) |
| `StandardMaterial` | render | already serde ✓ |
| `Texture` | render | serde via B2 |
| `UIMaterial` / `SkyboxMaterial` / `WorldGridMaterial` | ui / skybox / world-grid | serde via B3 |
| `FakeAsset`, cook test fakes | essential / asset-cook tests | add serde / convert `impl CookedAsset` → `impl Asset` |

The `#[derive(Asset)]` proc-macro is unchanged; the supertrait bound is enforced at each impl site.

## B5. `COOK_FORMAT_VERSION`

The cooked byte layout of `Texture` changes (`{width,height,srgb,pixels}` → `{width,height,format,kind,data}`). Bump `COOK_FORMAT_VERSION` (`3` → `4`, `crates/asset-cook/src/cook.rs`). Re-cook `render-test`, `tech-demo`, `animation-test`; confirm `errors: 0`.

---

## Data flow (unchanged in shape)

Import: `Importer::import` → `ImportContext::emit::<T: Asset>(name, &value)` → `bincode::serialize` → `.cooked/<id>.bin`.
Runtime load: `AssetServer::load` → loader → `load_cooked_asset_bytes` → `bincode::deserialize::<T>()`, `T: LoadableAsset` (hence `Asset`, hence `DeserializeOwned`).
Arena: `AssetServer::add::<T: Asset>(value)` → `AssetStore<T>` — never serializes.

## Testing

- Phase A: `AnimationGraph` bincode round-trip (A7); existing animation tests adapted.
- Phase B: `Texture` bincode round-trip (a `Sampled` RGBA8 and a `RenderTarget`); updated `image_importer` / `texture_pipeline_e2e` tests; `asset-cook` test fakes on `impl Asset`.
- Unchanged, must pass: all `scene` / `mesh` / `gltf-loader` cook & spawn tests.
- CI gates unchanged: `cargo build --workspace` (zero warnings), `cargo test --workspace`, `cargo fmt --all -- --check`, `cargo clippy -- -A clippy::type_complexity -A clippy::too_many_arguments -D warnings`.
- Visual (per the project recipe): re-run `tech-demo` + `animation-test` under XWayland after Phase A and again after Phase B — the character must still animate (the FSM + blend space are exactly what Phase A rewrites).

## Risks

- **`AnimationNode` trait deletion** touches every node instance's `evaluate`/`update` (`node/mod.rs`, `blend_space.rs`, `state_machine.rs`) plus `graph.rs`, `player.rs`, `evaluation.rs`. Mitigation: `AnimationNodeInstance` (the stateful half) is untouched in shape; the change is mechanical (trait-object + downcast → enum + match); the existing animation tests and the visual check catch regressions.
- **Loss of arbitrary FSM/blend closures** (`from_condition`, custom samplers). Verified zero callers. A future need is met by adding a data variant, not by re-opening the closure hatch.
- **`petgraph` `serde-1` feature** — pulls `serde_derive` and `serde/alloc`. Additive; `cargo build --workspace` in the first Phase-A task confirms.
- **`wgpu-types` `serde` feature unification** across `ui`/`skybox`. Additive; the first Phase-B task's build confirms.
- **`COOK_FORMAT_VERSION` bump** invalidates every consumer's `.cooked/`; the three example re-cooks are in the plan.
- **Runtime→cooked handle upgrade for `AnimationGraph`** (A6) is deferred; safe because nothing cooks a graph yet.
