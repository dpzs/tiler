#!/usr/bin/env bash
# e2e-test.sh — End-to-end integration test using headless GNOME Shell.
#
# Tests every major tiler goal against a real GNOME Shell instance.
# Geometry assertions use tolerance to accommodate X11/CSD decoration offsets.
#
# Usage: nix develop --command bash scripts/e2e-test.sh

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

# ── Write inner test script ──────────────────────────────────────────────────
INNER_SCRIPT=$(mktemp)
RESULT_FILE=$(mktemp)

cat > "$INNER_SCRIPT" << 'INNER_EOF'
#!/usr/bin/env bash
set -euo pipefail

TILER_BIN="$1"
RESULT_FILE="$2"

DBUS_NAME="org.gnome.Shell.Extensions.Tiler"
DBUS_PATH="/org/gnome/Shell/Extensions/Tiler"
DBUS_IFACE="$DBUS_NAME"
TOL=50  # geometry tolerance in px for X11/CSD decorations

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

assert_contains() {
  local name="$1" haystack="$2" needle="$3"
  if echo "$haystack" | grep -q "$needle"; then green "$name"; else red "$name (missing: $needle)"; fi
}

assert_not_contains() {
  local name="$1" haystack="$2" needle="$3"
  if ! echo "$haystack" | grep -q "$needle"; then green "$name"; else red "$name (unexpected: $needle found)"; fi
}

# Check if geometry is within tolerance (returns 0=match, 1=mismatch)
_geom_matches() {
  local expected="$1" actual="$2"
  local ex ey ew eh ax ay aw ah
  read -r ex ey ew eh <<< "$expected"
  read -r ax ay aw ah <<< "$actual"
  for pair in "$ex:$ax" "$ey:$ay" "$ew:$aw" "$eh:$ah"; do
    local e="${pair%%:*}" a="${pair##*:}"
    local diff=$(( a > e ? a - e : e - a ))
    if [ "$diff" -gt "$TOL" ]; then return 1; fi
  done
  return 0
}

# Assert geometry with retry: window_id, expected "x y w h", test name
# Retries up to 3 times (move_resize_frame can be async in headless/XWayland)
assert_geom_near() {
  local name="$1" expected="$2" actual="$3"
  if _geom_matches "$expected" "$actual"; then
    green "$name"
    return
  fi
  # Retry: maybe the move hasn't been processed yet
  for retry in 1 2 3; do
    sleep 1
    # re-read from the window_id stashed in $4 if provided
    if [ -n "${4:-}" ]; then
      actual=$(get_window_geometry "$4")
    fi
    if _geom_matches "$expected" "$actual"; then
      green "$name (retry $retry)"
      return
    fi
  done
  red "$name (expected≈$expected got=$actual tol=$TOL)"
}

# Assert window is on a given monitor (by x-range), with retry via window_id ($5)
assert_on_monitor() {
  local name="$1" mon_x="$2" mon_w="$3" actual="$4"
  local mon_end=$((mon_x + mon_w))
  for attempt in 0 1 2 3; do
    local ax
    read -r ax _ _ _ <<< "$actual"
    if [ "$ax" -ge "$mon_x" ] 2>/dev/null && [ "$ax" -lt "$mon_end" ] 2>/dev/null; then
      if [ "$attempt" -gt 0 ]; then green "$name (retry $attempt)"; else green "$name"; fi
      return
    fi
    [ "$attempt" -lt 3 ] && [ -n "${5:-}" ] && { sleep 1; actual=$(get_window_geometry "$5"); }
  done
  local ax; read -r ax _ _ _ <<< "$actual"
  red "$name (window x=$ax not in monitor range $mon_x-$mon_end)"
}

eval_() {
  gdbus call --session -d org.gnome.Shell -o /org/gnome/Shell -m org.gnome.Shell.Eval "$1" >/dev/null 2>&1
}

eval_val() {
  local raw
  raw=$(gdbus call --session -d org.gnome.Shell -o /org/gnome/Shell -m org.gnome.Shell.Eval "$1" 2>/dev/null)
  echo "$raw" | sed -e 's/^(true, //' -e 's/)$//' -e "s/^'//" -e "s/'$//" -e 's/^"//;s/"$//' -e 's/\\"/"/g'
}

get_window_geometry() {
  eval_val "let a = global.get_window_actors(); let w = a.find(a => a.get_meta_window().get_stable_sequence() === $1); if (w) { let r = w.get_meta_window().get_frame_rect(); r.x + ' ' + r.y + ' ' + r.width + ' ' + r.height; } else { 'not_found'; }"
}

