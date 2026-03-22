# devFlow2 Plan — swift-patch

SKIP_DEVFLOW_PLAN

## Task
Fix 14 of 20 bugs identified in the tiler audit (docs/plans/swift-menu/bug-checklist.md).
Defer 6 bugs needing design decisions (B1, B5, B7, G3, G5, G7).

## Dependency Graph

```
Lane 1: Core Engine Fixes (U1+U2+U4, auto-resolves B6+B8)
   |
   v
Lane 2: Config Loading (U3)

Lane 3: Extension Fixes (U5+B3+B4)  -- independent, parallel with Lane 1
Lane 4: Housekeeping (G1+G2+B2/G4+G6) -- independent, parallel with Lane 1
```

Batches:
- Batch 1: Lanes 1, 3, 4 (parallel)
- Batch 2: Lane 2 (after Lane 1)

## Lane 1: Core Monitor Assignment + Tiling Guard

**Bugs:** U1, U2, U4 (B6, B8 resolved automatically)

**Goal:** Windows are only added to the stack when they are on the stack screen.
New windows get their actual monitor_id. Geometry events during active tiling
are suppressed.

**Key design decision (U2):** Add `monitor_id: u32` to `Event::WindowOpened`
and `handle_window_opened` signature. The extension already knows the monitor
when a window opens. No new D-Bus method needed.

**Changes:**
- `src/gnome/event.rs`: Add `monitor_id: u32` to `Event::WindowOpened`
- `src/gnome/zbus_proxy.rs`: Pass monitor_id from signal (requires D-Bus signal change)
- `extension/extension.js`: Include monitor in WindowOpened signal
- `extension/dbus.js`: Update signal signature, add GetWindowMonitor method
- `extension/dbus-interface.xml`: Update signal + add method
- `src/gnome/dbus_proxy.rs`: Update GnomeProxy trait + MockGnomeProxy
- `src/tiling/engine.rs`:
  - `startup()`: Only append to stack if window.monitor_id == stack_screen_index
  - `handle_window_opened()`: Accept + use monitor_id parameter
  - Add `is_tiling: bool` field, set during tile_stack/apply_layout
  - `handle_geometry_changed()`: Skip if is_tiling
- `src/daemon.rs`: Pass monitor_id through dispatch_event
- Update ~10 test files for new signatures

**Acceptance criteria:**
- Opening a window on monitor 1 does NOT move windows from other monitors
- Windows on non-stack monitors are tracked but not in stack_windows
- Geometry events during tiling produce zero extra move_resize calls
- All existing tests pass (with updated signatures)

## Lane 2: Config Loading

**Bugs:** U3

**Goal:** The daemon reads /etc/tiler/config.toml and uses stack_screen_position
to select the correct monitor as the stack screen.

**Changes:**
- `src/config/schema.rs`: Add `resolve_stack_screen_index(monitors)` method
- `src/cli.rs`: Add optional `--config` to Daemon subcommand
- `src/main.rs`: Load config, resolve index, pass to run_daemon
- `nix/module.nix`: Add --config flag to ExecStart

**Acceptance criteria:**
- `stack_screen_position = "right"` uses rightmost monitor
- Missing config file uses defaults (leftmost = index 0)
- Existing config_test.rs tests pass

## Lane 3: Extension Fixes

**Bugs:** U5, B3, B4

**Goal:** New windows are ready before tiling. Menu Escape fires once. Overlay
covers all monitors.

**Changes:**
- `extension/extension.js`: Defer WindowOpened signal until window's first-frame
- `extension/menu.js`:
  - Remove local `this.hide()` on Escape (let daemon handle it)
  - Size overlay to bounding box of all monitors

**Acceptance criteria:**
- Rapidly opening windows results in correct stack positions (no overlapping)
- Pressing Escape hides menu exactly once (no GNOME Shell errors)
- Menu dim overlay covers all 3 monitors

## Lane 4: Housekeeping

**Bugs:** G1, G2, B2/G4, G6

**Goal:** Tests pass in CI with live daemon. No pixel gaps. No dead code.

**Changes:**
- `tests/cli_dispatch_test.rs`: Override XDG_RUNTIME_DIR to temp dir
- `src/tiling/preset.rs`: Last slot absorbs remainder pixels
- `src/tiling/stack.rs`: Last row/column absorbs remainder
- Delete `src/ipc/server.rs`, remove from `src/ipc/mod.rs`
- Delete `src/tiling/filter.rs`, remove from `src/tiling/mod.rs`
- Clean up `src/model.rs` WindowType if unused
- Update/delete affected tests (ipc_server_test, window_filter_test)

**Acceptance criteria:**
- cli_dispatch_test passes even with live daemon
- Odd-dimension monitors produce gap-free layouts
- `cargo build` succeeds with no dead code
- All remaining tests pass

## Deferred Bugs

| Bug | Reason |
|-----|--------|
| B1 (status payload) | Needs design: what data to return |
| B5 (excess windows) | Needs design: minimize vs stack vs hide |
| B7 (workspace tracking) | Moderate scope, new signal chain |
| G3 (connector names) | Needs GNOME version testing |
| G5 (extension tests) | Infrastructure work, not a bug |
| G7 (push vs append) | May be intentional, needs UX discussion |
