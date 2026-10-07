#!/bin/sh
# Testing aid: runs one session script inside a private nested KWin, with a
# virtual output, its own Xwayland, and its own D-Bus, so test windows never
# appear on the desktop you're using. The script sees DISPLAY and
# WAYLAND_DISPLAY for the nested session and can drive it there (for
# example, KWin scripts through qdbus6, or key events through XTEST).
# KWin starts any extra words as separate applications, so pass parameters
# to the script through the environment. Write the output to a file rather
# than a pipe: the private D-Bus can keep a pipe open after the session ends.
#
# Usage: scripts/nested-kwin.sh <width> <height> <session-script> > log 2>&1
[ $# -eq 3 ] || { echo "usage: $0 <width> <height> <session-script>" >&2; exit 2; }
exec dbus-run-session -- kwin_wayland --virtual --xwayland --no-lockscreen \
  --no-global-shortcuts --socket "cinderwake-nested-$$" --width "$1" --height "$2" \
  --exit-with-session "$3"