spawn_xterm() {
  eval_ "imports.gi.GLib.spawn_command_line_async('xterm -e sleep 120')"
  sleep 2
}

close_window() {
  eval_ "let actors = global.get_window_actors(); let w = actors.find(a => a.get_meta_window().get_stable_sequence() === $1); if (w) w.get_meta_window().delete(global.get_current_time());"
  sleep 2
}

cleanup() {
  kill "$DAEMON_PID" "$SHELL_PID" 2>/dev/null || true
  wait 2>/dev/null || true
  rm -f /tmp/e2e-daemon.log
}
trap cleanup EXIT

# ── Setup ────────────────────────────────────────────────────────────────────
gsettings set org.gnome.shell enabled-extensions '["tiler@gnome-extensions"]' 2>/dev/null

gnome-shell --headless --unsafe-mode \
  --virtual-monitor 1920x1080 \
  --virtual-monitor 1920x1080 \
  --virtual-monitor 1920x1080 \
  >/dev/null 2>&1 &
SHELL_PID=$!

for i in $(seq 1 100); do
  gdbus call --session -d org.freedesktop.DBus -o /org/freedesktop/DBus \
     -m org.freedesktop.DBus.NameHasOwner "$DBUS_NAME" 2>/dev/null | grep -q true && break
  sleep 0.1
done

# ═════════════════════════════════════════════════════════════════════════════
printf "\033[1m── Extension & D-Bus Interface ──\033[0m\n"
# ═════════════════════════════════════════════════════════════════════════════

HAS_NAME=$(gdbus call --session -d org.freedesktop.DBus -o /org/freedesktop/DBus \
  -m org.freedesktop.DBus.NameHasOwner "$DBUS_NAME" 2>/dev/null)
assert_eq "Extension D-Bus name registered" "(true,)" "$HAS_NAME"

INTROSPECT=$(gdbus introspect --session -d "$DBUS_NAME" -o "$DBUS_PATH" 2>&1)
HAS_MON=$(echo "$INTROSPECT" | grep -A6 "WindowOpened" | grep -c "monitor_id" || true)
assert_eq "WindowOpened signal includes monitor_id" "1" "$HAS_MON"

MON_JSON=$(gdbus call --session -d "$DBUS_NAME" -o "$DBUS_PATH" -m "$DBUS_IFACE.GetMonitors" 2>&1)
MON_COUNT=$(echo "$MON_JSON" | grep -o '"Monitor-' | wc -l)
assert_eq "GetMonitors returns 3 virtual monitors" "3" "$MON_COUNT"

WS=$(gdbus call --session -d "$DBUS_NAME" -o "$DBUS_PATH" -m "$DBUS_IFACE.GetActiveWorkspace" 2>&1)
assert_eq "GetActiveWorkspace returns 0" "(uint32 0,)" "$WS"

# ═════════════════════════════════════════════════════════════════════════════
printf "\n\033[1m── Daemon Startup & IPC (FR-001/002/003) ──\033[0m\n"
# ═════════════════════════════════════════════════════════════════════════════

"$TILER_BIN" daemon >/tmp/e2e-daemon.log 2>&1 &
DAEMON_PID=$!
sleep 1

STATUS=$("$TILER_BIN" status 2>&1)
assert_eq "Daemon responds to CLI status" "Ok" "$STATUS"

W_BEFORE=$("$TILER_BIN" windows 2>&1)
assert_eq "No windows at startup (FR-004)" "[]" "$W_BEFORE"

# ═════════════════════════════════════════════════════════════════════════════
printf "\n\033[1m── New Window Detection (FR-005) ──\033[0m\n"
# ═════════════════════════════════════════════════════════════════════════════

spawn_xterm  # id=1

DAEMON_LOG=$(cat /tmp/e2e-daemon.log 2>/dev/null)
OPEN1=$(echo "$DAEMON_LOG" | grep -c "event: WindowOpened" || true)
assert_eq "WindowOpened signal received" "1" "$OPEN1"

W_AFTER1=$("$TILER_BIN" windows 2>&1)
W1_COUNT=$(echo "$W_AFTER1" | grep -c "xterm.desktop" || true)
assert_eq "Window tracked via CLI" "1" "$W1_COUNT"

