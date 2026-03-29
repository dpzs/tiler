#!/usr/bin/env bash
# stacking-test.sh — Test window stacking order with multi-monitor screenshots.
#
# Verifies windows are NOT buried behind each other after tiling operations.
# Takes screenshots at each step for visual inspection.
#
# Usage: nix develop --command bash scripts/stacking-test.sh

set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PROJECT_DIR="$(cd "$SCRIPT_DIR/.." && pwd)"
TARGET="x86_64-unknown-linux-musl"
TILER_BIN="$PROJECT_DIR/target/$TARGET/debug/tiler"
EXT_SRC="$PROJECT_DIR/extension"
EXT_DEST="$HOME/.local/share/gnome-shell/extensions/tiler@gnome-extensions"
SCREENSHOT_DIR="$PROJECT_DIR/test-screenshots"

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

mkdir -p "$SCREENSHOT_DIR"

# ── Write inner test script ──────────────────────────────────────────────────
INNER_SCRIPT=$(mktemp)
RESULT_FILE=$(mktemp)

cat > "$INNER_SCRIPT" << 'INNER_EOF'
#!/usr/bin/env bash
set -euo pipefail

TILER_BIN="$1"
RESULT_FILE="$2"
SCREENSHOT_DIR="$3"
ITERATION="$4"

DBUS_NAME="org.gnome.Shell.Extensions.Tiler"
DBUS_PATH="/org/gnome/Shell/Extensions/Tiler"
DBUS_IFACE="$DBUS_NAME"
TOL=50

PASS=0; FAIL=0

green() { printf "\033[32m  PASS: %s\033[0m\n" "$*"; PASS=$((PASS + 1)); }
red()   { printf "\033[31m  FAIL: %s\033[0m\n" "$*"; FAIL=$((FAIL + 1)); }

assert_eq() {
  local name="$1" expected="$2" actual="$3"
  if [ "$expected" = "$actual" ]; then green "$name"; else red "$name (expected=$expected got=$actual)"; fi
}

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

