"""Headless end-to-end check for the Game Engine Components add-on.

Run with a Blender binary:

    blender --background --python blender/export_test.py

It registers the add-on from this directory (no install needed), tags a cube
with two components, exports a GLB, then reads the GLB's JSON chunk back and
asserts the node's ``extras.components`` matches the contract exactly.
Exits non-zero on failure so it can gate CI.
"""

import json
import os
import struct
import sys
import tempfile

import bpy

# Make the sibling ``game_engine_components`` package importable, then register.
sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
import game_engine_components as addon  # noqa: E402

addon.register()


def read_glb_json(path):
    """Return the parsed JSON chunk of a binary glTF (.glb) file."""
    with open(path, "rb") as handle:
        blob = handle.read()
    magic, _version, _length = struct.unpack_from("<III", blob, 0)
    assert magic == 0x46546C67, "not a GLB file"
    chunk_len, chunk_type = struct.unpack_from("<II", blob, 12)
    assert chunk_type == 0x4E4F534A, "first chunk is not JSON"
    return json.loads(blob[20:20 + chunk_len])


def main():
    # Start from a clean scene.
    bpy.ops.object.select_all(action="SELECT")
    bpy.ops.object.delete()

    bpy.ops.mesh.primitive_cube_add()
    obj = bpy.context.active_object
    obj.name = "Hero"

    health = obj.game_components.add()
    health.name = "Health"
    health.data = '{"max": 100, "current": 100}'

    enemy = obj.game_components.add()
    enemy.name = "Enemy"
    enemy.data = "{}"

    errors = addon.sync_object(obj)
    assert not errors, "sync errors: {}".format(errors)

    out_path = os.path.join(tempfile.gettempdir(), "components_export_test.glb")
    bpy.ops.export_scene.gltf(
        filepath=out_path,
        export_format="GLB",
        export_extras=True,
    )

    gltf = read_glb_json(out_path)
    node = next(n for n in gltf["nodes"] if n.get("name") == "Hero")
    components = node.get("extras", {}).get("components")

    expected = {
        "Health": {"max": 100, "current": 100},
        "Enemy": {},
    }
    assert components == expected, "MISMATCH:\n  got:      {}\n  expected: {}".format(
        components, expected
    )
    print("OK  extras.components ==", json.dumps(components))


if __name__ == "__main__":
    try:
        main()
    except AssertionError as exc:
        print("FAIL", exc)
        sys.exit(1)
