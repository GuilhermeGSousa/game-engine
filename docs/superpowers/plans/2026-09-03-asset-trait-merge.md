# One `Asset` Trait + Serializable `AnimationGraph` — Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Collapse `Asset` and `CookedAsset` into one `Asset` trait that requires `Serialize + DeserializeOwned`, removing the `CookedTexture` shim and the closure-laden `AnimationGraph` that block it.

**Architecture:** Two phases. **Phase A** turns `AnimationGraph` from `DiGraph<Box<dyn AnimationNode>, ()>` (with `Arc<dyn Fn>` fields) into `DiGraph<AnimationNodeKind, ()>` — a closed data enum; FSM triggers and the blend-space input become data; the graph serializes via petgraph's `serde-1` feature. **Phase B** adds a `Serialize + DeserializeOwned` supertrait to `Asset`, deletes `CookedAsset`, reworks `Texture` to serialize directly (`wgpu_types::TextureFormat` + a `Sampled`/`RenderTarget` kind), adds serde derives to three materials, and bumps the cook format version. Phase A runs first so every `Asset` type already derives serde when the supertrait bound lands.

**Tech Stack:** Rust; `serde` + `bincode`; `petgraph` 0.8 (`serde-1` feature); `wgpu-types` 24 (`serde` feature); the existing `asset-cook` pipeline.

**Spec:** `docs/superpowers/specs/2026-09-03-asset-trait-merge-design.md`

## Global Constraints

- Branch: `asset-store-rework` (this stacks on the current HEAD; do not branch again).
- CI gates, all green with zero warnings, exactly these forms:
  - `cargo build --workspace`
  - `cargo test --workspace`
  - `cargo fmt --all -- --check`
  - `cargo clippy -- -A clippy::type_complexity -A clippy::too_many_arguments -D warnings` (NO `--workspace`, NO `--all-targets`)
- No unnamed tuple structs as data types (single-field newtypes like `AnimationNodeIndex(NodeIndex)` are exempt).
- Lean comments: API docs + non-obvious constraints only.
- Commit message trailer: `Co-Authored-By: Claude Sonnet 5 <noreply@anthropic.com>`.
- Match the style/idiom of each file you touch.
- Phase A must fully land (crate builds, animation tests pass) before any Phase B task starts.

---

# Phase A — Serializable `AnimationGraph`

## Task 1: `AnimationFSMTrigger` becomes data

**Files:**
- Modify: `crates/animation/src/node/state_machine.rs`
- Modify: `crates/animation/src/lib.rs` (tests already use `on_bool` / `OnAnimationEnd` — should need no change; confirm)

**Interfaces:**
- Consumes: `AnimationBlackboard::{get_bool, get_vec2}` (unchanged).
- Produces: `enum AnimationFSMTrigger { Instant, OnAnimationEnd, BoolEquals { param: String, value: bool }, Vec2NonZero { param: String } }` deriving `Serialize, Deserialize, Clone`; constructors `on_bool(param, value)` and `on_non_zero_vec(param)` retained with the same call signatures.

- [ ] **Step 1: Replace the enum and its constructors**

In `crates/animation/src/node/state_machine.rs`, replace:

```rust
pub enum AnimationFSMTrigger {
    Instant,
    OnAnimationEnd,
    Condition(Arc<dyn Fn(&AnimationBlackboard) -> bool + Send + Sync>),
}

impl AnimationFSMTrigger {
    pub fn from_condition<F>(condition: F) -> Self
    where
        F: Fn(&AnimationBlackboard) -> bool + Send + Sync + 'static,
    {
        Self::Condition(Arc::new(condition))
    }

    pub fn on_bool(param_name: impl Into<String>, cond: bool) -> Self {
        let param_name = param_name.into();
        AnimationFSMTrigger::from_condition(move |blackboard| {
            blackboard.get_bool(&param_name).is_some_and(|v| v == cond)
        })
    }

    pub fn on_non_zero_vec(param_name: impl Into<String>) -> Self {
        let param_name = param_name.into();
        AnimationFSMTrigger::from_condition(move |blackboard| {
            blackboard
                .get_vec2(&param_name)
                .is_some_and(|val| val.length_squared() > f32::EPSILON)
        })
    }
}
```

with:

```rust
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub enum AnimationFSMTrigger {
    Instant,
    OnAnimationEnd,
    BoolEquals { param: String, value: bool },
    Vec2NonZero { param: String },
}

impl AnimationFSMTrigger {
    pub fn on_bool(param_name: impl Into<String>, cond: bool) -> Self {
        Self::BoolEquals {
            param: param_name.into(),
            value: cond,
        }
    }

    pub fn on_non_zero_vec(param_name: impl Into<String>) -> Self {
        Self::Vec2NonZero {
            param: param_name.into(),
        }
    }
}
```

- [ ] **Step 2: Update the trigger evaluation site**

In `AnimationStateMachineInstance::update`, replace the `match &transition.trigger` arms:

```rust
match &transition.trigger {
    AnimationFSMTrigger::Instant => {
        self.transition(context, transition);
        break;
    }
    AnimationFSMTrigger::BoolEquals { param, value } => {
        if context.blackboard().get_bool(param).is_some_and(|v| v == *value) {
            self.transition(context, transition);
            break;
        }
    }
    AnimationFSMTrigger::Vec2NonZero { param } => {
        if context
            .blackboard()
            .get_vec2(param)
            .is_some_and(|v| v.length_squared() > f32::EPSILON)
        {
            self.transition(context, transition);
            break;
        }
    }
    AnimationFSMTrigger::OnAnimationEnd => {
        if self
            .state_graph_instances
            .get(self.current_state.as_graph_id())
            .is_some_and(|current_graph| current_graph.is_finished())
        {
            self.transition(context, transition);
            break;
        }
    }
}
```

- [ ] **Step 3: Drop the now-unused import**

Remove `use std::sync::Arc;` from `state_machine.rs` if nothing else in the file uses it (check: `HashMap` import line was `use std::{collections::HashMap, sync::Arc};` → becomes `use std::collections::HashMap;`).

