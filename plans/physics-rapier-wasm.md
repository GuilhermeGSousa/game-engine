# Rapier as the Web Physics Backend — Analysis & Plan

Status: **implemented** (revised after #48, "Better Colliders, ground checks
and movement", reworked the `jolt_physics` API). Two notes from the
implementation that differ from the design below: `probe_ground` on Rapier
uses parry's `contact_manifolds` (closest-point and shape-cast queries both
proved numerically unreliable for a body resting exactly on the ground), and
Rapier scene queries scan the collider set directly instead of the query BVH,
which is only refreshed during a step and would miss freshly spawned bodies.

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

### The #48 rework (`1ae24b8`)

PR #48 changed the crate's architecture from imperative body construction to
**declarative components with lifecycle-driven body creation**, and grew the
feature set considerably:

- `Collider` is now a plain shape-description enum component
  (`Sphere`/`Cuboid`/`Capsule`). Its `on_add` lifecycle reads sibling
  components (`Transform`, `RigidBody`, `ColliderOffset`), calls
  `PhysicsState::create_body`, registers the body↔entity mapping, and inserts
  a `BodyId` component (plus `TransformInterpolation` for non-static bodies).
  `on_remove` destroys the body. **The lifecycle is on `Collider`, not
  `RigidBody`.**
- `RigidBody` is now a passive descriptor: `density`, `allowed_dofs`
  (`AllowedDofs`, currently a newtype over `jolt_ffi::JoltAllowedDofs`
  bitmask constants), `motion_type` (`Dynamic`/`Kinematic`).
- `BodyId` is itself a `Component`, inserted by the lifecycle and queried by
  systems (`step_simulation`, `probe_ground`, `apply_character_movement`) and
  by game code (tech-demo jumps via `add_impulse`).
- New `GroundProbe` component + `probe_ground` system, backed by a custom
  Jolt-side query (`jolt_body_probe_ground`): collide the body's own shape
  with `max_separation` margin, keep the most-upward contact normal, classify
  against `max_slope_angle`, and report the ground body's velocity at the
  contact point (moving platforms).
