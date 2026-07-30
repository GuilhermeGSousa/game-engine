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

Run the installer — it copies the add-on into Blender's add-ons folder and
works with Blender from the Cosmic store / Flatpak or a native install:

```sh
./blender/install.sh          # install / update (copy)
./blender/install.sh --test   # install, then run the headless export test
```

Re-run it after editing the add-on source to push the changes to Blender.

Then enable it: *Edit ▸ Preferences ▸ Add-ons* → search **Game Engine
Components** → enable.

Manual alternative: zip the `game_engine_components/` folder and use
*Preferences ▸ Add-ons ▸ Install from Disk…*.

## Use

1. Select an object. Open the 3D viewport sidebar (**N**) → **Components** tab.
2. **Add** a component and set its **name** (the Rust type name).
3. **Add Field** for each of the component's fields: type a key, pick a **type**
   from the dropdown (Float, Int, Bool, String, Vec3, Color, or JSON for nested
   data), and edit the value with the widget that appears. A component with no
   fields is a marker (`{}`). No hand-written JSON required.
4. Export with *File ▸ Export ▸ glTF 2.0*, and keep
   **Include ▸ Custom Properties** checked (it is by default).

The add-on keeps an `obj["components"]` custom property in sync with the list
(on every edit, on save, and via the **Sync** button). The glTF exporter turns
that nested custom property into the `extras.components` object above — no
exporter patching required.

Use **Copy Components to Selected** to apply the active object's components to
every other selected object.

## Field types

| Type   | Widget           | JSON output        |
|--------|------------------|--------------------|
| Float  | number field     | `2.5`              |
| Int    | number field     | `100`              |
| Bool   | checkbox         | `true` / `false`   |
| String | text field       | `"red"`            |
| Vec3   | 3 number fields  | `[x, y, z]`        |
| Color  | color picker     | `[r, g, b, a]`     |
| JSON   | text (raw JSON)  | parsed as-is       |

Use **JSON** for a nested object (e.g. `{"str": 5, "dex": 7}`) or anything the
scalar/vector types can't express. Loading a file that already has a
`components` custom property back-fills the UI, inferring a field type per value.

## Notes / limitations

- **JSON `null` is unsupported** in field values — Blender custom properties
  can't store `None`. Use a real value or omit the field (this only affects the
  raw **JSON** field type; the scalar/vector types can't produce null).
- **Numbers**: an **Int** field exports as an integer, **Float** as a float. The
  engine's deserializer tolerates int↔float for numeric fields.

## Tests

- `python3 blender/test_core.py` — unit tests for the JSON assembly logic
  (no Blender needed).
- Full round-trip — tags a cube, exports a GLB, asserts `extras.components`
  matches the contract (exits non-zero on failure). Use `--factory-startup` and
  an absolute path:
  - Flatpak: `flatpak run org.blender.Blender --background --factory-startup --python "$PWD/blender/export_test.py"`
  - Native:  `blender --background --factory-startup --python "$PWD/blender/export_test.py"`
  - Or just: `./blender/install.sh --test`
