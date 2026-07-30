"""Unit tests for ``game_engine_components.core`` — no Blender required.

Run with a plain interpreter:

    python3 blender/test_core.py
"""

import os
import sys

sys.path.insert(0, os.path.join(os.path.dirname(os.path.abspath(__file__)), "game_engine_components"))
import core  # noqa: E402


def field(key, ftype, value):
    return {"key": key, "type": ftype, "value": value}


def test_validate_json_text():
    assert core.validate_json_text("{}") == (True, "")
    assert core.validate_json_text("") == (True, "")
    assert core.validate_json_text('{"a": 1}') == (True, "")
    ok, err = core.validate_json_text("{nope}")
    assert not ok and err


def test_component_from_fields_typed():
    data, errors = core.component_from_fields([
        field("max", "INT", 100),
        field("regen", "FLOAT", 2.5),
        field("alive", "BOOL", True),
        field("team", "STRING", "red"),
        field("spawn", "VEC3", [1.0, 2.0, 3.0]),
        field("tint", "COLOR", [1.0, 0.0, 0.0, 1.0]),
    ])
    assert errors == []
    assert data == {
        "max": 100, "regen": 2.5, "alive": True, "team": "red",
        "spawn": [1.0, 2.0, 3.0], "tint": [1.0, 0.0, 0.0, 1.0],
    }


def test_marker_component_is_empty():
    data, errors = core.component_from_fields([])
    assert data == {}
    assert errors == []


def test_json_field_parsed_and_nested():
    data, errors = core.component_from_fields([
        field("stats", "JSON", '{"str": 5, "dex": 7}'),
        field("empty", "JSON", ""),
    ])
    assert errors == []
    assert data == {"stats": {"str": 5, "dex": 7}, "empty": {}}


def test_bad_rows_skipped():
    data, errors = core.component_from_fields([
        field("ok", "INT", 1),
        field("", "INT", 2),                 # blank key skipped
        field("broken", "JSON", "{oops}"),   # invalid json skipped
    ])
    assert data == {"ok": 1}
    assert len(errors) == 2


def test_infer_and_round_trip():
    data = {
        "max": 100, "regen": 2.5, "alive": True, "team": "red",
        "spawn": [1.0, 2.0, 3.0], "tint": [1.0, 0.0, 0.0, 1.0],
        "stats": {"str": 5},
    }
    inferred = core.fields_from_component(data)
    types = {key: ftype for key, ftype, _ in inferred}
    assert types == {
        "max": "INT", "regen": "FLOAT", "alive": "BOOL", "team": "STRING",
        "spawn": "VEC3", "tint": "COLOR", "stats": "JSON",
    }
    rebuilt, errors = core.component_from_fields(
        {"key": k, "type": t, "value": v} for k, t, v in inferred
    )
    assert errors == []
    assert rebuilt == data


def test_to_plain_passthrough():
    assert core.to_plain({"a": [1, 2, {"b": 3}]}) == {"a": [1, 2, {"b": 3}]}


def _run():
    tests = [v for k, v in sorted(globals().items()) if k.startswith("test_")]
    for fn in tests:
        fn()
        print("ok  ", fn.__name__)
    print("\nAll {} tests passed".format(len(tests)))


if __name__ == "__main__":
    _run()