- New `CharacterMovement` component + `apply_character_movement` system
  (spring-damped horizontal velocity via `linear_velocity` /
  `set_linear_velocity`, preserving the body's vertical velocity).
- New `TransformInterpolation` component + `interpolate_body_transforms`
  system: fixed-step pose history blended at `Time::fixed_alpha()` each frame.
  Pure Rust over `Transform`s — no backend contact at all.
- `PhysicsState` grew: `create_body`, `destroy_body`, `probe_ground`,
  `linear_velocity`, `set_linear_velocity`, `body_transform`, `cast_ray`,
  `add_impulse`, `add_impulse_at`, `add_force`, `add_force_at`.
- The plugin now registers the `Collider` lifecycle and four systems across
  `FixedUpdate`, `LateFixedUpdate`, and `Update`; `simulation.rs` uses the
  `profiling` crate for scopes.
- Tests expanded: `falls`, `raycast`, `body_entity_cache`, plus new
  `despawn` and `ground` integration tests and an in-module interpolation
  test.

Two consequences matter for this plan. First, the old Rapier crate recoverable
from `f442e77~1` is now only a *reference* for rapier3d API usage — its shape
(`RigidBody::new`, `make_sphere`, `make_cuboid`) no longer matches anything;
the Rapier backend is essentially a fresh implementation of the new, larger
surface. Second, the rework is actually *favourable* to the backend split:
components are now pure data descriptors, and every backend touch already
funnels through `PhysicsState` methods — the seam this plan needs is exactly
where #48 put it. The one violation is `AllowedDofs` leaking `jolt_ffi`
constants into a public component, which this plan fixes.

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
  gameplay-shaped — including the tech demo's character controller, which now
  depends on physics for movement and ground checks.

The rename also makes sense: consumers (examples, `game_engine::` re-exports,
future gameplay code) should import `physics::…` and never care which backend
is underneath. Note the crate is currently named `jolt_physics` (not
`physics-jolt`); the rename target is `crates/physics`.

### Costs to accept

- **Behavioural divergence.** The same scene will simulate slightly
  differently native vs web (different solvers, iteration counts, no
  cross-backend determinism). After #48 this now includes *gameplay-adjacent*
  behaviour — ground classification and character movement feel — not just
  ragdoll trajectories. The shared `ground`/`falls`/`despawn`/`raycast` tests
  running against both backends keep the divergence bounded to tuning, not
  semantics.
- **Double maintenance.** Every new capability added to the `PhysicsBackend`
  trait must be implemented twice. #48 demonstrates the real growth rate: the
  surface went from ~7 operations to ~13 in one PR. The trait keeps the cost
  visible and mechanical, but it is real.
- **Rapier version gap.** The old crate used rapier3d 0.26.1; crates.io is at
  0.34.0. Since the recovered code is no longer a drop-in starting point
  anyway (see above), starting directly on a recent rapier3d is now the better
  choice — there is no working 0.26.1 code to preserve.

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
    ├── interpolation.rs   # TransformInterpolation (shared, backend-free)
    ├── ground.rs          # GroundProbe/GroundState + probe_ground (shared)
    ├── movement.rs        # CharacterMovement + apply_character_movement (shared)
    ├── physics_state.rs   # PhysicsState resource (shared, wraps the backend)
    ├── physics_pipeline.rs# PhysicsPipeline resource (shared, wraps the stepper)
    ├── rigid_body.rs      # RigidBody/AllowedDofs/MotionType descriptors (shared)
    ├── collider.rs        # Collider/ColliderOffset + lifecycle (shared)
    ├── body.rs            # BodyId component (shared newtype over the handle)
    ├── ray.rs             # RayHit (shared)
    └── backend/
        ├── jolt.rs        # JoltBackend: impl PhysicsBackend over jolt-ffi
        └── rapier.rs      # RapierBackend: impl PhysicsBackend over rapier3d
```

### The interface

The trait absorbs everything `PhysicsState` currently does through `jolt_ffi`,
taking the shared descriptor types (`Collider`, `RigidBody`, offsets) as
inputs so backends translate descriptors, not re-expose their own:

```rust
/// Raw simulation operations a physics backend must provide. Everything
/// engine-facing (ECS components, lifecycles, the body→entity cache, RayHit
/// assembly, ground-state classification plumbing) lives above this trait.
pub trait PhysicsBackend: Send + Sync + Sized {
    /// Backend-native body handle. Jolt: a `u32` body id. Rapier: a
    /// generational `RigidBodyHandle`. Opaque above the trait; the facade
    /// wraps it in the shared `BodyId` component.
    type Handle: Copy + Eq + Hash + Debug + Send + Sync;
    /// Per-step scratch (Jolt: temp allocator + job system; Rapier: the
    /// `rapier3d::PhysicsPipeline` and integration parameters).
    type Stepper: Send + Sync;

    fn new() -> Self;
    fn new_stepper() -> Self::Stepper;
    /// Advances the world by `dt` seconds (one fixed timestep).
    fn step(&mut self, stepper: &mut Self::Stepper, dt: f32);

    /// Creates a body from the shared descriptors: static when `rigid_body`
    /// is `None`, otherwise dynamic/kinematic per its `MotionType`, with its
    /// `density` and `AllowedDofs`, and the shape offset applied.
    fn create_body(
        &mut self,
        collider: Collider,
        transform: &Transform,
        rigid_body: Option<RigidBody>,
        offset: Option<ColliderOffset>,
    ) -> Self::Handle;
    fn destroy_body(&mut self, body: Self::Handle);

    fn body_transform(&self, body: Self::Handle) -> Transform;
    fn linear_velocity(&self, body: Self::Handle) -> Vec3;
    fn set_linear_velocity(&mut self, body: Self::Handle, velocity: Vec3);
    fn add_impulse(&mut self, body: Self::Handle, impulse: Vec3);
    fn add_impulse_at(&mut self, body: Self::Handle, impulse: Vec3, position: Vec3);
    fn add_force(&mut self, body: Self::Handle, force: Vec3);
    fn add_force_at(&mut self, body: Self::Handle, force: Vec3, position: Vec3);

    /// Closest hit along `direction` (whose length bounds the cast), as
    /// (handle, fraction, normal). Entity resolution and hit-point math
    /// happen in the facade.
    fn cast_ray(&self, origin: Vec3, direction: Vec3) -> Option<(Self::Handle, f32, Vec3)>;
    /// Most upward-facing contact within `max_separation` of the body's
    /// shape, as (ground handle, point, normal, ground velocity at point);
    /// `None` when airborne. Slope classification against `max_slope_angle`
    /// happens in the facade — backends only find the contact.
    fn probe_ground(
        &self,
        body: Self::Handle,
        max_separation: f32,
    ) -> Option<(Self::Handle, Vec3, Vec3, Vec3)>;
}
```

### The single resources

```rust
// lib.rs — the only cfg in the crate:
#[cfg(any(target_arch = "wasm32", feature = "force-rapier"))]
pub type ActiveBackend = backend::rapier::RapierBackend;
#[cfg(not(any(target_arch = "wasm32", feature = "force-rapier")))]
pub type ActiveBackend = backend::jolt::JoltBackend;

// body.rs — BodyId must derive Component (it is inserted by Collider's
// lifecycle and queried by systems), so it cannot be a bare type alias to
// the associated type. It is a concrete shared newtype instead:
#[derive(Clone, Copy, PartialEq, Eq, Debug, Component)]
pub struct BodyId(pub(crate) <ActiveBackend as PhysicsBackend>::Handle);

// physics_state.rs
#[derive(Resource)]
pub struct PhysicsState {
    backend: ActiveBackend,
    /// Body→entity cache — already facade-level data today; stays here.
    body_to_entity: HashMap<BodyId, Entity>,
}

// physics_pipeline.rs
#[derive(Resource)]
pub struct PhysicsPipeline {
    stepper: <ActiveBackend as PhysicsBackend>::Stepper,
}
```

`PhysicsState` keeps its current public methods (`create_body`,
`destroy_body`, `probe_ground`, `linear_velocity`, `set_linear_velocity`,
`body_transform`, `cast_ray`, `add_impulse(_at)`, `add_force(_at)`,
`get_entity`, `register/unregister_body_entity`) implemented **once** over the
trait: `cast_ray` resolves the entity through the cache and computes the hit
point; `probe_ground` resolves the ground entity and classifies the contact
normal against `max_slope_angle` into `GroundState`. `PhysicsPipeline::step`
passes `Time::fixed_delta_time()` into `PhysicsBackend::step`.

Everything else in the crate becomes shared code with no backend imports:
`Collider`/`ColliderOffset` and the lifecycle callbacks, `RigidBody` and its
descriptor types, `GroundProbe`/`GroundState`/`GroundContact` and
`probe_ground`, `CharacterMovement` and `apply_character_movement`,
`TransformInterpolation` and `interpolate_body_transforms` (already
backend-free), `RayHit`, `step_simulation`, and the plugin.

Notes on the design:

- **`AllowedDofs` must stop leaking `jolt_ffi`.** Today it newtypes
  `jolt_ffi::JoltAllowedDofs` and its constants — impossible on wasm where
  `jolt_ffi` does not exist. It becomes a backend-neutral bitmask (`u32`
  newtype with the same six axis constants and `BitOr`) in shared
  `rigid_body.rs`; the Jolt backend maps it to Jolt's DOF bits, the Rapier
  backend to the complement (`LockedAxes` locks what `AllowedDofs` omits).
  Same for `MotionType`: `Kinematic` maps to Jolt's kinematic type and
  Rapier's `KinematicVelocityBased` (both are driven by
  `set_linear_velocity`, so semantics line up).
- **`Send + Sync` bounds live on the trait.** The Jolt backend holds raw
  pointers into C++, so its `unsafe impl Send/Sync` (with the existing
  scheduler-exclusivity justification) moves onto `JoltBackend`/its stepper;
  the Rapier backend is safe Rust and gets them for free.
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
  rapier3d = { version = "0.34", optional = true }
  nalgebra = { version = "0.34", features = ["convert-glam030"], optional = true }

  [target.'cfg(target_arch = "wasm32")'.dependencies]
  rapier3d = { version = "0.34" }
  nalgebra = { version = "0.34", features = ["convert-glam030"] }
  ```

  The same `cfg(any(target_arch = "wasm32", feature = "force-rapier"))`
  condition gates the `backend::rapier` module and the `ActiveBackend` alias.
  On wasm, `jolt-ffi` is not a dependency at all, so wasm builds still need no
  submodule checkout and no C++ toolchain. The `profiling` scopes in
  `simulation.rs` are unaffected: the crate compiles to no-ops on wasm unless
  a profiler backend feature is enabled.

### The Rapier backend, operation by operation

Post-#48 this is a fresh implementation (the `f442e77~1` code predates the
descriptor API and is reference material only). Most of the trait maps
directly onto rapier3d:

