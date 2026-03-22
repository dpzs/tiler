# Tiler Bug Checklist — 2026-03-22

Comprehensive audit of the deployed tiler (commit b95c7ca). Findings from
code review, `cargo test`, live D-Bus interaction, and spawning kitty windows
against the running daemon+extension on a 3-monitor setup.

Monitors:
- Monitor 0: 1440x2560 (portrait, left) — stack screen
- Monitor 1: 2560x1600 (landscape, center)
- Monitor 2: 2560x1440 (landscape, right)

---

## THE UGLY — Critical / breaks core functionality

### U1. All windows get dragged to the stack screen
**File:** `src/tiling/engine.rs:136-138` (startup), `engine.rs:168-171` (window-open)
**Symptom:** Opening ANY window causes ALL toplevel windows from ALL monitors
to be yanked onto monitor 0 (the stack screen). Settings from monitor 2,
browser from monitor 1 — everything piles onto the portrait display.
**Root cause:** Both `startup()` and `handle_window_opened()` unconditionally
add every toplevel non-fullscreen window to `desktop.stack_windows`, regardless
of which monitor the window is on. Then `tile_stack()` positions all of them
on the stack screen rect.
**Expected:** Only windows already on the stack screen should be in the stack.
Windows on other monitors should be left alone.

### U2. New windows always assigned monitor_id = stack screen
**File:** `src/tiling/engine.rs:162`
**Symptom:** `handle_window_opened()` hardcodes `monitor_id: self.stack_screen_index as u32`
for every new window. The daemon never queries which monitor the window actually
appeared on. This means the internal tracking is wrong for every window that
opens on monitors 1 or 2.
**Impact:** All downstream logic that checks `w.monitor_id` (enforcement,
layout application, move-window) operates on stale/wrong data.

### U3. Config file is generated but never loaded
**File:** `src/main.rs` — no reference to `TilerConfig` or config path
**Symptom:** The NixOS module generates `/etc/tiler/config.toml` with
`stack_screen_position = "left"`, but `main.rs` never reads it. The
`stack_screen_index` is hardcoded to `0` on line 33.
**Impact:** The config system is dead code. Changing `stack_screen_position`
in NixOS config has zero effect.

### U4. Geometry-change signal storm during tiling
**File:** `src/daemon.rs:104-106`, `extension/extension.js:97-113`
**Symptom:** When `tile_stack()` calls `move_resize_window` for N windows,
the extension fires `position-changed` and `size-changed` signals for each
move. These arrive as `WindowGeometryChanged` events in the daemon's event
loop, potentially causing re-entrant tiling or enforcement snap-backs
mid-layout.
**Observed:** After opening a single kitty window, window positions are
scrambled — some land in swapped column/row slots, suggesting concurrent
tile operations are fighting each other.
**Fix needed:** Either suppress geometry signals while the daemon is actively
tiling, or add a "tiling in progress" flag that skips geometry-change handling.

### U5. Newly opened windows fail to tile (race condition)
**File:** `src/tiling/engine.rs:148-174`, `extension/dbus.js:158-168`
**Symptom:** A freshly opened kitty window (id:10) remained at its default
size (1440x1568) overlapping other windows instead of being placed in its
stack slot (expected 720x512).
**Root cause:** The `MoveResizeWindow` D-Bus call arrives before the window
is fully mapped/ready. The `unmaximize()` + `move_resize_frame()` sequence
in dbus.js may be silently ignored by Mutter for windows still initializing.
**Evidence:** id:10 at (0, 496, 1440, 1568) vs expected (0, 0, 720, 512).

---

## THE BAD — Significant logic errors / wrong behavior

### B1. Status command returns no useful information
**File:** `src/daemon.rs:53`
**Symptom:** `Command::Status => Response::Ok` — just returns "Ok" with no
payload. No window count, layout state, workspace info, enforcement status,
or anything actionable.
**CLI output:** `tiler status` prints `Ok` and exits.

### B2. Duplicate IPC server code (dead code)
**File:** `src/ipc/server.rs`
**Symptom:** `run_server()` is a standalone IPC server that duplicates the
command handling in `daemon.rs`. The daemon builds its own `UnixListener`
loop (daemon.rs:31-64) and never calls `run_server()`.
**Impact:** Dead code that will drift out of sync with the real handler.

### B3. Menu Escape key handled twice
**File:** `extension/menu.js:329-333`, daemon `menu/state.rs:58-59`
**Symptom:** When Escape is pressed in the menu overlay:
1. menu.js `_onKeyPressEvent` calls `this.hide()` immediately (destroys overlay)
2. menu.js ALSO fires the key callback to the daemon
3. Daemon processes Escape, transitions to Closed, calls `proxy.hide_menu()`
4. Extension's `HideMenu` D-Bus method calls `this._menuOverlay.hide()` again
**Impact:** Double-hide. The second `hide()` call tries to `removeChrome`
and `destroy` an already-destroyed overlay. May cause GNOME Shell warnings
or crashes depending on St/Clutter cleanup timing.

### B4. Menu overlay only covers the primary monitor
**File:** `extension/menu.js:112-116`
**Symptom:** The overlay is sized to `primaryMonitor` dimensions. On a
multi-monitor setup, the dim backdrop only covers one screen. Other monitors
remain fully visible and interactive — confusing when the menu is supposed
to be a modal overlay.