- [ ] **Step 4: Build and test**

Run: `cargo build -p animation && cargo test -p animation`
Expected: PASS. `from_condition` had zero callers, so nothing else breaks.

- [ ] **Step 5: Full gates + commit**

```bash
cargo build --workspace && cargo test --workspace && cargo fmt --all -- --check && cargo clippy -- -A clippy::type_complexity -A clippy::too_many_arguments -D warnings
git add crates/animation
git commit -m "$(cat <<'EOF'
refactor(animation): make AnimationFSMTrigger a data enum

Replaces the Arc<dyn Fn> Condition variant (no callers of from_condition)
with BoolEquals / Vec2NonZero data variants; on_bool / on_non_zero_vec keep
their signatures. Prepares AnimationStateMachine for serialization.

Co-Authored-By: Claude Sonnet 5 <noreply@anthropic.com>
EOF
)"
```

---

## Task 2: Blend-space input becomes data

**Files:**
- Modify: `crates/animation/src/node/blend_space.rs`
- Modify: `crates/animation/src/graph.rs` (the `with_blend_space_2d_input` builder)
- Modify: `examples/tech-demo/src/character.rs`
- Modify: `examples/animation-test/src/movement_animation.rs`

**Interfaces:**
- Consumes: `AnimationBlackboard::get_vec2` (unchanged); `Triangulation2D::build(points: Vec<Vec2>) -> Self`.
- Produces: `struct BlendInput { pub param: String }` deriving `Serialize, Deserialize, Clone`. `BlendSpace2DNode` (still a `#[derive(AsAny)]` node for now — the enum conversion is Task 3) holds `input: BlendInput` instead of `sampler: Arc<dyn Fn>`. Builder: `AnimationNodeContext::with_blend_space_2d_input(param: &str, f: impl FnOnce(&mut BlendSpace2DBuilderContext<'_>))`.

- [ ] **Step 1: Add `BlendInput` and reshape `BlendSpace2DNode`**

In `crates/animation/src/node/blend_space.rs`:

```rust
/// Which blackboard `Vec2` param drives a blend space. Read with
/// `blackboard.get_vec2(&param).unwrap_or(Vec2::ZERO)`.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct BlendInput {
    pub param: String,
}
```

Change `BlendSpace2DNode`:

```rust
#[derive(AsAny)]
pub struct BlendSpace2DNode {
    triangulation: Triangulation2D,
    input: BlendInput,
}

impl BlendSpace2DNode {
    pub(crate) fn new(points: Vec<Vec2>, input: BlendInput) -> Self {
        Self {
            triangulation: Triangulation2D::build(points),
            input,
        }
    }
    // points() / triangles() unchanged
}
```

In `BlendSpace2DInstanceNode::update`, replace `let sample = (blend_space.sampler)(context.blackboard());` with:

```rust
let sample = context
    .blackboard()
    .get_vec2(&blend_space.input.param)
    .unwrap_or(glam::Vec2::ZERO);
```

- [ ] **Step 2: Reshape `BlendSpace2DBuilderContext`**

In `blend_space.rs`, the builder context loses `sampler`, gains `input`:

```rust
pub struct BlendSpace2DBuilderContext<'a> {
    pub(crate) graph: &'a mut AnimationGraph,
    pub(crate) output_node_index: AnimationNodeIndex,
    pub(crate) points: Vec<Vec2>,
    pub(crate) nodes: Vec<Box<dyn AnimationNode>>,
    pub(crate) input: BlendInput,
}
```

`build()` passes `self.input` into `BlendSpace2DNode::new(self.points, self.input)`. Remove `use std::sync::Arc;` if now unused.

- [ ] **Step 3: Change the `with_blend_space_2d_input` signature**

In `crates/animation/src/graph.rs`:

```rust
pub fn with_blend_space_2d_input(
    &mut self,
    param: &str,
    f: impl FnOnce(&mut BlendSpace2DBuilderContext<'_>),
) -> &mut Self {
    let mut builder_context = BlendSpace2DBuilderContext {
        graph: self.graph,
        output_node_index: self.node_index,
        points: Vec::new(),
        nodes: Vec::new(),
        input: crate::node::blend_space::BlendInput { param: param.to_string() },
    };
    f(&mut builder_context);
    builder_context.build();
    self
}
```

Drop the now-unused `use std::sync::Arc;` / `AnimationBlackboard` import in `graph.rs` if they become unused.

- [ ] **Step 4: Update the two example call sites**

`examples/tech-demo/src/character.rs` — `movement_graph.result_node().with_blend_space_2d_input(|blackboard| blackboard.get_vec2("movement").unwrap_or(Vec2::ZERO), |context| { … })` becomes:

```rust
movement_graph.result_node().with_blend_space_2d_input("movement", |context| { … });
```

`examples/animation-test/src/movement_animation.rs` — the same edit (its closure is `|blackboard: &AnimationBlackboard| blackboard.get_vec2("movement").unwrap_or(Vec2::ZERO)`).

Remove any now-unused `AnimationBlackboard` import from those files.

- [ ] **Step 5: Build, test, gates, commit**

```bash
cargo build --workspace && cargo test --workspace && cargo fmt --all -- --check && cargo clippy -- -A clippy::type_complexity -A clippy::too_many_arguments -D warnings
git add crates/animation examples/tech-demo examples/animation-test
git commit -m "$(cat <<'EOF'
refactor(animation): blend-space input is a blackboard param, not a closure

BlendSpace2DNode holds BlendInput { param } instead of Arc<dyn Fn>;
with_blend_space_2d_input takes the param name. Every caller already passed
`|bb| bb.get_vec2("movement").unwrap_or(ZERO)`.

Co-Authored-By: Claude Sonnet 5 <noreply@anthropic.com>
EOF
)"
```

---

## Task 3: `AnimationNode` trait → `AnimationNodeKind` enum

