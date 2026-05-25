# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## Commands

```bash
# Build (native)
cargo build

# Build (WASM)
cargo build --target wasm32-unknown-unknown

# Run native binary
cargo run --bin game-engine-bin

# Run WASM in browser (requires trunk and wasm-server-runner)
trunk serve

# Run all tests
cargo test --workspace

# Run tests for a specific crate
cargo test -p ecs

# Lint
cargo clippy -- -A clippy::type_complexity -A clippy::too_many_arguments -D warnings

# Format check
cargo fmt --all -- --check

# Format
cargo fmt --all
```

The `build.rs` script copies `res/` into `target/<profile>/res/` automatically. WASM builds use `trunk` with `Trunk.toml`.

## Architecture Overview

This is a Bevy-inspired, data-driven 3D game engine. The top-level `game-engine` crate (in `src/`) is both the engine entry point and a demo application. Core functionality lives in workspace crates under `crates/`.

### Crate Map

| Crate | Role |
|---|---|
| `ecs` | Core ECS: `World`, archetype storage, `Query`, `Schedule`, `System`, change detection, events |
| `app` | `App` builder, `Plugin` trait, schedule runner, `UpdateGroup` |
| `essential` | `Transform`/`GlobalTransform`, `Time`, `AssetServer`/`AssetStore`, utility types |
| `render` | wgpu-based renderer, material system, `RenderPlugin` |
| `animation` | Animation graph, blend trees, state machines |
| `physics` | Rapier3D integration (`PhysicsPlugin`) |
| `window` | winit window + input (`WindowPlugin`, `Input`) |
| `gltf-loader` / `obj-loader` | Asset loaders for 3D formats |
| `skybox` | Skybox rendering |
| `ui` / `ui-egui` | UI layer |
| `tasks` | Thread pool (`ComputeTaskPool`) used by multithreaded executor |

### ECS

The ECS uses **archetype-based storage** (each unique component combination gets its own archetype/table). Key types:

- `World` — owns all entities, components, resources, and the archetype graph.
- `Entity` — index + generation handle.
- `Query<D, F>` — typed iterator; D is data (e.g. `(&Position, &mut Velocity)`), F is filter (`With<T>`, `Without<T>`, `Added<T>`, `Changed<T>`, `Or<(...)>`).
- `Res<T>` / `ResMut<T>` — resource access in systems.
- `CommandQueue` — deferred spawn/despawn/insert, applied at end of system.
- `EventChannel<T>` — double-buffered event queue; flush via `update_event_channel::<T>` each frame (registered automatically by `App::register_event`).
- `Schedule` — collection of systems with ordering constraints (`.after()` / `.before()`). Compiled into a dependency graph executed by either `SingleThreadedExecutor` or `MultiThreadedExecutor`.

The `multithreaded` feature (enabled by default, disabled on WASM) switches the executor. The multi-threaded executor uses the `tasks` crate's thread pool and a `FixedBitSet`-based readiness tracker.

Change detection (`Added<T>`, `Changed<T>`) is tracked per-component at the table level; `world.tick()` advances the tick counter each frame.

Derive macros (`#[derive(Component)]`, `#[derive(Resource)]`, `#[derive(Event)]`) live in `crates/ecs/macros/`.

### App Lifecycle

```
App::new()
  → register_plugin(P)   // calls P::build() immediately
  → ...
  → run()
      → plugin_state() loop until PluginsState::Ready
      → finish_plugin_build()  // calls P::finish(), compiles schedules, runs Startup
      → frame loop: app.update()
          → FixedUpdate (repeated until budget exhausted)
          → Update
          → LateUpdate
          → Render / LateRender
          → world.tick()
```

`UpdateGroup` variants: `Startup`, `FixedUpdate`, `LateFixedUpdate`, `Update`, `LateUpdate`, `Render`, `LateRender`.

### Render System

The renderer is wgpu-based. The entry point is `RenderPlugin` in `crates/render/src/plugin.rs`.

**Material system** — custom materials implement the `Material` trait (which extends `AsBindGroup`) and are registered with `MaterialPlugin::<M>::new()`. The bind-group slot convention:

| Group | Contents | Condition |
|---|---|---|
| 0 | Material's own bindings | always |
| 1 | Camera uniform | `M::needs_camera()` |
| 2 | Lighting uniform | `M::needs_lighting()` |
| 3 | Skeleton uniforms | `M::needs_skeleton()` |

`#[derive(AsBindGroup)]` from `render-macros` generates the bind group layout and creation code.

Entities that need rendering carry `MeshComponent` + `MaterialComponent<M>` + `Transform`.

### Transform Hierarchy

`Transform` (local) and `GlobalTransform` are separate components. Adding a `Transform` automatically inserts a `GlobalTransform` via a component lifecycle callback (`on_add`). `TransformPlugin` registers systems that propagate parent-to-child global transforms each `LateUpdate`.

### Assets

`AssetServer` manages async asset loading. Register an asset type with `app.register_asset::<MyAsset>()` (requires `AssetManagerPlugin`). Assets are identified by `AssetHandle<T>`.

### Lints

`[lints.rust] warnings = "deny"` is set in the root `Cargo.toml` — all warnings are errors. Clippy is run with `-A clippy::type_complexity -A clippy::too_many_arguments`.