GEOM1=$(get_window_geometry 1)
assert_on_monitor "Window 1 on stack screen (monitor 0)" 0 1920 "$GEOM1" 1
assert_geom_near "Window 1 fills stack screen (FR-008)" "0 0 1920 1080" "$GEOM1" 1

# ═════════════════════════════════════════════════════════════════════════════
printf "\n\033[1m── Vertical Stack: 2 windows (FR-008) ──\033[0m\n"
# ═════════════════════════════════════════════════════════════════════════════

spawn_xterm  # id=2

# push_window: newest (id=2) at top, older (id=1) at bottom, each ~540px tall
GEOM_W2=$(get_window_geometry 2)
GEOM_W1=$(get_window_geometry 1)
assert_on_monitor "Stack(2): win 2 on monitor 0" 0 1920 "$GEOM_W2" 2
assert_on_monitor "Stack(2): win 1 on monitor 0" 0 1920 "$GEOM_W1" 1
assert_geom_near "Stack(2): top half" "0 0 1920 540" "$GEOM_W2" 2
assert_geom_near "Stack(2): bottom half" "0 540 1920 540" "$GEOM_W1" 1

# ═════════════════════════════════════════════════════════════════════════════
printf "\n\033[1m── Vertical Stack: 3 windows (FR-008) ──\033[0m\n"
# ═════════════════════════════════════════════════════════════════════════════

spawn_xterm  # id=3

GEOM_W3=$(get_window_geometry 3)
GEOM_W2b=$(get_window_geometry 2)
GEOM_W1b=$(get_window_geometry 1)
assert_on_monitor "Stack(3): all on monitor 0" 0 1920 "$GEOM_W3" 3
assert_geom_near "Stack(3): top (360px)" "0 0 1920 360" "$GEOM_W3" 3
assert_geom_near "Stack(3): mid (360px)" "0 360 1920 360" "$GEOM_W2b" 2
assert_geom_near "Stack(3): bot (360px)" "0 720 1920 360" "$GEOM_W1b" 1

# ═════════════════════════════════════════════════════════════════════════════
printf "\n\033[1m── Stack Retile on Close (FR-009) ──\033[0m\n"
# ═════════════════════════════════════════════════════════════════════════════

close_window 3

DAEMON_LOG2=$(cat /tmp/e2e-daemon.log 2>/dev/null)
CLOSE_COUNT=$(echo "$DAEMON_LOG2" | grep -c "event: WindowClosed" || true)
assert_ge "WindowClosed signal received" "1" "$CLOSE_COUNT"

GEOM_W2c=$(get_window_geometry 2)
GEOM_W1c=$(get_window_geometry 1)
assert_geom_near "Retiled: top half" "0 0 1920 540" "$GEOM_W2c" 2
assert_geom_near "Retiled: bottom half" "0 540 1920 540" "$GEOM_W1c" 1

# ═════════════════════════════════════════════════════════════════════════════
printf "\n\033[1m── Layout: Side-by-Side (FR-015, preset 2) ──\033[0m\n"
# ═════════════════════════════════════════════════════════════════════════════

APPLY_SBS=$("$TILER_BIN" apply 1 2 2>&1)
assert_eq "Apply side-by-side succeeds" "Ok" "$APPLY_SBS"
sleep 1

GEOM_L=$(get_window_geometry 2)
GEOM_R=$(get_window_geometry 1)
assert_on_monitor "SBS: left on monitor 0" 0 1920 "$GEOM_L" 2
assert_on_monitor "SBS: right on monitor 0" 0 1920 "$GEOM_R" 1
assert_geom_near "SBS: left half" "0 0 960 1080" "$GEOM_L" 2
assert_geom_near "SBS: right half" "960 0 960 1080" "$GEOM_R" 1

# ═════════════════════════════════════════════════════════════════════════════
printf "\n\033[1m── Layout: Top-Bottom (FR-015, preset 3) ──\033[0m\n"
# ═════════════════════════════════════════════════════════════════════════════

APPLY_TB=$("$TILER_BIN" apply 1 3 2>&1)
assert_eq "Apply top-bottom succeeds" "Ok" "$APPLY_TB"
sleep 1

GEOM_T=$(get_window_geometry 2)
GEOM_B=$(get_window_geometry 1)
assert_geom_near "TB: top half" "0 0 1920 540" "$GEOM_T" 2
assert_geom_near "TB: bottom half" "0 540 1920 540" "$GEOM_B" 1