This is the largest task. It removes the last trait object from the graph's node payloads. The `AnimationNodeInstance` trait (per-playback state) is **unchanged in shape** — only the `node: &dyn AnimationNode` parameter it receives becomes `node: &AnimationNodeKind`.

**Files:**
- Modify: `crates/animation/src/node/mod.rs`
- Modify: `crates/animation/src/node/blend_space.rs`
- Modify: `crates/animation/src/node/state_machine.rs`
- Modify: `crates/animation/src/graph.rs`
- Modify: `crates/animation/src/player.rs`
- Modify: `crates/animation/src/lib.rs` (tests)
- Modify: `examples/tech-demo/src/character.rs`

**Interfaces:**
- Consumes: everything from Task 1 (`AnimationFSMTrigger` data enum) and Task 2 (`BlendInput`).
- Produces:
  ```rust
  pub enum AnimationNodeKind {
      Result,
      Clip(AnimationClipNode),
      Blend,
      BlendSpace2D(BlendSpace2DNode),   // now purely a definition: { points: Vec<Vec2>, input: BlendInput }
      StateMachine(AnimationStateMachine),
  }
  impl AnimationNodeKind {
      pub(crate) fn create_instance(&self, ctx: &AnimationGraphContext) -> Box<dyn AnimationNodeInstance>;
  }
  ```
  `AnimationGraph::{add_node(AnimationNodeKind, output) , get_node(idx) -> Option<&AnimationNodeKind>, from_node(AnimationNodeKind)}`. `AnimationNodeInstance::{evaluate, update}` take `node: &AnimationNodeKind`.

- [ ] **Step 1: Define `AnimationNodeKind` and move `create_instance` onto it**

In `crates/animation/src/node/mod.rs`:

- Delete the `AnimationNode` trait.
- Keep `AnimationClipNode` (drop `#[derive(AsAny)]`; keep `new` / `with_start_time` / `with_play_mode`). Add `#[derive(Clone)]`.
- Delete `AnimationResultNode`, `AnimationBlendNode`, `AnimationStateMachineNode`, `AnimationStateMachineNodeState` (dead / replaced by enum variants).
- Add:

```rust
pub enum AnimationNodeKind {
    Result,
    Clip(AnimationClipNode),
    Blend,
    BlendSpace2D(crate::node::blend_space::BlendSpace2DNode),
    StateMachine(crate::node::state_machine::AnimationStateMachine),
}

impl AnimationNodeKind {
    pub(crate) fn create_instance(
        &self,
        creation_context: &AnimationGraphContext,
    ) -> Box<dyn AnimationNodeInstance> {
        match self {
            AnimationNodeKind::Result | AnimationNodeKind::Blend => Box::new(NoneInstance),
            AnimationNodeKind::Clip(def) => {
                Box::new(AnimationClipNodeInstance::new().with_start_time(def.start_time))
            }
            AnimationNodeKind::BlendSpace2D(def) => {
                Box::new(crate::node::blend_space::BlendSpace2DInstanceNode::new(def.points()))
            }
            AnimationNodeKind::StateMachine(fsm) => fsm.create_instance_boxed(creation_context),
        }
    }
}
```

(`AnimationClipNode.start_time` is private but same-module in `node/mod.rs`, so `def.start_time` is fine.)

- [ ] **Step 2: `AnimationClipNodeInstance` reads the enum**

In `AnimationClipNodeInstance::evaluate` and `::update`, change the parameter to `node: &AnimationNodeKind` and replace `node.as_any().downcast_ref::<AnimationClipNode>()` with:

```rust
let AnimationNodeKind::Clip(clip_node) = node else {
    return;
};
```

Then `clip_node.clip`, `clip_node.play_mode` as before.

- [ ] **Step 3: `BlendSpace2DNode` becomes a definition; triangulation moves to the instance**

In `crates/animation/src/node/blend_space.rs`:

```rust
pub struct BlendSpace2DNode {
    points: Vec<Vec2>,
    input: BlendInput,
}

impl BlendSpace2DNode {
    pub(crate) fn new(points: Vec<Vec2>, input: BlendInput) -> Self {
        Self { points, input }
    }
    pub fn points(&self) -> &[Vec2] { &self.points }
    // triangles() is gone from the definition; the instance owns triangulation
}

#[derive(AsAny)]
pub struct BlendSpace2DInstanceNode {
    triangulation: Triangulation2D,
    current_triangulated_point: Option<TriangulatedPoint2D>,
}

impl BlendSpace2DInstanceNode {
    pub(crate) fn new(points: &[Vec2]) -> Self {
        Self {
            triangulation: Triangulation2D::build(points.to_vec()),
            current_triangulated_point: None,
        }
    }
}
```

`BlendSpace2DInstanceNode::update`: `node: &AnimationNodeKind`; `let AnimationNodeKind::BlendSpace2D(def) = node else { return; };`; `let sample = context.blackboard().get_vec2(&def.input.param).unwrap_or(Vec2::ZERO);`; `self.current_triangulated_point = Some(self.triangulation.locate_or_nearest(sample));`.
`BlendSpace2DInstanceNode::evaluate`: `node: &AnimationNodeKind`; `let AnimationNodeKind::BlendSpace2D(def) = node else { return; };`; use `self.triangulation.triangles()` and `def.points().len()` for the input-count check; the rest of the barycentric blend is unchanged.
Remove `impl Default for BlendSpace2DInstanceNode` (replaced by `new`), or keep `Default` producing an empty triangulation — prefer `new`.

- [ ] **Step 4: `AnimationStateMachine` gets an instance builder, stops being a node**

In `crates/animation/src/node/state_machine.rs`:

- Delete `impl AnimationNode for AnimationStateMachine`.
- Add the instance construction as an inherent method (the old `create_instance` body):