assert_geom_near() {
  local name="$1" expected="$2" actual="$3"
  if _geom_matches "$expected" "$actual"; then
    green "$name"
    return
  fi
  for retry in 1 2 3; do
    sleep 1
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

# Get the stacking order of all windows (bottom-to-top), returning stable_sequence ids
get_stacking_order() {
  eval_val "
    let actors = global.get_window_actors();
    let order = [];
    for (let a of actors) {
      let mw = a.get_meta_window();
      if (mw.get_window_type() === 0)
        order.push(mw.get_stable_sequence());
    }
    JSON.stringify(order);
  "
}

# Check if window A is above window B in stacking order
# Returns "true" if A is above B (A has higher index in actors list)
is_window_above() {
  local win_a="$1" win_b="$2"
  eval_val "
    let actors = global.get_window_actors();
    let idxA = -1, idxB = -1;
    for (let i = 0; i < actors.length; i++) {
      let seq = actors[i].get_meta_window().get_stable_sequence();
      if (seq === $win_a) idxA = i;
      if (seq === $win_b) idxB = i;
    }
    (idxA > idxB).toString();
  "
}

# Take a screenshot using GNOME Shell's built-in screenshot API
take_screenshot() {
  local name="$1"
  local filepath="$SCREENSHOT_DIR/iter${ITERATION}_${name}.png"
  # Use org.gnome.Shell.Screenshot D-Bus API
  gdbus call --session \
    -d org.gnome.Shell.Screenshot \
    -o /org/gnome/Shell/Screenshot \
    -m org.gnome.Shell.Screenshot.Screenshot \
    false true "$filepath" >/dev/null 2>&1 || {
    # Alternative: try org.gnome.Shell.Screenshot.ScreenshotArea for full desktop
    echo "  (screenshot skipped: API unavailable)"
    return 0
  }
  if [ -f "$filepath" ]; then
    echo "  Screenshot: $filepath"
  fi
}

# ── Daemon setup ──────────────────────────────────────────────────────────────

DAEMON_PID=""
SHELL_PID=""

cleanup() {
  [ -n "$DAEMON_PID" ] && kill "$DAEMON_PID" 2>/dev/null || true
  [ -n "$SHELL_PID" ] && kill "$SHELL_PID" 2>/dev/null || true
  wait 2>/dev/null || true
  rm -f /tmp/e2e-daemon.log ~/tiler.log
}
trap cleanup EXIT

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

rm -f ~/tiler.log
"$TILER_BIN" daemon >/tmp/e2e-daemon.log 2>&1 &
DAEMON_PID=$!
sleep 1

STATUS=$("$TILER_BIN" status 2>&1)
assert_eq "Daemon responds" "Ok" "$STATUS"

# ═════════════════════════════════════════════════════════════════════════════
printf "\n\033[1m══ STACKING ORDER TEST (iteration $ITERATION) ══\033[0m\n"
# ═════════════════════════════════════════════════════════════════════════════

# ── Phase 1: Stack tiling — check windows aren't overlapping incorrectly ────
printf "\n\033[1m── Phase 1: 3-window stack — stacking order ──\033[0m\n"

spawn_xterm  # id=1
spawn_xterm  # id=2
spawn_xterm  # id=3

take_screenshot "01_three_windows_stacked"

ORDER=$(get_stacking_order)
echo "  Stacking order (bottom→top): $ORDER"

# In a 3-window vertical stack, all windows should be visible (no overlap)
# Check geometries don't fully overlap each other
GEOM1=$(get_window_geometry 1)
GEOM2=$(get_window_geometry 2)
GEOM3=$(get_window_geometry 3)
echo "  Window 1: $GEOM1"
echo "  Window 2: $GEOM2"
echo "  Window 3: $GEOM3"

# Each window should have distinct Y positions (vertical stack)
Y1=$(echo "$GEOM1" | awk '{print $2}')
Y2=$(echo "$GEOM2" | awk '{print $2}')
Y3=$(echo "$GEOM3" | awk '{print $2}')
if [ "$Y1" != "$Y2" ] && [ "$Y2" != "$Y3" ] && [ "$Y1" != "$Y3" ]; then
  green "Stack: all 3 windows have distinct Y positions (no full overlap)"
else
  red "Stack: windows may be overlapping (Y1=$Y1 Y2=$Y2 Y3=$Y3)"
fi

# ── Phase 2: Layout changes — do windows stay on top? ───────────────────────
printf "\n\033[1m── Phase 2: Side-by-Side layout — stacking after retile ──\033[0m\n"

"$TILER_BIN" apply 1 2 2>&1 >/dev/null  # side-by-side
sleep 1

take_screenshot "02_side_by_side"

ORDER2=$(get_stacking_order)
echo "  Stacking order after SBS: $ORDER2"

GEOM2_SBS=$(get_window_geometry 2)
GEOM1_SBS=$(get_window_geometry 1)
echo "  Window 2 (left): $GEOM2_SBS"
echo "  Window 1 (right): $GEOM1_SBS"

# After side-by-side, both windows should be visible (not stacked on top of each other)
X2=$(echo "$GEOM2_SBS" | awk '{print $1}')
X1=$(echo "$GEOM1_SBS" | awk '{print $1}')
if [ "$X2" != "$X1" ]; then
  green "SBS: windows have distinct X positions"
else
  red "SBS: windows have same X=$X1 — one is behind the other!"
fi

# ── Phase 3: Close window to reduce count, then verify retile ───────────────
printf "\n\033[1m── Phase 3: Close window 2 — daemon-tracked retile ──\033[0m\n"

close_window 2

take_screenshot "03_after_close_win2"

# After closing, SBS should retile remaining 2 windows on monitor 0
GEOM3_AFTER=$(get_window_geometry 3)
GEOM1_AFTER=$(get_window_geometry 1)
echo "  Window 3: $GEOM3_AFTER"
echo "  Window 1: $GEOM1_AFTER"

X3=$(echo "$GEOM3_AFTER" | awk '{print $1}')
X1=$(echo "$GEOM1_AFTER" | awk '{print $1}')
if [ "$X3" != "$X1" ]; then
  green "Close retile: windows have distinct X positions after SBS retile"
else
  red "Close retile: windows have same X=$X1 — SBS retile failed!"
fi

# Check stacking: window 3 should be raised above window 1
ABOVE=$(is_window_above 3 1)
if [ "$ABOVE" = "true" ]; then
  green "Close retile: window 3 is above window 1 (correct stacking)"
else
  red "Close retile: window 3 is BEHIND window 1"
fi

# ── Phase 4: Open new window — does it go on top? ──────────────────────────
printf "\n\033[1m── Phase 4: New window opens — stacking check ──\033[0m\n"

spawn_xterm  # id=4

take_screenshot "04_new_window_opened"

ORDER3=$(get_stacking_order)
echo "  Stacking order after new window: $ORDER3"

GEOM4=$(get_window_geometry 4)
echo "  Window 4: $GEOM4"

# Window 4 should be on top of the stack
ABOVE_4_1=$(is_window_above 4 1)
ABOVE_4_3=$(is_window_above 4 3)
if [ "$ABOVE_4_1" = "true" ] && [ "$ABOVE_4_3" = "true" ]; then
  green "New window 4 is on top of existing windows"
else
  red "New window 4 is BEHIND existing windows — stacking bug!"
fi

# ── Phase 5: Close a window — retile check ──────────────────────────────────
printf "\n\033[1m── Phase 5: Close window 3 — retile stacking ──\033[0m\n"

close_window 3

take_screenshot "05_after_close"

ORDER4=$(get_stacking_order)
echo "  Stacking order after close: $ORDER4"

# Remaining windows (1, 4) should be tiled, not overlapping
GEOM4_AFTER=$(get_window_geometry 4)
GEOM1_AFTER=$(get_window_geometry 1)
echo "  Window 4: $GEOM4_AFTER"
echo "  Window 1: $GEOM1_AFTER"

# With SBS layout still active and 2 windows remaining, they should be side-by-side
X4=$(echo "$GEOM4_AFTER" | awk '{print $1}')
X1_AFTER=$(echo "$GEOM1_AFTER" | awk '{print $1}')
if [ "$X4" != "$X1_AFTER" ]; then
  green "After close: remaining windows have distinct X positions (SBS retile)"
else
  # Fall back to checking Y positions (stack layout)
  Y4=$(echo "$GEOM4_AFTER" | awk '{print $2}')
  Y1_AFTER=$(echo "$GEOM1_AFTER" | awk '{print $2}')
  if [ "$Y4" != "$Y1_AFTER" ]; then
    green "After close: remaining windows have distinct Y positions (stack retile)"
  else
    red "After close: windows overlap at ($X4,$Y4) — retile didn't separate them"
  fi
fi

# ── Phase 6: Apply fullscreen layout — last window should be visible ────────
printf "\n\033[1m── Phase 6: Fullscreen layout — topmost check ──\033[0m\n"

"$TILER_BIN" apply 1 1 2>&1 >/dev/null  # fullscreen
sleep 1

take_screenshot "06_fullscreen"

ORDER5=$(get_stacking_order)
echo "  Stacking order after fullscreen: $ORDER5"

# In fullscreen, the top window of the stack should actually be on top visually
# Parse order to find which is last (topmost)
TOPMOST=$(echo "$ORDER5" | sed 's/.*,//' | tr -d ']')
echo "  Topmost window in stacking: $TOPMOST"

# ── Phase 7: Quadrants layout ──────────────────────────────────────────────
printf "\n\033[1m── Phase 7: Quadrants layout — all visible ──\033[0m\n"

spawn_xterm  # id=5
spawn_xterm  # id=6
sleep 1

"$TILER_BIN" apply 1 4 2>&1 >/dev/null  # quadrants
sleep 1

take_screenshot "07_quadrants"

ORDER6=$(get_stacking_order)
echo "  Stacking order after quadrants: $ORDER6"

# All quadrant windows should have distinct positions
# With daemon tracking correct, windows 4, 1, 5, 6 should fill 4 quadrant slots
ALL_GEOM=""
for wid in 1 4 5 6; do
  g=$(get_window_geometry "$wid")
  echo "  Window $wid: $g"
  ALL_GEOM="$ALL_GEOM$g\n"
done

# Check we have 4 distinct x,y combinations
UNIQUE=$(printf "$ALL_GEOM" | awk 'NF{print $1","$2}' | sort -u | wc -l)
if [ "$UNIQUE" -ge 4 ]; then
  green "Quadrants: $UNIQUE distinct positions — all windows visible"
else
  red "Quadrants: only $UNIQUE distinct positions — windows overlapping!"
fi

# Verify each quadrant has a window (TL, TR, BL, BR)
HAS_TL=$(printf "$ALL_GEOM" | awk 'NF{if ($1 < 100 && $2 < 100) print "yes"}' | head -1)
HAS_TR=$(printf "$ALL_GEOM" | awk 'NF{if ($1 > 800 && $1 < 1100 && $2 < 100) print "yes"}' | head -1)
HAS_BL=$(printf "$ALL_GEOM" | awk 'NF{if ($1 < 100 && $2 > 400) print "yes"}' | head -1)
HAS_BR=$(printf "$ALL_GEOM" | awk 'NF{if ($1 > 800 && $1 < 1100 && $2 > 400) print "yes"}' | head -1)

if [ "${HAS_TL:-no}" = "yes" ] && [ "${HAS_TR:-no}" = "yes" ] && [ "${HAS_BL:-no}" = "yes" ] && [ "${HAS_BR:-no}" = "yes" ]; then
  green "Quadrants: all 4 quadrant slots occupied"
else
  red "Quadrants: missing slots (TL=${HAS_TL:-no} TR=${HAS_TR:-no} BL=${HAS_BL:-no} BR=${HAS_BR:-no})"
fi

# ── Phase 8: Rapid layout switching — stress test stacking ──────────────────
printf "\n\033[1m── Phase 8: Rapid layout switching — stress test ──\033[0m\n"

for layout in 1 2 3 4 1 2 3 4; do
  "$TILER_BIN" apply 1 "$layout" 2>&1 >/dev/null
  sleep 0.5
done
sleep 1

take_screenshot "08_after_rapid_switch"

ORDER7=$(get_stacking_order)
echo "  Stacking order after rapid switching: $ORDER7"

# Final check: daemon log for errors
DAEMON_LOG=$(cat ~/tiler.log 2>/dev/null)
ERROR_COUNT=$(echo "$DAEMON_LOG" | grep -c "ERROR" || true)
if [ "$ERROR_COUNT" -eq 0 ]; then
  green "No errors in daemon log"
else
  red "Found $ERROR_COUNT errors in daemon log"
  echo "$DAEMON_LOG" | grep "ERROR" | tail -5
fi

echo "$PASS $FAIL" > "$RESULT_FILE"
INNER_EOF

chmod +x "$INNER_SCRIPT"

# ── Run test iterations ──────────────────────────────────────────────────────
TOTAL_PASS=0
TOTAL_FAIL=0
MAX_ITERATIONS=${1:-3}

for iter in $(seq 1 "$MAX_ITERATIONS"); do
  bold ""
  bold "═══════════════════════════════════════════════════════════"
  bold "  ITERATION $iter of $MAX_ITERATIONS"
  bold "═══════════════════════════════════════════════════════════"

  dbus-run-session -- bash "$INNER_SCRIPT" "$TILER_BIN" "$RESULT_FILE" "$SCREENSHOT_DIR" "$iter" 2>&1 \
    | grep -v "^dbus-daemon\|^amdgpu\|^SpiRegistry\|^discover\|^goa-daemon\|Gjs-CRITICAL\|Stack trace\|^#[0-9]\|evolution\|gnome-shell-calendar\|libedbus\|libgoaident\|GVFS-Remote\|fusermount\|xdg-desktop-portal\|GLib-GIO-CRITICAL\|Gdk-Message\|error: fuse\|process:.*WARNING\|^A connection\|RNING"

  read P F < "$RESULT_FILE" 2>/dev/null || { P=0; F=1; }
  TOTAL_PASS=$((TOTAL_PASS + P))
  TOTAL_FAIL=$((TOTAL_FAIL + F))
done

rm -f "$INNER_SCRIPT" "$RESULT_FILE"

echo ""
bold "═══════════════════════════════════════════════════════════"
bold "  TOTAL: $TOTAL_PASS passed, $TOTAL_FAIL failed ($MAX_ITERATIONS iterations)"
bold "═══════════════════════════════════════════════════════════"
if [ "$TOTAL_FAIL" -eq 0 ]; then
    printf '\033[32m%s\033[0m\n' "All stacking tests passed across all iterations"
else
    printf '\033[31m%s\033[0m\n' "Stacking issues detected!"
fi

echo ""
bold "Screenshots saved to: $SCREENSHOT_DIR"
ls -la "$SCREENSHOT_DIR"/*.png 2>/dev/null || echo "  (no screenshots captured)"