| Trait op | Rapier mapping |
| --- | --- |
| `create_body` | `RigidBodyBuilder::{fixed,dynamic,kinematic_velocity_based}` + `locked_axes` from `AllowedDofs` complement; `ColliderBuilder::{ball,cuboid,capsule_y}` with `.density(density)` and the `ColliderOffset` as the collider's local translation; insert with parent |
| `destroy_body` | `RigidBodySet::remove` (removes attached colliders too) |
| `body_transform` | body `position()` → translation/rotation |
| `linear_velocity` / `set_linear_velocity` | `linvel` / `set_linvel(.., true)` |
| `add_impulse(_at)` / `add_force(_at)` | `apply_impulse(_at_point)` / `add_force(_at_point)` |
| `cast_ray` | `QueryPipeline::cast_ray_and_get_normal`, then resolve the hit `ColliderHandle` to its parent body handle |
| `step` | `rapier3d::PhysicsPipeline::step` with `IntegrationParameters.dt = dt`, then `QueryPipeline::update` so queries see post-step poses |

Two operations need real work:

1. **`probe_ground`** has no rapier built-in matching the Jolt shim's
   semantics (collide the body's own shape with a separation margin, pick the
   most-upward contact normal, report the ground body's point velocity).
   Reimplement with parry through the query pipeline: gather candidate
   colliders intersecting the body's AABB inflated by `max_separation`
   (excluding the body itself), run `parry::query::contact` between the
   body's shape and each candidate with `prediction = max_separation`, keep
   the contact whose normal points most upward, and compute the ground
   velocity via the ground body's `velocity_at_point`. (Rapier's
   `KinematicCharacterController` ground detection exists but is entangled
   with its own movement solver — not a fit for `GroundProbe`'s
   read-only probe semantics.) This is the highest-effort, highest-risk piece;
   the existing `tests/ground.rs` suite (walkable slopes, steep slopes,
   separation limits, moving-platform velocity) run under `force-rapier` is
   the acceptance gate.