```rust
impl AnimationStateMachine {
    pub(crate) fn create_instance_boxed(
        &self,
        creation_context: &AnimationGraphContext,
    ) -> Box<dyn AnimationNodeInstance> {
        let mut instanced_internal_graphs = Vec::new();
        let mut state_names = Vec::new();
        for fsm_state in &self.states {
            let mut instanced_internal_graph = AnimationGraphInstance::default();
            instanced_internal_graph.initialize(fsm_state.graph.clone(), creation_context);
            instanced_internal_graphs.push(instanced_internal_graph);
            state_names.push(fsm_state.name.clone());
        }
        Box::new(AnimationStateMachineInstance::new(
            self.initial_state,
            instanced_internal_graphs,
            state_names,
        ))
    }
}
```

- `AnimationStateMachineInstance::update` / `::evaluate` take `node: &AnimationNodeKind`. `update` currently does `node.as_any().downcast_ref::<AnimationStateMachine>()` → `let AnimationNodeKind::StateMachine(fsm) = node else { return; };`. `evaluate` ignores `node` (`_node`), leave it.
- Remove `use std::any::Any;` / `AsAny` derive from `AnimationStateMachine` if now unused (the *instance* keeps `#[derive(AsAny)]`).

- [ ] **Step 5: `graph.rs` — the graph holds `AnimationNodeKind`**

In `crates/animation/src/graph.rs`:

```rust
type AnimationDirectedGraph = DiGraph<AnimationNodeKind, ()>;
```

- `AnimationGraph::new()` → `graph.add_node(AnimationNodeKind::Result)`.
- `from_node(node: AnimationNodeKind)`; `add_node(&mut self, node: AnimationNodeKind, output_node)`; delete `add_boxed_node` (fold into `add_node`).
- `get_node(&self, idx) -> Option<&AnimationNodeKind> { self.graph.node_weight(*idx) }` (no `.deref()`).
- `AnimationGraphInstance::initialize` (line ~216): `graph.get_node(node_index)` now yields `&AnimationNodeKind`; `anim_node.create_instance(creation_context)` still valid (inherent method).
- `AnimationGraphInstance::update` / `evaluate` (lines ~245, ~267): `graph.get_node(...)` → `&AnimationNodeKind`, passed straight into `node_state.node_instance.update(node, …)` / `.evaluate(node, …)`.
- `AnimationNodeContext::with_input`:

```rust
pub fn with_input(
    &mut self,
    node: AnimationNodeKind,
    f: impl FnOnce(AnimationNodeContext<'_>),
) -> &mut Self {
    f(self.graph.add_node(node, self.node_index));
    self
}
```

- `player.rs`: `ActiveNodeInstance::update(&mut self, node: &AnimationNodeKind, …)` and its one call site.
- `BlendSpace2DBuilderContext`: `nodes: Vec<AnimationNodeKind>`; `input(node: AnimationNodeKind, point)`; `animation_clip_input(clip, point)` pushes `AnimationNodeKind::Clip(AnimationClipNode::new(clip))`; `build()` calls `self.graph.add_node(...)` for the blend-space node and each input.

- [ ] **Step 6: Update tests and the tech-demo state machine**

`crates/animation/src/lib.rs` tests: `AnimationGraph::from_node(AnimationClipNode::new(x))` → `AnimationGraph::from_node(AnimationNodeKind::Clip(AnimationClipNode::new(x)))` (add a `use` for `AnimationNodeKind`). Triggers unchanged.

`examples/tech-demo/src/character.rs`: `graph.result_node().with_input(AnimationStateMachine::from_initial_state(…).…build(), |node_context| { … })` → `…with_input(AnimationNodeKind::StateMachine(AnimationStateMachine::from_initial_state(…).…build()), |node_context| { … })`. The commented-out `AnimationClipNode::new(_jump_start).with_play_mode(PlayOnce)` inside `AnimationGraph::from_node(...)` calls elsewhere in the file → wrap in `AnimationNodeKind::Clip(...)`. `AnimationClipNode::new(jump_loop)` in `AnimationGraph::from_node(AnimationClipNode::new(jump_loop))` → `AnimationNodeKind::Clip(AnimationClipNode::new(jump_loop))`.

- [ ] **Step 7: Build, test, gates, commit**

```bash
cargo build --workspace && cargo test --workspace && cargo fmt --all -- --check && cargo clippy -- -A clippy::type_complexity -A clippy::too_many_arguments -D warnings
git add crates/animation examples/tech-demo
git commit -m "$(cat <<'EOF'
refactor(animation)!: replace the AnimationNode trait with a data enum

Node definitions are now a closed AnimationNodeKind enum; the graph is
DiGraph<AnimationNodeKind, ()>. AnimationNodeInstance (per-playback state)
is unchanged in shape — it just receives &AnimationNodeKind instead of
&dyn AnimationNode. BlendSpace2D's triangulation moves onto its instance.

Co-Authored-By: Claude Sonnet 5 <noreply@anthropic.com>
EOF
)"
```

---

## Task 4: Serialize the graph

**Files:**
- Modify: `crates/animation/Cargo.toml`
- Modify: `crates/animation/src/graph.rs`
- Modify: `crates/animation/src/node/mod.rs`, `node/blend_space.rs`, `node/state_machine.rs`
- Create: `crates/animation/tests/graph_serialization.rs`

**Interfaces:**
- Consumes: `AnimationNodeKind` (Task 3), `AnimationFSMTrigger` (Task 1), `BlendInput` (Task 2).
- Produces: `AnimationGraph: serde::Serialize + serde::de::DeserializeOwned` (still `#[derive(Asset)]`); a `bincode` round-trip test.

- [ ] **Step 1: Enable petgraph's serde feature**

`crates/animation/Cargo.toml`: `petgraph = { version = "0.8", features = ["serde-1"] }`.

- [ ] **Step 2: Derive serde across the node types**

