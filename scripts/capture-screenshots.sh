#!/usr/bin/env bash
# Capture the Index Mail window on an Omarchy/Hyprland session.
#
# Usage:
#   ./scripts/capture-screenshots.sh <label> [output-dir]
#
# Examples:
#   ./scripts/capture-screenshots.sh 01-inbox
#   ./scripts/capture-screenshots.sh 04-settings-design
#
# Put the app in the state you want, then run the script. It finds the
# `unkai-app` window through Hyprland's IPC and grabs exactly that window at
# its current size (the app opens at 1920x1080 on a 1080p display).
#
# Personal data: blur sender addresses/subjects before committing a capture
# (e.g. with GIMP or `magick input.png -region ... -blur 0x12 output.png`).

set -euo pipefail

label="${1:-}"
out_dir="${2:-docs/screenshots}"

if [[ -z "$label" ]]; then
  echo "usage: $0 <label> [output-dir]" >&2
  exit 2
fi

if ! command -v grim >/dev/null 2>&1; then
  echo "grim not found. On Omarchy: sudo pacman -S grim" >&2
  exit 1
fi
if ! command -v hyprctl >/dev/null 2>&1; then
  echo "hyprctl not found — this script targets Hyprland." >&2
  exit 1
fi

if [[ -z "${HYPRLAND_INSTANCE_SIGNATURE:-}" ]]; then
  # Pick the only/first Hyprland instance for this user.
  HYPRLAND_INSTANCE_SIGNATURE="$(find "${XDG_RUNTIME_DIR:-/run/user/$(id -u)}/hypr" \
    -mindepth 1 -maxdepth 1 -type d -printf '%f\n' 2>/dev/null | head -n1 || true)"
  export HYPRLAND_INSTANCE_SIGNATURE
fi
if [[ -z "${WAYLAND_DISPLAY:-}" ]]; then
  export WAYLAND_DISPLAY="wayland-1"
fi

geometry="$(hyprctl clients -j | python3 -c '
import json, sys
try:
    clients = json.load(sys.stdin)
except Exception:
    sys.exit(1)
for c in clients:
    if c.get("class") == "unkai-app":
        x, y = c["at"]
        w, h = c["size"]
        print(f"{x},{y} {w}x{h}")
        break
')"

if [[ -z "$geometry" ]]; then
  echo "No unkai-app window found. Start it with: cargo tauri dev" >&2
  exit 1
fi

mkdir -p "$out_dir"
out="$out_dir/${label}.png"
grim -g "$geometry" "$out"
echo "wrote $out ($geometry)"