2. **Compound/offset shapes.** Jolt bakes `ColliderOffset` into the body's
   shape (`set_shape_offset`); rapier expresses it as the collider's position
   relative to the parent body. Same result, but `body_transform` must return
   the *body* pose (rapier already does) so the offset stays out of the
   entity's `Transform` — matching Jolt's `RotatedTranslated` behaviour.
   Verify with the `ColliderOffset::bottom_origin` path used by the
   tech-demo character.

### Non-goals

- Cross-backend determinism or identical tuning (including character-feel
  parity — spring constants may need per-backend tweaking by games, not by
  this crate).
- Exposing backend-native types (`rapier3d::…`, `jolt_ffi::…`) through the
  `physics` API. #48's `AllowedDofs` leak is fixed as part of this work.
- Joints, contact events, or other features neither backend currently
  exposes through `PhysicsState`.

## Implementation steps

1. **Rename the crate.** `git mv crates/jolt_physics crates/physics`; package
   name `physics`. Keep `jolt-ffi` untouched.
2. **Define the interface and refit Jolt to it.** Add `backend.rs` with the
   `PhysicsBackend` trait. Move the `jolt_ffi` halves of `physics_state.rs` /
   `physics_pipeline.rs` into `backend/jolt.rs` (`JoltBackend` + stepper; the
   `unsafe impl Send/Sync` move here). Rebuild `PhysicsState` /
   `PhysicsPipeline` as the single shared resources wrapping `ActiveBackend`;
   `BodyId` becomes the shared newtype component over
   `ActiveBackend::Handle`. Make `AllowedDofs`/`MotionType` backend-neutral
   with the Jolt mapping inside `backend/jolt.rs`. Slope classification and
   `GroundContact` assembly move from the backend into facade
   `PhysicsState::probe_ground`. All components, lifecycles, and systems
   (`collider.rs`, `rigid_body.rs`, `ground.rs`, `movement.rs`,
   `interpolation.rs`, `simulation.rs`, `plugin.rs`, `ray.rs`) become shared
   code with no `jolt_ffi` imports. **At this point the crate builds and
   behaves exactly as before on native — a good intermediate commit, verified
   by the existing test suite.**