Add `serde::Serialize, serde::Deserialize` (alongside existing derives) to:
- `crates/animation/src/node/mod.rs`: `AnimationNodeKind`, `AnimationClipNode`, `AnimationPlayMode` (already `#[derive(Default)]` → add serde).
- `crates/animation/src/node/blend_space.rs`: `BlendSpace2DNode` (the definition — `{ points: Vec<Vec2>, input: BlendInput }`), `BlendInput` (already has it from Task 2).
- `crates/animation/src/node/state_machine.rs`: `AnimationStateMachine`, `AnimationFSMState`, `AnimationStateMachineTransition`, `StateId`. `StateId(usize)` is a single-field newtype — `#[derive(Clone, Copy, Deref, serde::Serialize, serde::Deserialize)]`. `AnimationFSMState` fields (`name: String`, `graph: AssetHandle<AnimationGraph>`) and `AnimationStateMachineTransition` fields (`next_state: StateId`, `trigger: AnimationFSMTrigger`, `transition_time: f32`) are all serde-ready.

- [ ] **Step 3: Derive serde on `AnimationGraph` and `AnimationNodeIndex`**

`crates/animation/src/graph.rs`:

```rust
#[derive(Clone, Copy, Eq, Hash, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct AnimationNodeIndex(NodeIndex);

#[derive(Asset, serde::Serialize, serde::Deserialize)]
pub struct AnimationGraph {
    graph: AnimationDirectedGraph,
    result_node: AnimationNodeIndex,
}
```

Add a `// TODO(asset-trait-merge): a cooked-then-loaded graph carries Weak
AssetHandles; needs an upgrade pass like scene components. No graph is
cooked yet.` near `AnimationNodeKind::Clip`.

- [ ] **Step 4: Write the round-trip test**

Create `crates/animation/tests/graph_serialization.rs`:

```rust
use animation::clip::AnimationClip;
use animation::graph::AnimationGraph;
use animation::node::AnimationNodeKind;
use animation::node::blend_space::BlendInput;
use essential::assets::handle::AssetHandle;
use essential::assets::AssetId;

#[test]
fn animation_graph_round_trips_through_bincode() {
    let clip = |name: &str| AssetHandle::<AnimationClip>::weak(AssetId::from_path(name));

    let mut graph = AnimationGraph::new();
    graph.result_node().with_blend_space_2d_input("movement", |ctx| {
        ctx.animation_clip_input(clip("idle"), glam::Vec2::ZERO)
            .animation_clip_input(clip("walk"), glam::Vec2::new(0.0, 1.0))
            .animation_clip_input(clip("strafe"), glam::Vec2::new(1.0, 0.0));
    });

    let bytes = bincode::serialize(&graph).expect("serialize");
    let restored: AnimationGraph = bincode::deserialize(&bytes).expect("deserialize");

    let count = |g: &AnimationGraph| g.iter().count();
    assert_eq!(count(&restored), count(&graph), "node count survives");

    // Blackboard param on the blend-space node survives.
    let has_movement_input = restored.iter().any(|idx| {
        matches!(
            restored.get_node(idx),
            Some(AnimationNodeKind::BlendSpace2D(def)) if def.input().param == "movement"
        )
    });
    assert!(has_movement_input, "BlendSpace2D input param survived the round trip");
}
```

Add a `pub fn input(&self) -> &BlendInput` accessor to `BlendSpace2DNode` if the test needs it (or make `input` `pub`). Add whatever `pub use` is needed so `animation::node::AnimationNodeKind` and `animation::node::blend_space::BlendInput` resolve.

- [ ] **Step 5: Run the test**

Run: `cargo test -p animation --test graph_serialization`
Expected: PASS.

- [ ] **Step 6: Full gates + commit**

```bash
cargo build --workspace && cargo test --workspace && cargo fmt --all -- --check && cargo clippy -- -A clippy::type_complexity -A clippy::too_many_arguments -D warnings
git add crates/animation
git commit -m "$(cat <<'EOF'
feat(animation): AnimationGraph serializes via serde

DiGraph<AnimationNodeKind, ()> round-trips through bincode (petgraph
serde-1 feature). Every node kind, FSM trigger, and blend input is data.

Co-Authored-By: Claude Sonnet 5 <noreply@anthropic.com>
EOF
)"
```

---

## Task 5: Visual regression check (Phase A)

**Files:** none (verification only).

- [ ] **Step 1: Re-cook and run both animated examples**

```bash
cargo run -p cook -- examples/animation-test/assets.toml examples/animation-test/assets examples/animation-test/res
cargo run -p cook -- examples/tech-demo/assets.toml examples/tech-demo/assets examples/tech-demo/res
```

Then, per the project's visual-verification recipe (XWayland + a small `xshot`), run each example and capture 2–3 frames a few seconds apart:

```bash
env -u WAYLAND_DISPLAY DISPLAY=:1 ./target/debug/animation-test &
env -u WAYLAND_DISPLAY DISPLAY=:1 ./target/debug/tech-demo &
```

Expected: `animation-test`'s ninja plays its idle/blend animation (authored pose, frames advance); `tech-demo`'s mannequin is grounded and animating (not a bind-pose T). If either stands still, the FSM/blend-space rewrite regressed — investigate before Phase B. Record the outcome in the ledger; no commit.

---

# Phase B — Merge `Asset` and `CookedAsset`

## Task 6: Make every asset type serialize (still two traits)

Lands `Texture`'s new shape and the material derives under the *existing* `Asset` / `CookedAsset` split, so Phase B's trait change (Task 7) is pure trait plumbing.

**Files:**
- Modify: `crates/render/Cargo.toml`
- Modify: `crates/render/src/assets/texture.rs`
- Delete: `crates/render/src/assets/cooked_texture.rs`
- Modify: `crates/render/src/assets/mod.rs`
- Modify: `crates/render/src/render_asset/render_texture.rs`
- Modify: `crates/render/src/components/camera.rs` (`texture.size()` now returns an owned `wgpu::Extent3d`, not `&`; the 2 call sites deref accordingly)
- Modify: `crates/render/src/importers/image_importer.rs`
- Modify: `crates/render/src/loaders/texture_loader.rs`
- Modify: `crates/gltf-loader/src/gltf_importer.rs`
- Modify: `crates/render/tests/image_importer.rs`, `crates/render/tests/texture_pipeline_e2e.rs`
- Modify: `crates/ui/src/material.rs`, `crates/skybox/src/material.rs`, `crates/world-grid/src/material.rs`

