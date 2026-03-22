#!/usr/bin/env bash
# e2e-test.sh — End-to-end integration test using headless GNOME Shell.
#
# Launches a headless GNOME Shell with virtual monitors, starts the tiler
# daemon, opens test windows, and verifies the full signal + tiling pipeline.
#
# Usage: nix develop --command bash scripts/e2e-test.sh
#
# Requires: gnome-shell (>=48), xterm, gdbus, dbus-run-session

set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PROJECT_DIR="$(cd "$SCRIPT_DIR/.." && pwd)"
TARGET="x86_64-unknown-linux-musl"
TILER_BIN="$PROJECT_DIR/target/$TARGET/debug/tiler"
EXT_SRC="$PROJECT_DIR/extension"
EXT_DEST="$HOME/.local/share/gnome-shell/extensions/tiler@gnome-extensions"

bold()  { printf '\033[1m%s\033[0m\n' "$*"; }

# ── Build ────────────────────────────────────────────────────────────────────
bold "Building tiler..."
(cd "$PROJECT_DIR" && cargo build --target "$TARGET" 2>&1 | tail -1)

# ── Deploy extension ─────────────────────────────────────────────────────────
bold "Deploying extension..."
mkdir -p "$EXT_DEST"
for f in metadata.json extension.js dbus.js menu.js dbus-interface.xml; do
    [ -f "$EXT_DEST/$f" ] && chmod u+w "$EXT_DEST/$f" 2>/dev/null || true
    cp "$EXT_SRC/$f" "$EXT_DEST/$f"
done

# ── Run tests inside isolated D-Bus session ──────────────────────────────────
bold "Running E2E tests in headless GNOME Shell..."
echo ""

RESULT_FILE=$(mktemp)

