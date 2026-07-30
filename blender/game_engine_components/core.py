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

``assemble_components`` builds that ``components`` object from the add-on's UI
list; ``split_components`` is its inverse, used to back-fill the UI when a file
already carries the custom property.
"""

import json

# The single reserved custom-property / extras key that holds every component.
RESERVED_KEY = "components"


def validate_json(text):
    """Validate one component's JSON text.

    Returns ``(ok, error_message)``.  Blank text is treated as an empty object
    (``{}``) so marker components need no typing.  The value must be a JSON
    object, since each component maps to a struct of named fields.
    """
    text = (text or "").strip()
    if text == "":
        return True, ""
    try:
        value = json.loads(text)
    except json.JSONDecodeError as exc:
        return False, "{} (line {}, col {})".format(exc.msg, exc.lineno, exc.colno)
    if not isinstance(value, dict):
        return False, 'Data must be a JSON object, e.g. {} or {"field": 1}'
    return True, ""


def assemble_components(items):
    """Build the ``components`` dict from ``(name, data_text)`` pairs.

    Entries with a blank name or invalid JSON are skipped and reported in the
    returned ``errors`` list, so a single bad row never discards the good ones.
    Blank data becomes ``{}`` (a marker component).
    """
    components = {}
    errors = []
    for name, data_text in items:
        name = (name or "").strip()
        if not name:
            errors.append("Skipped a component with an empty name")
            continue
        ok, err = validate_json(data_text)
        if not ok:
            errors.append("'{}': {}".format(name, err))
            continue
        text = (data_text or "").strip()
        if name in components:
            errors.append("Duplicate component '{}' (last one wins)".format(name))
        components[name] = json.loads(text) if text else {}
    return components, errors


def split_components(idprop_value):
    """Inverse of :func:`assemble_components`.

    Given a dict-like custom-property value (a Blender ``IDPropertyGroup`` or a
    plain dict), yield ``(name, json_text)`` pairs suitable for re-populating the
    UI list.
    """
    plain = to_plain(idprop_value)
    if not isinstance(plain, dict):
        return []
    return [(name, json.dumps(value)) for name, value in plain.items()]


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
