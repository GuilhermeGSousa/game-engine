"""Pure-Python helpers for the game-engine component add-on.

This module deliberately does **not** import ``bpy`` so its logic can be unit
tested with a plain Python interpreter (see ``blender/test_core.py``).  The
Blender-facing glue lives in ``__init__.py``.

The engine reads components from a GLTF node's extras under one reserved key:

    node.extras = {
        "components": {
            "Health": { "max": 100, "current": 100 },
            "Enemy":  {}
        }
    }

Components are authored as a list of typed **fields** (key + type + value)
rather than raw JSON.  ``component_from_fields`` turns those fields into a
component's JSON value; ``fields_from_component`` is the inverse, inferring a
field type per JSON value so an existing custom property can be edited in the UI.
"""

import json

# The single reserved custom-property / extras key that holds every component.
RESERVED_KEY = "components"

# Field value types the UI offers.  JSON is the escape hatch for nested objects
# or anything the scalar/vector types can't express.
FIELD_TYPES = ("FLOAT", "INT", "BOOL", "STRING", "VEC3", "COLOR", "JSON")


def validate_json_text(text):
    """Validate the text of a ``JSON``-typed field.

    Returns ``(ok, error_message)``.  Blank text is treated as an empty object.
    """
    text = (text or "").strip()
    if text == "":
        return True, ""
    try:
        json.loads(text)
    except json.JSONDecodeError as exc:
        return False, "{} (line {}, col {})".format(exc.msg, exc.lineno, exc.colno)
    return True, ""


def component_from_fields(fields):
    """Build one component's JSON value from typed fields.

    ``fields`` is an iterable of dicts ``{"key", "type", "value"}`` where
    ``value`` is already a plain Python value (float / int / bool / str / list),
    except for type ``"JSON"`` whose ``value`` is raw JSON text.  Entries with a
    blank key or invalid JSON are skipped and reported; the rest still make it in.
    """
    data = {}
    errors = []
    for field in fields:
        key = (field.get("key") or "").strip()
        if not key:
            errors.append("Skipped a field with an empty name")
            continue
        ftype = field.get("type")
        value = field.get("value")
        if ftype == "JSON":
            text = value.strip() if isinstance(value, str) else ""
            if text == "":
                value = {}
            else:
                try:
                    value = json.loads(text)
                except json.JSONDecodeError as exc:
                    errors.append("'{}': {}".format(key, exc.msg))
                    continue
        if key in data:
            errors.append("Duplicate field '{}' (last one wins)".format(key))
        data[key] = value
    return data, errors


def fields_from_component(data):
    """Inverse of :func:`component_from_fields`.

    Infer a ``(key, type, value)`` tuple per entry of a component's JSON object,
    for populating the UI from an existing custom property.
    """
    return [_infer_field(key, value) for key, value in data.items()]


def _infer_field(key, value):
    # bool must be checked before int (bool is a subclass of int in Python).
    if isinstance(value, bool):
        return (key, "BOOL", value)
    if isinstance(value, int):
        return (key, "INT", value)
    if isinstance(value, float):
        return (key, "FLOAT", value)
    if isinstance(value, str):
        return (key, "STRING", value)
    if isinstance(value, (list, tuple)):
        numeric = all(isinstance(x, (int, float)) and not isinstance(x, bool) for x in value)
        if numeric and len(value) == 3:
            return (key, "VEC3", [float(x) for x in value])
        if numeric and len(value) == 4:
            return (key, "COLOR", [float(x) for x in value])
        return (key, "JSON", json.dumps(value))
    # dicts, null, or anything else -> raw JSON escape hatch
    return (key, "JSON", json.dumps(value))


def to_plain(value):
    """Recursively convert Blender ID-property containers to plain Python.

    ``IDPropertyGroup`` exposes ``to_dict`` and ``IDPropertyArray`` exposes
    ``to_list``; everything else (int/float/str/bool, plain dict/list) passes
    through unchanged.
    """
    if hasattr(value, "to_dict"):
        return {k: to_plain(v) for k, v in value.to_dict().items()}
    if hasattr(value, "to_list"):
        return [to_plain(v) for v in value.to_list()]
    if isinstance(value, dict):
        return {k: to_plain(v) for k, v in value.items()}
    if isinstance(value, (list, tuple)):
        return [to_plain(v) for v in value]
    return value
