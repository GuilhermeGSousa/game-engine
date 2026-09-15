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
All**. Clicking rendered mesh bounds synchronizes selection with the hierarchy.

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