**Interfaces:**
- Consumes: `wgpu_types::TextureFormat`, `TextureUsages`, `Extent3d`.
- Produces:
  ```rust
  #[derive(Asset, serde::Serialize, serde::Deserialize)]
  pub struct Texture { pub width: u32, pub height: u32, pub format: wgpu_types::TextureFormat, pub kind: TextureKind, pub data: Vec<u8> }
  #[derive(serde::Serialize, serde::Deserialize, Clone, Copy, PartialEq, Eq, Debug)]
  pub enum TextureKind { Sampled, RenderTarget }
  impl Texture { fn render_target(w: u32, h: u32) -> Self; fn size(&self) -> wgpu::Extent3d; fn data(&self) -> &[u8]; }
  ```
  A temporary `impl CookedAsset for Texture { const TYPE_NAME = "Texture"; }` (removed in Task 7). `UIMaterial` / `SkyboxMaterial` / `WorldGridMaterial` derive serde.

- [ ] **Step 1: Enable `wgpu-types` serde**

`crates/render/Cargo.toml`: `wgpu-types = { version = "24", default-features = false, features = ["serde"] }`.
Run `cargo build --workspace` — confirm `ui` / `skybox` still build with the unified feature. Expected: PASS.

- [ ] **Step 2: Rewrite `Texture`**

Replace the `Texture` struct, `TextureUsageSettings`, and the constructor family in `crates/render/src/assets/texture.rs` with:

```rust
use asset_cook::CookedAsset;
use essential::assets::{Asset, LoadableAsset};
use crate::loaders::texture_loader::TextureLoader;

#[derive(serde::Serialize, serde::Deserialize, Clone, Copy, PartialEq, Eq, Debug)]
pub enum TextureKind {
    Sampled,
    RenderTarget,
}

#[derive(Asset, serde::Serialize, serde::Deserialize)]
pub struct Texture {
    pub width: u32,
    pub height: u32,
    pub format: wgpu_types::TextureFormat,
    pub kind: TextureKind,
    /// RGBA8 pixels matching `format`. Empty for `TextureKind::RenderTarget`.
    // TODO(asset-trait-merge): a cooked-then-loaded Texture handle is Weak;
    // block-compressed formats will need format.block_copy_size() at upload.
    pub data: Vec<u8>,
}

impl Texture {
    /// A GPU-only render target; the camera system allocates the wgpu texture.
    pub fn render_target(width: u32, height: u32) -> Self {
        Self {
            width,
            height,
            format: wgpu_types::TextureFormat::Rgba8UnormSrgb,
            kind: TextureKind::RenderTarget,
            data: Vec::new(),
        }
    }

    pub fn size(&self) -> wgpu::Extent3d {
        wgpu::Extent3d {
            width: self.width,
            height: self.height,
            depth_or_array_layers: 1,
        }
    }

    pub fn data(&self) -> &[u8] {
        &self.data
    }
}

impl LoadableAsset for Texture {
    type UsageSettings = ();
    fn loader() -> Box<dyn essential::assets::asset_loader::AssetLoader<Asset = Self>> {
        Box::new(TextureLoader)
    }
    fn default_usage_settings() -> Self::UsageSettings {}
}

// Removed in Task 7 when `emit` switches to `T: Asset`.
impl CookedAsset for Texture {
    const TYPE_NAME: &'static str = "Texture";
}
```

Delete `crates/render/src/assets/cooked_texture.rs` and its `pub mod cooked_texture;` line in `crates/render/src/assets/mod.rs`. Grep the crate for `CookedTexture` / `cooked_texture` and remove every remaining reference.

- [ ] **Step 3: `RenderTexture::from_texture` builds its own descriptor**

`crates/render/src/render_asset/render_texture.rs` — replace the `usage_settings()` reads:

```rust
pub fn from_texture(texture: &Texture, device: &wgpu::Device, queue: &wgpu::Queue) -> Self {
    let usage = match texture.kind {
        TextureKind::Sampled => TextureUsages::TEXTURE_BINDING | TextureUsages::COPY_DST,
        TextureKind::RenderTarget => {
            TextureUsages::RENDER_ATTACHMENT | TextureUsages::TEXTURE_BINDING
        }
    };
    let size = texture.size();
    let wgpu_texture = device.create_texture(&wgpu::TextureDescriptor {
        label: Some("texture"),
        size,
        mip_level_count: 1,
        sample_count: 1,
        dimension: wgpu::TextureDimension::D2,
        format: texture.format,
        usage,
        view_formats: &[],
    });
    let view = wgpu_texture.create_view(&wgpu::TextureViewDescriptor::default());

    if !texture.data().is_empty() {
        queue.write_texture(
            wgpu::TexelCopyTextureInfo {
                aspect: wgpu::TextureAspect::All,
                texture: &wgpu_texture,
                mip_level: 0,
                origin: wgpu::Origin3d::ZERO,
            },
            texture.data(),
            wgpu::TexelCopyBufferLayout {
                offset: 0,
                bytes_per_row: Some(4 * size.width),
                rows_per_image: Some(size.height),
            },
            size,
        );
    }
    // sampler unchanged
    …
}
```

In `prepare_asset`, replace the RTT sniff:

```rust
if matches!(source_asset.kind, crate::assets::texture::TextureKind::RenderTarget) {
    return Err(AssetPreparationError::NotReady);
}
```

Remove the now-unused `use wgpu::TextureUsages;` only if nothing else in the file needs it (it does — keep).

- [ ] **Step 4: Importers emit `Texture`; loader deserializes it**

`crates/render/src/importers/image_importer.rs`:

