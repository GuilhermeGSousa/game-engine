# Game Engine Components — Blender add-on

Author ECS components directly on Blender objects and ship them through GLTF so
the engine can attach real components to spawned entities. This is the
**authoring** half of the Blender-as-level-editor workflow; the runtime half
(reading the extras and inserting components) lives in the engine crates.

## The contract

Each object's components are written into its GLTF node's `extras` under one
reserved `components` key:

```json
"extras": {
  "components": {
    "Health": { "max": 100, "current": 100 },
    "Enemy":  {}
  }
}
```

- **Key** — the component's registered type name (matches the Rust type passed
  to `register_type` on the engine side).
- **Value** — the component's fields as a JSON **object**. A marker component
  (no fields) is `{}`.

## Install

1. Zip the `game_engine_components/` folder (or point Blender at it directly).
2. Blender → *Edit ▸ Preferences ▸ Add-ons ▸ Install from Disk…* → pick the zip.
3. Enable **Game Engine Components**.

## Use

1. Select an object. Open the 3D viewport sidebar (**N**) → **Components** tab.
2. **Add** a component, set its **name**, and edit its **Data** as JSON
   (`{}` for a marker). Invalid JSON is flagged inline and simply not exported.
3. Export with *File ▸ Export ▸ glTF 2.0*, and keep
   **Include ▸ Custom Properties** checked (it is by default).

The add-on keeps an `obj["components"]` custom property in sync with the list
(on every edit, on save, and via the **Sync** button). The glTF exporter turns
that nested custom property into the `extras.components` object above — no
exporter patching required.

Use **Copy Components to Selected** to apply the active object's components to
every other selected object.

## Notes / limitations

- **JSON `null` is unsupported** in field values — Blender custom properties
  can't store `None`. Use a real value or omit the field.
- **Numbers**: `100` exports as an integer, `100.0` as a float. The engine's
  deserializer tolerates int↔float for numeric fields.
- Data is edited as a single-line JSON string. That is fine for the small
  payloads components usually carry; a larger multi-line editor is a possible
  future addition.

## Tests

- `python3 blender/test_core.py` — unit tests for the JSON assembly logic
  (no Blender needed).
- `blender --background --python blender/export_test.py` — full round-trip:
  tags a cube, exports a GLB, and asserts `extras.components` matches the
  contract. Exits non-zero on failure.