### B5. Layout presets drop excess windows silently
**File:** `src/tiling/preset.rs:4-10` (fullscreen), `13-24` (side-by-side), etc.
**Symptom:** `apply_fullscreen` takes 1 window, `apply_side_by_side` takes 2,
`apply_quadrants` takes 4. Extra windows are silently dropped — they receive
no position at all and remain wherever they were.
**Expected:** At minimum, excess windows should be stacked or minimized.
With 5 windows on a SideBySide layout, 3 windows become orphaned with no
defined position.

### B6. Enforcement snap-back uses wrong window set
**File:** `src/tiling/engine.rs:274-284`
**Symptom:** `handle_geometry_changed` computes expected positions by
filtering `desktop.stack_windows` for windows matching `monitor_id`. But
since all windows have `monitor_id = stack_screen_index` (bug U2), the
filter either catches everything (stack monitor) or nothing (other monitors).
Enforcement on non-stack monitors can never work.

### B7. Window workspace tracking is static after open
**File:** `src/tiling/engine.rs:160-162`
**Symptom:** A new window's `workspace_id` is set to `self.active_workspace`
at open time. If the user moves a window to a different workspace via GNOME,
the daemon never updates the tracked `workspace_id`. The window stays in the
wrong desktop's stack forever.
**Note:** There is no `WindowWorkspaceChanged` signal or D-Bus method to
detect workspace moves.

### B8. `apply_layout_to_monitor` can't find windows on non-stack monitors
**File:** `src/tiling/engine.rs:408-417`
**Symptom:** When applying a layout (e.g., SideBySide on monitor 1), the
code filters `stack_windows` for `w.monitor_id == monitor_id`. But since
`handle_window_opened` assigns all windows `monitor_id = stack_screen_index`,
no windows match monitor 1 or 2. Layout application on non-stack monitors
is always a no-op.

---

## THE GOOD (but still bugs) — Minor / cosmetic / test issues

### G1. Two integration tests fail due to live daemon
**File:** `tests/cli_dispatch_test.rs:8-26`, `tests/cli_dispatch_test.rs:29-47`
**Symptom:** `should_exit_nonzero_when_menu_and_no_daemon` and
`should_exit_nonzero_when_status_and_no_daemon` both FAIL because the tests
use the real `$XDG_RUNTIME_DIR` which contains a live `tiler.sock` from the
deployed daemon. The tests expect connection failure but get success.
**Fix:** Tests should use a temp dir for `XDG_RUNTIME_DIR`, or use a unique
socket path.

### G2. Pixel rounding gaps in layouts
**File:** `src/tiling/preset.rs`, `src/tiling/stack.rs`
**Symptom:** Integer division truncates. A 1441px-wide monitor with
SideBySide gives 720 + 720 = 1440, leaving a 1px gap at the right edge.
Same issue in stack layout for row heights on odd-height screens.
**Severity:** Cosmetic — visible as thin strips of desktop background between
tiles.

### G3. Monitor connector name fallback shows "Monitor-0"
**File:** `extension/dbus.js:192-208`
**Symptom:** The `_getMonitorConnector` method falls back to `Monitor-${index}`
but both GNOME <48 and >=48 paths may also return this fallback. In the live
system all 3 monitors show as "Monitor-0", "Monitor-1", "Monitor-2" instead
of their actual connector names (e.g., "DP-1", "HDMI-1").
**Impact:** Menu overview shows unhelpful monitor names instead of physical
connector IDs.

### G4. `ipc/mod.rs` exports but daemon doesn't use the module's server
**File:** `src/ipc/mod.rs`
```rust
pub mod client;
pub mod protocol;
pub mod server;
```
The `server` module is public but unused by any production code path.

### G5. Extension tests are structural only
**File:** `extension/tests/`
**Symptom:** All 100 extension tests pass, but they only check file existence,
string patterns, and XML structure. No tests actually execute D-Bus methods,
menu rendering, or signal emission. There is no behavioral test coverage for
the extension.

### G6. `WindowType` mismatch between model and engine
**File:** `src/model.rs:24-29` vs `src/tiling/engine.rs:113-115`
**Symptom:** The `model.rs` defines `WindowType` enum (`Normal`, `Dialog`,
`Popup`, `Splash`), but the engine uses string comparison
`wtype == "toplevel"`. The model's `WindowType` is never used by the engine —
the `Window` struct in `model.rs` with its `window_type: WindowType` field
is only used by the `filter.rs` module, which itself is never called from the
engine.
**Impact:** `filter::is_toplevel()` is dead code. The engine has its own
inline equivalent.

### G7. `VirtualDesktop::append_window` vs `push_window` ordering inconsistency
**File:** `src/model.rs:90-99`
**Symptom:** Startup uses `append_window` (back of stack), but
`handle_window_opened` uses `push_window` (front of stack). This means
pre-existing windows tile bottom-to-top by window list order, while newly
opened windows jump to the front. The ordering strategy is inconsistent.

---

## Summary

| Category | Count | Key theme |
|----------|-------|-----------|
| Ugly (critical) | 5 | All-windows-to-stack-screen, no config, race conditions |
| Bad (significant) | 8 | Dead code, double-fire, broken enforcement, missing data |
| Good (minor) | 7 | Test isolation, rounding, cosmetic, dead model code |
| **Total** | **20** | |

### Priority fix order
1. **U1 + U2 + U3**: Fix window-to-monitor assignment (the core tiling is broken)
2. **U4 + U5**: Suppress geometry signals during tiling, delay new-window tile
3. **B3**: Fix double Escape handling
4. **B6 + B8**: Fix enforcement/layout to use correct monitor data
5. **G1**: Fix test isolation
6. Everything else