```rust
use crate::assets::texture::{Texture, TextureKind};
…
let texture = Texture {
    width,
    height,
    format: wgpu_types::TextureFormat::Rgba8UnormSrgb, // TODO(asset-import-pipeline): colour space
    kind: TextureKind::Sampled,
    data: img.to_rgba8().into_raw(),
};
ctx.emit("main", &texture).map_err(…)?;
```

`crates/gltf-loader/src/gltf_importer.rs` — where it currently builds `CookedTexture { width, height, srgb, pixels }` and `ctx.emit(&texture_name(...), &cooked)`:

```rust
let texture = Texture {
    width: rgba.width(),
    height: rgba.height(),
    format: if key.srgb {
        wgpu_types::TextureFormat::Rgba8UnormSrgb
    } else {
        wgpu_types::TextureFormat::Rgba8Unorm
    },
    kind: TextureKind::Sampled,
    data: rgba.into_raw(),
};
ctx.emit(&texture_name(key.image_index, key.srgb), &texture)?;
```

Update the `use` (`render::assets::cooked_texture::CookedTexture` → `render::assets::texture::{Texture, TextureKind}`). `gltf-loader` already depends on `render` and `wgpu-types`? — if `wgpu-types` isn't a direct dep, use `render`'s re-export or add it; prefer a `render::assets::texture::TextureFormat` re-export to avoid a new dep. **Decision:** add `pub use wgpu_types::TextureFormat;` to `crates/render/src/assets/texture.rs` and reference `render::assets::texture::TextureFormat` from the importer.

`crates/render/src/loaders/texture_loader.rs`: replace `Ok(Texture::from_cooked(cooked))` and the `bincode::deserialize::<CookedTexture>` with `let texture: Texture = bincode::deserialize(&bytes).with_context(|| "failed to deserialize cooked texture")?; Ok(texture)`. Drop the stale `usage_settings` NOTE comment.

- [ ] **Step 5: Material serde derives**

Add `#[derive(serde::Serialize, serde::Deserialize)]` to `UIMaterial` (`crates/ui/src/material.rs`), `SkyboxMaterial` (`crates/skybox/src/material.rs`), and `WorldGridMaterial` + `WorldGridUniform` (`crates/world-grid/src/material.rs`).

- [ ] **Step 6: Fix the texture tests**

`crates/render/tests/image_importer.rs` and `crates/render/tests/texture_pipeline_e2e.rs`: replace `Texture::from_cooked(cooked)` / `CookedTexture { … }` with direct `bincode::deserialize::<Texture>(&entry.bytes)` and assert on `width`/`height`/`format`/`kind`/`data`.

- [ ] **Step 7: Build, test, cook smoke, gates, commit**

```bash
cargo build --workspace && cargo test --workspace && cargo fmt --all -- --check && cargo clippy -- -A clippy::type_complexity -A clippy::too_many_arguments -D warnings
cargo run -p cook -- examples/render-test/assets.toml examples/render-test/assets examples/render-test/res   # errors: 0
git add crates/render crates/gltf-loader crates/ui crates/skybox crates/world-grid
git commit -m "$(cat <<'EOF'
refactor(render): Texture serializes directly; drop CookedTexture

Texture is { width, height, format: wgpu TextureFormat, kind, data }.
RenderTexture builds its own wgpu descriptor. Three AsBindGroup materials
gain serde derives. Still under the Asset/CookedAsset split — the merge is
the next commit.

Co-Authored-By: Claude Sonnet 5 <noreply@anthropic.com>
EOF
)"
```

---

## Task 7: Collapse the traits

**Files:**
- Modify: `crates/essential/src/assets/mod.rs`
- Modify: `crates/asset-cook/src/lib.rs`
- Modify: `crates/asset-cook/src/import_context.rs`
- Modify: `crates/asset-cook/src/cook.rs`
- Modify: `crates/asset-cook/tests/{import_context,cook,incremental,validation}.rs`
- Modify: `crates/essential/tests/asset_handle.rs`
- Modify: `crates/mesh/src/mesh.rs`, `crates/mesh/src/skeleton.rs`, `crates/animation/src/clip.rs`
- Modify: `crates/scene/src/scene.rs`, `crates/render/src/assets/material.rs`
- Modify: `crates/render/src/assets/texture.rs` (drop the temp `impl CookedAsset`)

**Interfaces:**
- Consumes: every `Asset` type now derives `Serialize + DeserializeOwned` (Phase A + Task 6).
- Produces:
  ```rust
  pub trait Asset: Send + Sync + 'static + serde::Serialize + serde::de::DeserializeOwned {
      fn name() -> &'static str;
      fn referenced_sub_assets(&self) -> Vec<AssetId> { Vec::new() }
  }
  ```
  `CookedAsset` deleted. `ImportContext::emit<T: Asset>`.

- [ ] **Step 1: Change the `Asset` trait**

`crates/essential/src/assets/mod.rs`:

```rust
pub trait Asset:
    Send + Sync + 'static + serde::Serialize + serde::de::DeserializeOwned
{
    fn name() -> &'static str;

    /// AssetIds of every sub-asset this one references — the cook tool's
    /// reference-integrity pass. Empty for leaf assets.
    fn referenced_sub_assets(&self) -> Vec<AssetId> {
        Vec::new()
    }
}
```

- [ ] **Step 2: Delete `CookedAsset`, switch `emit`**

`crates/asset-cook/src/lib.rs`: delete the `CookedAsset` trait and the `use serde::{de::DeserializeOwned, Serialize};` if now unused. Add `use essential::assets::Asset;` where needed.

`crates/asset-cook/src/import_context.rs`:

```rust
pub fn emit<T: Asset>(&mut self, name: &str, value: &T) -> Result<(), ImportError> {
    let bytes = bincode::serialize(value).map_err(|err| ImportError::SerializationFailed {
        sub_asset_name: name.to_string(),
        message: err.to_string(),
    })?;
    self.sub_assets.push(EmittedSubAsset {
        name: name.to_string(),
        asset_id: self.sub_asset_id(name),
        type_name: T::name(),
        bytes,
        references: value.referenced_sub_assets(),
    });
    Ok(())
}
```

