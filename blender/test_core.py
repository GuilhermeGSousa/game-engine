"""Unit tests for ``game_engine_components.core`` — no Blender required.

Run with a plain interpreter:

    python3 blender/test_core.py
"""

import os
import sys

sys.path.insert(0, os.path.join(os.path.dirname(os.path.abspath(__file__)), "game_engine_components"))
import core  # noqa: E402


def test_validate_json():
    assert core.validate_json("{}") == (True, "")
    assert core.validate_json("") == (True, "")          # blank == marker
    assert core.validate_json('{"max": 100}') == (True, "")
    ok, err = core.validate_json("{not json}")
    assert not ok and err
    ok, err = core.validate_json("[1, 2, 3]")            # must be an object
    assert not ok and err
    ok, err = core.validate_json("42")
    assert not ok and err


def test_assemble_components():
    comps, errors = core.assemble_components([
        ("Health", '{"max": 100, "current": 100}'),
        ("Enemy", "{}"),
        ("Marker", ""),                                  # blank -> {}
    ])
    assert comps == {
        "Health": {"max": 100, "current": 100},
        "Enemy": {},
        "Marker": {},
    }
    assert errors == []


def test_assemble_skips_bad_rows():
    comps, errors = core.assemble_components([
        ("Health", '{"max": 100}'),
        ("", "{}"),                                      # blank name skipped
        ("Broken", "{oops}"),                            # bad json skipped
    ])
    assert comps == {"Health": {"max": 100}}
    assert len(errors) == 2


def test_duplicate_last_wins():
    comps, errors = core.assemble_components([
        ("Health", '{"max": 1}'),
        ("Health", '{"max": 2}'),
    ])
    assert comps == {"Health": {"max": 2}}
    assert any("Duplicate" in e for e in errors)


def test_split_round_trip():
    original = {"Health": {"max": 100, "current": 100}, "Enemy": {}}
    pairs = core.split_components(original)
    rebuilt, errors = core.assemble_components(pairs)
    assert rebuilt == original
    assert errors == []


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
