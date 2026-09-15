# Wonderland editor

The engine-native Looking Glass editor imports glTF content into a temporary,
read-only world. It uses the engine's ECS, renderer, window, asset, scene, and UI
crates directly.

```sh
cargo run -p editor -- --project examples/render-test
```

Run without arguments to choose a project folder. Linux folder selection uses
`zenity`; entering a path and clicking **Open Project** works without it.
Folder dialogs on other desktop platforms are not implemented yet.

The bottom Content browser scans imported asset headers and supports search and
paging. Click a Scene to add a fresh instance at the origin. Repeated clicks add
independent instances; other asset kinds can be selected for inspection.

The hierarchy preserves each scene as a separate instance root. Disclosure
arrows expand branches and search reveals matches with their ancestors. Details
shows imported component JSON and can remove the selected whole instance without
deleting its assets.

The center viewport uses a dedicated render target, editor camera, ground grid,
and fallback light. Right-drag orbits, middle-drag or Shift-right-drag pans, and
the wheel zooms. Press F over the viewport to frame selection or use **Frame
All**.

Navigate the focused tree with Up/Down, Left/Right and Home/End. Wheel/trackpad
scrolling applies only over the tree; buttons also scroll by one visible page.
The tree uses twelve reusable rows, keeping UI entity count independent of scene
size. Deep indentation is visually capped; the document retains full ancestry.

Import `.gltf`/`.glb` with the toolbar or drop a file over the viewport. Jobs are
serialized. A successful import rescans and atomically publishes the UUID
registry, refreshes resident assets, adds the emitted Scene, selects it, and
frames it when bounds become available. A failed import spawns nothing and
attempts to recover a fully valid content catalogue.

World composition is deliberately temporary and is cleared on project switch or
exit. Imported outputs and their stable UUIDs remain in the project.

## Custom property editors

The inspector edits live component values through typed `PropertyEditor<T>`
adapters. `editable` provides structural access; adapters own snapshots, widgets,
and validated edits. A registered editor can handle a whole struct or an opaque
type, and can replace any built-in numeric editor. Snapshots are cached per
component, with snapshots owned by property-row entities. Component cards carry
`InspectedComponent` and can be accessed through ordinary ECS queries.
Unchanged components reuse their snapshots;
component changes, selection changes, and editor registration changes refresh
them. Widget input and focused edit buffers continue updating every frame.

See the [adapter guide](src/inspector/custom_editors.md) and the
[complete custom widget example](examples/custom_property.rs). The example
includes registration, a non-Clone domain type, click handling, snapshot refresh,
and validation. Run its headless demonstration with:

```sh
cargo run -p editor --example custom_property
```

Register custom widget systems in `LateUpdate` after `InspectorPlugin`. Queued
edits apply in the following `Update` against their captured world targets.