dbus-run-session -- bash -c '
  TILER_BIN="'"$TILER_BIN"'"
  RESULT_FILE="'"$RESULT_FILE"'"

  DBUS_NAME="org.gnome.Shell.Extensions.Tiler"
  DBUS_PATH="/org/gnome/Shell/Extensions/Tiler"
  DBUS_IFACE="$DBUS_NAME"

  PASS=0; FAIL=0

  green() { printf "\033[32m  PASS: %s\033[0m\n" "$*"; PASS=$((PASS + 1)); }
  red()   { printf "\033[31m  FAIL: %s\033[0m\n" "$*"; FAIL=$((FAIL + 1)); }

  assert_eq() {
    local name="$1" expected="$2" actual="$3"
    if [ "$expected" = "$actual" ]; then green "$name"; else red "$name (expected=$expected got=$actual)"; fi
  }

  assert_ge() {
    local name="$1" min="$2" actual="$3"
    if [ "$actual" -ge "$min" ] 2>/dev/null; then green "$name"; else red "$name (expected>=$min got=$actual)"; fi
  }

  gsettings set org.gnome.shell enabled-extensions "[\"tiler@gnome-extensions\"]" 2>/dev/null

  gnome-shell --headless --unsafe-mode \
    --virtual-monitor 1920x1080 \
    --virtual-monitor 1920x1080 \
    --virtual-monitor 1920x1080 \
    >/dev/null 2>&1 &
  SHELL_PID=$!

  # Wait for extension D-Bus name (up to 10s)
  for i in $(seq 1 100); do
    gdbus call --session -d org.freedesktop.DBus -o /org/freedesktop/DBus \
       -m org.freedesktop.DBus.NameHasOwner "$DBUS_NAME" 2>/dev/null | grep -q true && break
    sleep 0.1
  done

  eval_() {
    gdbus call --session -d org.gnome.Shell -o /org/gnome/Shell -m org.gnome.Shell.Eval "$1" >/dev/null 2>&1
  }

  # ── 1. Extension loaded ──
  HAS_NAME=$(gdbus call --session -d org.freedesktop.DBus -o /org/freedesktop/DBus \
    -m org.freedesktop.DBus.NameHasOwner "$DBUS_NAME" 2>/dev/null)
  assert_eq "Extension D-Bus name registered" "(true,)" "$HAS_NAME"

  # ── 2. WindowOpened signal has monitor_id ──
  INTROSPECT=$(gdbus introspect --session -d "$DBUS_NAME" -o "$DBUS_PATH" 2>&1)
  HAS_MON=$(echo "$INTROSPECT" | grep -A6 "WindowOpened" | grep -c "monitor_id" || true)
  assert_eq "WindowOpened signal has monitor_id" "1" "$HAS_MON"

  # ── 3. GetMonitors returns 3 monitors ──
  MON_JSON=$(gdbus call --session -d "$DBUS_NAME" -o "$DBUS_PATH" -m "$DBUS_IFACE.GetMonitors" 2>&1)
  # Count "Monitor-" occurrences in the JSON for reliable counting
  MON_COUNT=$(echo "$MON_JSON" | grep -o "Monitor-" | wc -l)
  assert_eq "GetMonitors returns 3 monitors" "3" "$MON_COUNT"

  # ── 4. Start daemon ──
  "$TILER_BIN" daemon >/tmp/e2e-daemon.log 2>&1 &
  DAEMON_PID=$!
  sleep 1

  STATUS=$("$TILER_BIN" status 2>&1)
  assert_eq "Daemon responds to status" "Ok" "$STATUS"

  # ── 5. No windows initially ──
  W_BEFORE=$("$TILER_BIN" windows 2>&1)
  assert_eq "No windows before test" "[]" "$W_BEFORE"

  # ── 6. Open test windows and verify signals ──
  eval_ "imports.gi.GLib.spawn_command_line_async(\"xterm -e sleep\\ 60\")"
  sleep 2  # Wait for first-frame timeout (1s) + margin

  eval_ "imports.gi.GLib.spawn_command_line_async(\"xterm -e sleep\\ 60\")"
  sleep 2

  DAEMON_LOG=$(cat /tmp/e2e-daemon.log 2>/dev/null)
  OPEN_COUNT=$(echo "$DAEMON_LOG" | grep -c "event: WindowOpened" || true)
  assert_eq "Daemon received 2 WindowOpened events" "2" "$OPEN_COUNT"

  # ── 7. Windows visible via CLI ──
  W_AFTER=$("$TILER_BIN" windows 2>&1)
  # Count "xterm.desktop" occurrences for reliable counting
  WIN_COUNT=$(echo "$W_AFTER" | grep -c "xterm.desktop" || true)
  assert_eq "2 windows tracked after opening" "2" "$WIN_COUNT"

  # ── 8. Apply layout via CLI ──
  APPLY=$("$TILER_BIN" apply 1 2 2>&1)
  assert_eq "Apply layout succeeds" "Ok" "$APPLY"

  # ── 9. Close a window and verify signal ──
  eval_ "let actors = global.get_window_actors(); if (actors.length > 0) actors[0].get_meta_window().delete(global.get_current_time()); \"deleted\""
  sleep 2
  DAEMON_LOG2=$(cat /tmp/e2e-daemon.log 2>/dev/null)
  CLOSE_COUNT=$(echo "$DAEMON_LOG2" | grep -c "event: WindowClosed" || true)
  assert_ge "Daemon received WindowClosed event" "1" "$CLOSE_COUNT"

  # ── Print daemon log ──
  echo ""
  printf "\033[1mDaemon log:\033[0m\n"
  grep "^\[tiler\]" /tmp/e2e-daemon.log 2>/dev/null || true

  # ── Results ──
  echo "$PASS $FAIL" > "$RESULT_FILE"

  kill $DAEMON_PID $SHELL_PID 2>/dev/null
  wait 2>/dev/null
  rm -f /tmp/e2e-daemon.log
' 2>&1 | grep -v "^dbus-daemon\|^amdgpu\|^SpiRegistry\|^discover\|^goa-daemon\|Gjs-CRITICAL\|Stack trace\|^#[0-9]\|evolution\|gnome-shell-calendar\|libedbus\|libgoaident\|GVFS-Remote\|fusermount\|xdg-desktop-portal\|GLib-GIO-CRITICAL\|Gdk-Message\|error: fuse\|process:.*WARNING"

# Read results
read PASS FAIL < "$RESULT_FILE" 2>/dev/null || { PASS=0; FAIL=1; }
rm -f "$RESULT_FILE"

echo ""
bold "Results: $PASS passed, $FAIL failed"
if [ "$FAIL" -eq 0 ]; then
    printf '\033[32m%s\033[0m\n' "All E2E tests passed"
else
    printf '\033[31m%s\033[0m\n' "Some tests failed"
    exit 1
fi
