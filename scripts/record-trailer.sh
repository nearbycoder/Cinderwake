#!/bin/sh
# Records the README trailer (docs/media/cinderwake-demo.mp4) and teaser
# (docs/media/demo.gif) at graphics fidelity Ultra and 1920 × 1080.
#
# The captures run inside a private nested KWin (scripts/nested-kwin.sh), so
# no window reaches the desktop in use, with a temporary XDG_DATA_HOME (the
# capture modes don't read or write saves anyway). Captures and intermediate
# clips go to target/trailer/, which git ignores; delete it when done (it
# holds several gigabytes). Needs FFmpeg with libx264 and drawtext.
#
# Usage: scripts/record-trailer.sh > target/trailer.log 2>&1
set -eu
ROOT=$(cd "$(dirname "$0")/.." && pwd)
WORK="$ROOT/target/trailer"
SIZE=1920x1080

if [ "${TRAILER_SESSION:-}" = 1 ]; then
  # Inside the nested session: record, then stop the helpers this session's
  # private D-Bus started.
  cd "$WORK/run"
  export XDG_DATA_HOME="$WORK/xdg"
  mkdir -p "$XDG_DATA_HOME"
  status=0
  for mode in --motion-capture --vertical-capture --environment-tour; do
    "$ROOT/target/release/cinderwake" "$mode" --trailer --fidelity ultra --window-size "$SIZE" || status=1
  done
  for mode in --ui-gallery --gallery; do
    "$ROOT/target/release/cinderwake" "$mode" --trailer --fidelity ultra --window-size "$SIZE" || status=1
  done
  for p in $(pgrep -u "$(id -u)" -f 'ksecretd|xdg-desktop-portal|xdg-document-portal|xdg-permission-store|pipewire|wireplumber|kded|kactivitymanagerd' || true); do
    if tr '\0' '\n' < "/proc/$p/environ" 2>/dev/null | grep -qxF "DBUS_SESSION_BUS_ADDRESS=$DBUS_SESSION_BUS_ADDRESS"; then
      kill "$p" || true
    fi
  done
  echo "captures finished, status $status"
  exit "$status"
fi

cd "$ROOT"
cargo build --release --locked
rm -rf "$WORK/run/captures"
mkdir -p "$WORK/run"
TRAILER_SESSION=1 "$ROOT/scripts/nested-kwin.sh" 2200 1300 "$ROOT/scripts/record-trailer.sh"
python3 "$ROOT/scripts/trailer.py" "$WORK/run/captures" "$WORK/edit"
