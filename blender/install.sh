#!/usr/bin/env bash
# Installer for the Game Engine Components Blender add-on.
#
# Copies this package into Blender's add-ons directory. Handles Blender
# installed via Flatpak (Cosmic store / Pop!_OS) or natively.
#
# Note: this copies the files, so re-run after editing the add-on source to
# push changes to Blender.
#
#   ./blender/install.sh          # install / update (copy)
#   ./blender/install.sh --test   # install, then run the headless export test
set -euo pipefail

ADDON_NAME="game_engine_components"
HERE="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
SRC="$HERE/$ADDON_NAME"

FLATPAK_ROOT="$HOME/.var/app/org.blender.Blender/config/blender"
NATIVE_ROOT="$HOME/.config/blender"

# Echo the newest "<major>.<minor>" version directory under a Blender config root.
latest_version_dir() {
    local root="$1" ver
    [ -d "$root" ] || return 1
    ver="$(ls -1 "$root" 2>/dev/null | grep -E '^[0-9]+\.[0-9]+' | sort -V | tail -1)"
    [ -n "$ver" ] || return 1
    printf '%s/%s' "$root" "$ver"
}

KIND="Flatpak"
if ! VER_DIR="$(latest_version_dir "$FLATPAK_ROOT")"; then
    KIND="native"
    if ! VER_DIR="$(latest_version_dir "$NATIVE_ROOT")"; then
        cat >&2 <<EOF
Could not find a Blender config directory in:
  $FLATPAK_ROOT
  $NATIVE_ROOT
Launch Blender once so it creates its config, then re-run this script.
EOF
        exit 1
    fi
fi

ADDONS_DIR="$VER_DIR/scripts/addons"
DEST="$ADDONS_DIR/$ADDON_NAME"

mkdir -p "$ADDONS_DIR"
# Remove any previous install (a copied directory or an old symlink).
rm -rf "$DEST"
cp -r "$SRC" "$DEST"
# Drop bytecode that may have been copied along.
rm -rf "$DEST/__pycache__"

echo "Installed ($KIND Blender ${VER_DIR##*/}), copied to:"
echo "  $DEST"
echo
echo "Enable:  Blender > Edit > Preferences > Add-ons > search 'Game Engine Components'"
echo "Use:     3D viewport > press N > Components tab"
echo "Note:    re-run this script after editing the add-on to update Blender's copy."

if [ "${1:-}" = "--test" ]; then
    echo
    echo "=== running headless export test ==="
    if [ "$KIND" = "Flatpak" ]; then
        flatpak run org.blender.Blender --background --factory-startup --python "$HERE/export_test.py"
    else
        blender --background --factory-startup --python "$HERE/export_test.py"
    fi
fi