# ═════════════════════════════════════════════════════════════════════════════
printf "\n\033[1m── Layout: Fullscreen (FR-015, preset 1) ──\033[0m\n"
# ═════════════════════════════════════════════════════════════════════════════

APPLY_FS=$("$TILER_BIN" apply 1 1 2>&1)
assert_eq "Apply fullscreen succeeds" "Ok" "$APPLY_FS"
sleep 1

GEOM_FS=$(get_window_geometry 2)
assert_on_monitor "FS: on monitor 0" 0 1920 "$GEOM_FS" 2
assert_geom_near "FS: fills monitor" "0 0 1920 1080" "$GEOM_FS" 2

# ═════════════════════════════════════════════════════════════════════════════
printf "\n\033[1m── Layout: Quadrants (FR-015, preset 4) ──\033[0m\n"
# ═════════════════════════════════════════════════════════════════════════════

spawn_xterm  # id=4
spawn_xterm  # id=5

APPLY_Q=$("$TILER_BIN" apply 1 4 2>&1)
assert_eq "Apply quadrants succeeds" "Ok" "$APPLY_Q"
sleep 1

# Check all 4 quadrant slots are approximately filled
ALL_GEOM=""
for wid in 1 2 4 5; do
  g=$(get_window_geometry "$wid")
  ALL_GEOM="$ALL_GEOM
$g"
done

# Check we have windows in each quadrant region
HAS_TL=$(echo "$ALL_GEOM" | awk '{if ($1 < 100 && $2 < 100) print "yes"}' | head -1)
HAS_TR=$(echo "$ALL_GEOM" | awk '{if ($1 > 800 && $1 < 1100 && $2 < 100) print "yes"}' | head -1)
HAS_BL=$(echo "$ALL_GEOM" | awk '{if ($1 < 100 && $2 > 400) print "yes"}' | head -1)
HAS_BR=$(echo "$ALL_GEOM" | awk '{if ($1 > 800 && $1 < 1100 && $2 > 400) print "yes"}' | head -1)

assert_eq "Quadrants: has top-left window" "yes" "${HAS_TL:-no}"
assert_eq "Quadrants: has top-right window" "yes" "${HAS_TR:-no}"
assert_eq "Quadrants: has bottom-left window" "yes" "${HAS_BL:-no}"
assert_eq "Quadrants: has bottom-right window" "yes" "${HAS_BR:-no}"

# ═════════════════════════════════════════════════════════════════════════════
printf "\n\033[1m── Window Type Filtering (FR-006) ──\033[0m\n"
# ═════════════════════════════════════════════════════════════════════════════

WTYPE=$(gdbus call --session -d "$DBUS_NAME" -o "$DBUS_PATH" \
  -m "$DBUS_IFACE.GetWindowType" "uint64 1" 2>&1)
assert_contains "GetWindowType returns toplevel" "$WTYPE" "toplevel"

# ═════════════════════════════════════════════════════════════════════════════
printf "\n\033[1m── Menu Command (FR-012) ──\033[0m\n"
# ═════════════════════════════════════════════════════════════════════════════

MENU=$("$TILER_BIN" menu 2>&1)
assert_eq "Menu toggle command succeeds" "Ok" "$MENU"

DAEMON_LOG3=$(cat /tmp/e2e-daemon.log 2>/dev/null)
assert_not_contains "No errors in daemon log" "$DAEMON_LOG3" "error handling"

# ═════════════════════════════════════════════════════════════════════════════
printf "\n\033[1m── Daemon Log ──\033[0m\n"
# ═════════════════════════════════════════════════════════════════════════════

grep "^\[tiler\]" /tmp/e2e-daemon.log 2>/dev/null || true

echo "$PASS $FAIL" > "$RESULT_FILE"
INNER_EOF

chmod +x "$INNER_SCRIPT"

bold "Running E2E tests in headless GNOME Shell..."
echo ""

dbus-run-session -- bash "$INNER_SCRIPT" "$TILER_BIN" "$RESULT_FILE" 2>&1 \
  | grep -v "^dbus-daemon\|^amdgpu\|^SpiRegistry\|^discover\|^goa-daemon\|Gjs-CRITICAL\|Stack trace\|^#[0-9]\|evolution\|gnome-shell-calendar\|libedbus\|libgoaident\|GVFS-Remote\|fusermount\|xdg-desktop-portal\|GLib-GIO-CRITICAL\|Gdk-Message\|error: fuse\|process:.*WARNING\|^A connection\|RNING"

rm -f "$INNER_SCRIPT"

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
