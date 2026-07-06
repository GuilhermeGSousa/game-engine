# Rapier as the Web Physics Backend — Analysis & Plan

Status: **proposal / not implemented**

This document analyses bringing Rapier back as the physics backend for web
(wasm) builds while keeping Jolt as the native default, and folding both behind
a single backend-agnostic `physics` crate (replacing today's `jolt_physics`).

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

One crate, two backends, selected at compile time — no traits, no dynamic
dispatch. Since exactly one backend exists per target, the backend is a
`cfg`-selected module that must expose an identical surface.

```
crates/physics/
├── Cargo.toml
└── src/
    ├── lib.rs             # cfg-selects the backend, re-exports the API
    ├── plugin.rs          # PhysicsPlugin (shared)
    ├── simulation.rs      # step_simulation system (shared)
    ├── rigid_body.rs      # RigidBody component + lifecycle callbacks (shared)
    ├── collider.rs        # Collider component (shared)
    ├── ray.rs             # RayHit (shared)
    └── backend/
        ├── jolt/          # BodyId, PhysicsState, PhysicsPipeline (today's code)
        └── rapier/        # BodyId, PhysicsState, PhysicsPipeline (resurrected + extended)
```

- **Shared front-end** (`rigid_body.rs`, `collider.rs`, `ray.rs`, `plugin.rs`,
  `simulation.rs`): already backend-agnostic in shape. `RigidBody` wraps a
  `BodyId` and its lifecycle callbacks only call
  `register_body_entity`/`unregister_*`; `step_simulation` only calls
  `pipeline.step(&mut state)` and `state.get_rigid_body(..)`.
- **Backend contract** (each backend module defines, with identical
  signatures):
  - `BodyId` — opaque, `Copy + Eq + Debug`. Jolt: `u32`. Rapier:
    `RigidBodyHandle` (a generational index — this is why `BodyId` must stay
    opaque rather than exposing the inner id).
  - `PhysicsState` — `new`, `get_entity`, `register_body_entity`,
    `unregister_body_entity`, `unregister_entity`, `make_sphere`,
    `make_cuboid`, `get_rigid_body`, `cast_ray`, plus whatever
    `RigidBody::new` needs to create a dynamic body.
  - `PhysicsPipeline` — `new`, `step(&mut PhysicsState)` using
    `Time::fixed_delta_time()`.
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

  ```rust
  // lib.rs
  #[cfg(any(target_arch = "wasm32", feature = "force-rapier"))]
  use backend::rapier as active;
  #[cfg(not(any(target_arch = "wasm32", feature = "force-rapier")))]
  use backend::jolt as active;
  pub use active::{BodyId, PhysicsState, PhysicsPipeline};
  ```

  On wasm, `jolt-ffi` is not a dependency at all, so wasm builds still need no
  submodule checkout and no C++ toolchain.

### Rapier backend: gaps vs the recovered code

The `f442e77~1` code covers most of the contract, but the API grew during the
Jolt rewrite. The resurrected backend must add:

1. **`cast_ray` → `RayHit`.** Use `QueryPipeline::cast_ray_and_get_normal`
   (the old state already carried a `query_pipeline`; it must be `update()`d
   after each step). Rapier raycasts return a `ColliderHandle`; resolve it to
   the parent `RigidBodyHandle` for `RayHit::body`.
2. **Static geometry as bodies.** Old `make_cuboid(None)` inserted a bare
   collider with no body. To keep `RayHit::body: BodyId` total (matching
   Jolt, where everything is a body), attach parentless cuboids to a
   `RigidBodyBuilder::fixed()` body instead.
3. **Body→entity cache.** Same `HashMap<BodyId, Entity>` +
   `register/unregister` methods as the Jolt backend (the shared lifecycle
   callbacks depend on them).
4. **Fixed timestep.** Set `IntegrationParameters::dt =
   Time::fixed_delta_time()` in `step` (the Jolt backend passes it per step).

### Non-goals

- Cross-backend determinism or identical tuning.
- Exposing backend-native types (`rapier3d::…`, `jolt_ffi::…`) through the
  `physics` API.
- Bumping rapier3d past 0.26.1 in the same change.

## Implementation steps

1. **Rename the crate.** `git mv crates/jolt_physics crates/physics`; package
   name `physics`. Move today's `body.rs`, `physics_state.rs`,
   `physics_pipeline.rs` into `src/backend/jolt/`. Keep `jolt-ffi` untouched.
2. **Carve the seam.** Make `rigid_body.rs` backend-agnostic: move the
   `jolt_ffi::jolt_body_create_dynamic` call behind
   `PhysicsState::create_dynamic_body(&Transform) -> BodyId` on the backend,
   so `RigidBody::new` is shared code. `collider.rs`, `ray.rs`, `plugin.rs`,
   `simulation.rs` stay as shared modules referencing the `cfg`-selected
   backend types.
3. **Resurrect the Rapier backend.** Start from
   `git show f442e77~1:crates/physics/src/...` into `src/backend/rapier/`,
   then close the four gaps above.
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
  entirely inside `src/backend/rapier/`, which is exactly the point of the
  facade.
