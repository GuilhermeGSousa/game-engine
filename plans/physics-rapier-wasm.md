# Rapier as the Web Physics Backend — Analysis & Plan

Status: **proposal / not implemented**

This document analyses bringing Rapier back as the physics backend for web
(wasm) builds while keeping Jolt as the native default, and folding both behind
a single backend-agnostic `physics` crate (replacing today's `jolt_physics`):
one `PhysicsState` and one `PhysicsPipeline` resource, with a `PhysicsBackend`
trait that each backend implements.

## Where we are today

- PR #39 (`f442e77`) replaced the old Rapier-based `crates/physics` with
  `crates/jolt_physics`, backed by the in-repo `jolt-ffi` C++ shim over the
  JoltPhysics submodule.
- Jolt cannot be compiled for `wasm32-unknown-unknown`: its core requires
  `std::mutex`/`std::thread`, which the available wasm toolchains do not
  provide (wasi-libc ships no pthread stubs; libc++ is built with
  `_LIBCPP_HAS_NO_THREADS`). Jolt's own web builds go through emscripten. This
  was attempted and documented inside #39 itself.
- As a result **physics is simply absent on web**: the root `Cargo.toml` pulls
  `jolt_physics` only for non-wasm targets, and `DefaultPlugins` skips
  `PhysicsPlugin` under `#[cfg(target_arch = "wasm32")]`. Any game using
  physics does not work in the browser.
- The old Rapier crate is fully recoverable from git history at `f442e77~1`
  (`crates/physics`, rapier3d 0.26.1). Crucially, the Jolt rewrite
  **deliberately preserved its public API**: `PhysicsState::make_sphere` /
  `make_cuboid` / `get_rigid_body`, `RigidBody::new(&Transform, &mut
  PhysicsState)`, `Collider`, `PhysicsPipeline`, the `step_simulation` system,
  and `PhysicsPlugin`. The abstraction seam we need already exists.

## Does the request make sense?

**Yes.** Rapier is pure Rust and compiles to `wasm32-unknown-unknown` out of
the box (rapier.js and Bevy's web physics are built on it), so it is the
pragmatic way to get simulating physics into web builds. The alternatives are
worse:

- *Compile Jolt via emscripten*: requires an emscripten-based build of the
  whole engine (or a painful mixed toolchain), already investigated and
  abandoned in #39.
- *Use Rapier everywhere*: throws away the Jolt work that was just landed
  deliberately; Jolt remains the better native backend.
- *Keep physics off the web*: blocks the wasm story entirely for anything
  gameplay-shaped.

The rename also makes sense: consumers (examples, `game_engine::` re-exports,
future gameplay code) should import `physics::…` and never care which backend
is underneath. Note the crate is currently named `jolt_physics` (not
`physics-jolt`); the rename target is `crates/physics`.

### Costs to accept

- **Behavioural divergence.** The same scene will simulate slightly
  differently native vs web (different solvers, iteration counts, no
  cross-backend determinism). Fine for this engine today; worth stating as a
  non-goal.
- **Double maintenance.** Every new capability added to the shared `physics`
  API (joints, character controller, contact events, …) must be implemented
  twice. The current API surface is tiny (~7 operations), so the cost is low
  now and this plan keeps it low by keeping the facade minimal.
- **Rapier version gap.** The old crate used rapier3d 0.26.1; crates.io is at
  0.34.0. Resurrect on 0.26.1 first (known-good with the recovered code), bump
  in a separate change if wanted.

## Design

One crate, one set of resources, two backends behind a trait. `PhysicsState`
and `PhysicsPipeline` are **single concrete resource types** defined once in
the facade; all backend-specific work goes through a `PhysicsBackend` trait
that each backend implements. The active backend is still chosen at compile
time (a `cfg`-selected type alias), so there is no dynamic dispatch and no
`dyn` — the trait is the interface, the alias picks the implementation.

```
crates/physics/
├── Cargo.toml
└── src/
    ├── lib.rs             # cfg-selects the active backend type alias
    ├── backend.rs         # the PhysicsBackend trait (the interface)
    ├── plugin.rs          # PhysicsPlugin (shared)
    ├── simulation.rs      # step_simulation system (shared)
    ├── physics_state.rs   # PhysicsState resource (shared, wraps the backend)
    ├── physics_pipeline.rs# PhysicsPipeline resource (shared, wraps the stepper)
    ├── rigid_body.rs      # RigidBody component + lifecycle callbacks (shared)
    ├── collider.rs        # Collider component (shared)
    ├── ray.rs             # RayHit (shared)
    └── backend/
        ├── jolt.rs        # JoltBackend: impl PhysicsBackend over jolt-ffi
        └── rapier.rs      # RapierBackend: impl PhysicsBackend over rapier3d
```

### The interface

```rust
/// Raw simulation operations a physics backend must provide. Everything
/// engine-facing (ECS components, the body→entity cache, RayHit assembly)
/// lives above this trait and is written once.
pub trait PhysicsBackend: Send + Sync + Sized {
    /// Backend-native body handle. Jolt: a `u32` body id. Rapier: a
    /// generational `RigidBodyHandle`. Opaque to everything above the trait.
    type BodyId: Copy + Eq + Hash + Debug + Send + Sync;
    /// Per-step scratch (Jolt: temp allocator + job system; Rapier: the
    /// `rapier3d::PhysicsPipeline` and its counterparts).
    type Stepper: Send + Sync;

    fn new() -> Self;
    fn new_stepper() -> Self::Stepper;
    /// Advances the world by `dt` seconds (one fixed timestep).
    fn step(&mut self, stepper: &mut Self::Stepper, dt: f32);

    fn create_dynamic_body(&mut self, transform: &Transform) -> Self::BodyId;
    fn set_sphere_shape(&mut self, body: Self::BodyId, radius: f32);
    fn set_box_shape(&mut self, body: Self::BodyId, half_extents: Vec3);
    fn create_static_box(&mut self, transform: &Transform, half_extents: Vec3) -> Self::BodyId;
    fn body_transform(&self, body: Self::BodyId) -> Transform;
    /// Closest hit along `direction` (whose length bounds the cast), as
    /// (body, fraction, normal). Entity resolution and hit-point computation
    /// happen in the facade.
    fn cast_ray(&self, origin: Vec3, direction: Vec3) -> Option<(Self::BodyId, f32, Vec3)>;
}
```

### The single resources

```rust
// lib.rs — the only cfg in the crate:
#[cfg(any(target_arch = "wasm32", feature = "force-rapier"))]
pub type ActiveBackend = backend::rapier::RapierBackend;
#[cfg(not(any(target_arch = "wasm32", feature = "force-rapier")))]
pub type ActiveBackend = backend::jolt::JoltBackend;

pub type BodyId = <ActiveBackend as PhysicsBackend>::BodyId;

// physics_state.rs
#[derive(Resource)]
pub struct PhysicsState {
    backend: ActiveBackend,
    /// Body→entity cache, formerly duplicated per backend — now shared.
    body_to_entity: HashMap<BodyId, Entity>,
}

// physics_pipeline.rs
#[derive(Resource)]
pub struct PhysicsPipeline {
    stepper: <ActiveBackend as PhysicsBackend>::Stepper,
}
```

`PhysicsState` keeps its current public methods (`make_sphere`, `make_cuboid`,
`get_rigid_body`, `cast_ray`, `get_entity`, …) implemented **once** over the
trait — e.g. `cast_ray` calls the backend, then resolves the entity through
the shared cache and computes the hit point, so `RayHit` assembly is no longer
per-backend code. `PhysicsPipeline::step` passes `Time::fixed_delta_time()`
into `PhysicsBackend::step`. The components (`RigidBody`, `Collider`), the
lifecycle callbacks, `RayHit`, the plugin, and `step_simulation` are untouched
consumers of these two resources.

Notes on the trait design:

- **`BodyId` is an associated type**, so its concrete type still varies by
  backend, but the public API surface (`physics::BodyId`) is a single alias
  and stays opaque. Each backend supplies a newtype (as today's
  `BodyId(JoltBodyId)` already does; Rapier wraps `RigidBodyHandle`) so no
  `jolt_ffi::`/`rapier3d::` type leaks through the alias. If a build-independent id ever matters (savegames, network
  replication), a follow-up can switch to a universal `BodyId(u64)` newtype —
  Rapier's `RigidBodyHandle` packs into `(u32, u32)` and Jolt's id is a `u32`
  — but that is not needed now.
- **`Send + Sync` bounds live on the trait.** The Jolt backend holds raw
  pointers into C++, so its `unsafe impl Send/Sync` (with the existing
  scheduler-exclusivity justification) moves onto `JoltBackend`/its stepper;
  the Rapier backend is safe Rust and gets them for free. The facade resources
  then derive nothing unsafe.
- **The stepper is an associated type** rather than folded into the backend
  state so the two-resource split (`PhysicsState` / `PhysicsPipeline`) — and
  the ECS access pattern `step_simulation` relies on — is preserved as-is.
- **Backend selection** in `Cargo.toml` via target-specific dependencies, plus
  an escape hatch feature so the Rapier backend is testable natively (CI
  cannot run wasm tests):

  ```toml
  [features]
  # Use the Rapier backend on native targets too (testing/debugging).
  force-rapier = ["dep:rapier3d", "dep:nalgebra"]

  [target.'cfg(not(target_arch = "wasm32"))'.dependencies]
  jolt-ffi = { path = "../jolt-ffi" }
  rapier3d = { version = "0.26.1", features = ["simd-stable"], optional = true }
  nalgebra = { version = "0.34", features = ["convert-glam030"], optional = true }

  [target.'cfg(target_arch = "wasm32")'.dependencies]
  rapier3d = { version = "0.26.1" }
  nalgebra = { version = "0.34", features = ["convert-glam030"] }
  ```

  The same `cfg(any(target_arch = "wasm32", feature = "force-rapier"))`
  condition gates the `backend::rapier` module and the `ActiveBackend` alias
  shown above. On wasm, `jolt-ffi` is not a dependency at all, so wasm builds still need no
  submodule checkout and no C++ toolchain.

### Rapier backend: gaps vs the recovered code

The `f442e77~1` code covers most of the trait, but the API grew during the
Jolt rewrite. The resurrected backend must add:

1. **`cast_ray`.** Use `QueryPipeline::cast_ray_and_get_normal` (the old
   state already carried a `query_pipeline`; it must be `update()`d after
   each step). Rapier raycasts return a `ColliderHandle`; resolve it to the
   parent `RigidBodyHandle` before returning, since the trait reports hits by
   `Self::BodyId`. Entity lookup and hit-point math happen in the facade.
2. **Static geometry as bodies.** Old `make_cuboid(None)` inserted a bare
   collider with no body. To keep `RayHit::body: BodyId` total (matching
   Jolt, where everything is a body), `create_static_box` attaches the
   collider to a `RigidBodyBuilder::fixed()` body instead.
3. **Fixed timestep.** Set `IntegrationParameters::dt` to the `dt` passed
   into `PhysicsBackend::step`.

(The body→entity cache, which the Jolt crate grew in #39, does **not** need
porting — it moves up into the shared `PhysicsState`.)

### Non-goals

- Cross-backend determinism or identical tuning.
- Exposing backend-native types (`rapier3d::…`, `jolt_ffi::…`) through the
  `physics` API.
- Bumping rapier3d past 0.26.1 in the same change.

## Implementation steps

1. **Rename the crate.** `git mv crates/jolt_physics crates/physics`; package
   name `physics`. Keep `jolt-ffi` untouched.
2. **Define the interface and refit Jolt to it.** Add `backend.rs` with the
   `PhysicsBackend` trait. Turn today's `physics_state.rs`/
   `physics_pipeline.rs` internals into `backend/jolt.rs` (`JoltBackend` +
   its stepper implementing the trait; the `unsafe impl Send/Sync` move
   here). Rebuild `PhysicsState`/`PhysicsPipeline` as the single shared
   resources wrapping `ActiveBackend`, hoisting the body→entity cache and
   `RayHit` assembly out of the backend. `RigidBody::new` calls
   `PhysicsState::create_dynamic_body` (which delegates to the trait), so
   `rigid_body.rs`, `collider.rs`, `ray.rs`, `plugin.rs`, `simulation.rs`
   are shared code with no backend imports. At this point the crate builds
   and behaves exactly as before on native — a good intermediate commit.
3. **Resurrect the Rapier backend.** Start from
   `git show f442e77~1:crates/physics/src/...` into `backend/rapier.rs` as a
   second `PhysicsBackend` impl, closing the three gaps above.
4. **Rewire consumers.**
   - Root `Cargo.toml`: replace the target-gated `jolt_physics` dependency
     with an unconditional `physics = { path = "crates/physics" }`; delete the
     "physics is not supported on the web" comments.
   - `src/lib.rs`: `pub use physics;` unconditionally; `DefaultPlugins`
     registers `PhysicsPlugin` on all targets (drop the `cfg`).
   - `examples/physics-test`: imports become `game_engine::physics::…`.
5. **CI (`.github/workflows/ci.yml`).**
   - Wasm job: unchanged in spirit — still no submodules (verify `cargo build
     --target wasm32-unknown-unknown` never touches `jolt-ffi`).
   - Test job: update excludes (`--exclude physics` instead of
     `--exclude jolt_physics`), and **add** a cheap native run of the Rapier
     backend's tests: `cargo test -p physics --features force-rapier`
     (pure Rust — no submodule, no C++ compile, runs on all three OSes or just
     ubuntu).
   - Clippy: also lint the wasm/rapier side, e.g. add
     `cargo clippy -p physics --features force-rapier`.
6. **Tests.** The existing `jolt_physics/tests` (`falls`, `raycast`,
   `body_entity_cache`) are already written against the shared API — they move
   to `crates/physics/tests` unchanged and become backend-parametric for free:
   default features exercise Jolt locally, `--features force-rapier` exercises
   Rapier on CI.
7. **Verify end-to-end.** Native: run the `physics-test` example (spheres fall,
   click-raycast reports hits). Web: build the root crate for
   `wasm32-unknown-unknown` and load it in a browser; spheres must simulate.

## Risks / open questions

- **`force-rapier` feature unification.** If some crate in a native build ever
  enables `force-rapier`, the whole workspace's native build switches backend
  (features are additive). Acceptable since nothing depends on it by default
  and it exists only for CI/debugging; alternatively gate it as
  `cfg(physics_backend = "rapier")` via `RUSTFLAGS` to keep it out of the
  feature graph.
- **`simd-stable` on wasm.** The `wide`-based SIMD falls back to scalar
  without `+simd128`; leave the feature off for the wasm dependency initially
  and revisit with `-C target-feature=+simd128` later.
- **Query pipeline staleness.** Raycasts against a `QueryPipeline` that isn't
  updated after stepping return stale results — easy to miss; covered by the
  raycast test under `force-rapier`.
- **rapier3d 0.26.1 → 0.34 bump** is deferred; when taken, it is contained
  entirely inside `backend/rapier.rs`, which is exactly the point of the
  interface.