3. **Implement the Rapier backend** in `backend/rapier.rs` per the table
   above, on a current rapier3d. The direct mappings can be developed and
   tested first (`falls`, `raycast`, `despawn`, `body_entity_cache` under
   `force-rapier`); `probe_ground` comes last, with `tests/ground.rs` as its
   gate. CI's `force-rapier` test run is only wired up once the full suite
   passes.
4. **Rewire consumers.**
   - Root `Cargo.toml`: unconditional `physics = { path = "crates/physics" }`;
     delete the "physics is not supported on the web" comments and the
     target-gating.
   - `src/lib.rs`: `pub use physics;` unconditionally; `DefaultPlugins`
     registers `PhysicsPlugin` on all targets (drop the `cfg`).
   - `examples/physics-test` and `examples/tech-demo` (`character.rs`,
     `scene.rs`): imports become `game_engine::physics::…`.
5. **CI (`.github/workflows/ci.yml`).**
   - Wasm job: still no submodules — verify `cargo build --target
     wasm32-unknown-unknown` never touches `jolt-ffi`.
   - Test job: update excludes (`--exclude physics` instead of
     `--exclude jolt_physics`), and **add** a native run of the Rapier
     backend's tests: `cargo test -p physics --features force-rapier`
     (pure Rust — no submodule, no C++ compile).
   - Clippy: add `cargo clippy -p physics --features force-rapier`.
6. **Tests.** The whole suite — `falls`, `raycast`, `body_entity_cache`,
   `despawn`, `ground`, and the in-module interpolation test — is already
   written against `PhysicsState`/components only. It moves to
   `crates/physics/tests` unchanged and becomes backend-parametric for free:
   default features exercise Jolt locally; `--features force-rapier`
   exercises Rapier on CI. `tests/ground.rs` doubles as the acceptance gate
   for the hand-written Rapier ground probe.
7. **Verify end-to-end.** Native: run `physics-test` and the tech demo
   (character walks, jumps, ground checks work). Web: build for
   `wasm32-unknown-unknown` and load in a browser; spheres must simulate and
   the character controller must move.

## Risks / open questions

- **Ground-probe parity** is the main risk: it is hand-written per backend
  against subtly different narrow-phase APIs. Mitigation: facade owns the
  classification logic (slope angle, state enum) so backends only find "the
  most upward contact within `max_separation`", and `tests/ground.rs` runs
  against both backends in CI.
- **`force-rapier` feature unification.** If some crate in a native build ever
  enables `force-rapier`, the whole workspace's native build switches backend
  (features are additive). Acceptable since nothing depends on it by default
  and it exists only for CI/debugging; alternatively gate it as
  `cfg(physics_backend = "rapier")` via `RUSTFLAGS` to keep it out of the
  feature graph.
- **Query pipeline staleness.** Rapier raycasts/probes read the
  `QueryPipeline`; it must be `update()`d after each step or queries return
  pre-step results — easy to miss; covered by the raycast and ground tests
  under `force-rapier`.
- **Kinematic write-back.** `step_simulation` copies `body_transform` into
  the entity `Transform` for every `BodyId`, including kinematic and (via the
  lifecycle) static bodies' entities. Verify the Rapier backend reports the
  same pose Jolt does for non-dynamic bodies so nothing drifts on web.
- **rapier3d version.** Plan targets current rapier3d (0.34); the old 0.26.1
  code no longer matters as a starting point. Any future bump stays contained
  inside `backend/rapier.rs`, which is exactly the point of the interface.