- [ ] **Step 3: Bump the cook format version**

`crates/asset-cook/src/cook.rs`: `pub const COOK_FORMAT_VERSION: u32 = 4;`.

- [ ] **Step 4: Convert the `impl CookedAsset` blocks**

- `crates/mesh/src/mesh.rs`, `crates/mesh/src/skeleton.rs`, `crates/animation/src/clip.rs`, `crates/render/src/assets/texture.rs`: **delete** the `impl CookedAsset for X { const TYPE_NAME = …; }` block entirely. `X` already `#[derive(Asset, Serialize, Deserialize)]`; the merged `Asset` covers it.
- `crates/scene/src/scene.rs`: `Scene` overrides `referenced_sub_assets`, so it can't use `#[derive(Asset)]`. Change `#[derive(Asset, Debug, Clone, Serialize, Deserialize)]` → `#[derive(Debug, Clone, Serialize, Deserialize)]` and add:

```rust
impl Asset for Scene {
    fn name() -> &'static str {
        "Scene"
    }
    fn referenced_sub_assets(&self) -> Vec<AssetId> {
        self.referenced_assets.clone()
    }
}
```

Delete the `impl CookedAsset for Scene` block.
- `crates/render/src/assets/material.rs`: `StandardMaterial` also overrides `referenced_sub_assets`. Same treatment: drop `Asset` from the derive list, add a hand `impl Asset for StandardMaterial { fn name() -> &'static str { "StandardMaterial" } fn referenced_sub_assets(&self) -> Vec<AssetId> { … } }` (move the body from the old `impl CookedAsset`). Delete the `impl CookedAsset` block.

- [ ] **Step 5: Update the test fakes**

`crates/asset-cook/tests/{import_context,cook,incremental,validation}.rs`: each `impl CookedAsset for FakeThing { const TYPE_NAME = "…"; [fn referenced_sub_assets …] }` becomes `#[derive(serde::Serialize, serde::Deserialize)] struct FakeThing { … }` + `impl essential::assets::Asset for FakeThing { fn name() -> &'static str { "…" } [fn referenced_sub_assets …] }`. Update the `use asset_cook::{CookedAsset, …}` imports.

`crates/essential/tests/asset_handle.rs`: `struct FakeAsset;` → `#[derive(serde::Serialize, serde::Deserialize)] struct FakeAsset;` and keep `impl Asset for FakeAsset { fn name() -> &'static str { "FakeAsset" } }`.

- [ ] **Step 6: Build, test, gates, commit**

```bash
cargo build --workspace && cargo test --workspace && cargo fmt --all -- --check && cargo clippy -- -A clippy::type_complexity -A clippy::too_many_arguments -D warnings
git add crates
git commit -m "$(cat <<'EOF'
refactor!: merge CookedAsset into Asset

Asset now requires Serialize + DeserializeOwned and carries the defaulted
referenced_sub_assets(); CookedAsset is deleted and ImportContext::emit
takes a bare T: Asset. TYPE_NAME is replaced by Asset::name(). Scene and
StandardMaterial hand-impl Asset to keep their referenced_sub_assets
override. COOK_FORMAT_VERSION -> 4.

Co-Authored-By: Claude Sonnet 5 <noreply@anthropic.com>
EOF
)"
```

---

## Task 8: Re-cook, verify, close out

**Files:** none (verification only), unless a fix is needed.

- [ ] **Step 1: Re-cook all three examples**

```bash
rm -rf examples/render-test/res examples/tech-demo/res examples/animation-test/res
cargo run -p cook -- examples/render-test/assets.toml examples/render-test/assets examples/render-test/res
cargo run -p cook -- examples/tech-demo/assets.toml examples/tech-demo/assets examples/tech-demo/res
cargo run -p cook -- examples/animation-test/assets.toml examples/animation-test/assets examples/animation-test/res
```

Expected: `errors: 0` for all three.

- [ ] **Step 2: Full workspace gates**

```bash
cargo build --workspace && cargo test --workspace && cargo fmt --all -- --check && cargo clippy -- -A clippy::type_complexity -A clippy::too_many_arguments -D warnings
```

Expected: all green, zero warnings.

- [ ] **Step 3: Visual check**

Per the project recipe (XWayland + `xshot`), run `render-test`, `animation-test`, `tech-demo`. Expected: Sponza renders (textures intact — the `Texture` format round-trip is what this proves); both characters animate. Record outcomes in the ledger. No commit unless a regression forces a fix.

---

## Self-Review Notes (author)

- **Spec coverage:** Phase A ⇒ spec §A1–A7; Phase B ⇒ spec §B1–B5 + risks. The deferred runtime→cooked handle upgrade (spec §A6) is a `TODO` in Task 4 / Task 6, not a task.
- **Type consistency:** `AnimationNodeKind` variants named identically across Task 3/Task 4 (`Result`, `Clip`, `Blend`, `BlendSpace2D`, `StateMachine`). `TextureKind` = `{ Sampled, RenderTarget }` in Task 6 and referenced by the same path in Task 7. `emit<T: Asset>` introduced in Task 7 matches the `T::name()` / `referenced_sub_assets()` on the merged trait from Task 7 Step 1.
- **Ordering hazard:** Task 6 keeps a temporary `impl CookedAsset for Texture`; Task 7 Step 4 deletes it. If Task 7 is executed without Task 6, `emit(&Texture)` fails to compile — the plan states Phase A fully lands before Phase B and Task 6 before Task 7.
- **Risk:** Task 3 is large. If an implementer stalls, split it at the file boundary (`node/mod.rs` + `blend_space.rs` first with a temporary `AnimationNode` shim, then `state_machine.rs` + `graph.rs`), but the graph's node payload type cannot be half-converted — the shim would have to be `enum AnimationNodeKind { … , Legacy(Box<dyn AnimationNode>) }`, which is ugly; prefer to keep Task 3 atomic and give it a more capable model.
