# devFlow2 Plan — swift-patch

SKIP_DEVFLOW_PLAN

## Overview

Address three unresolved issues from the tile-wave session on branch devflow/tile-wave.

## Dependency Graph

```
Lane 1: MoveWindow        Lane 2: Real zbus D-Bus     Lane 3: Nix Test
Implementation             Proxy + Daemon Event Loop   Improvements
(engine.rs, tests)         (zbus_proxy.rs, daemon.rs,  (nix/tests/*.sh)
                            main.rs)
     │                          │                          │
     │ (all parallel)           │                          │
     v                          v                          v
              Integration merge into devflow/swift-patch
```

## Lanes

### Lane 1: MoveWindow Implementation
**Goal:** Implement actual cross-monitor window movement when Shift+N is pressed in the menu.
**Files:**
- src/tiling/engine.rs (modify)
- tests/move_window_test.rs (create)
- tests/menu_engine_test.rs (modify)
**Acceptance:**
- `focused_window_id: Option<u64>` field tracks which window has focus
- `handle_focus_changed(window_id)` updates focused window
- `move_window_to_monitor(target)` moves focused window to fill target monitor
- Existing toplevel windows on target monitor displaced to stack screen
- Stack screen retiles after displacement
- If no focused window, MoveWindow is a safe no-op
- Menu closes after MoveWindow (already handled by state machine)
**Dependencies:** None

### Lane 2: Real zbus D-Bus Proxy + Daemon Event Loop
**Goal:** Replace MockGnomeProxy in daemon mode with real zbus proxy. Add D-Bus signal listener and event dispatch.
**Files:**
- src/gnome/dbus_proxy.rs (modify — add serde derives)
- src/gnome/zbus_proxy.rs (create)
- src/gnome/mod.rs (modify)
- src/main.rs (modify)
- src/daemon.rs (modify)
- tests/zbus_proxy_test.rs (create)
**Acceptance:**
- ZbusGnomeProxy implements GnomeProxy trait via zbus 5
- JSON deserialization for ListWindows/GetMonitors
- Signal listener converts D-Bus signals to Event enum via tokio::mpsc
- Daemon event loop uses tokio::select! over IPC + D-Bus signals
- All events dispatched to engine handlers (including focus changes)
- main.rs uses ZbusGnomeProxy in daemon mode
- Compile-time and deserialization tests pass
**Dependencies:** None

### Lane 3: Nix Test Improvements
**Goal:** Improve Nix test coverage with better structural tests and conditional nix eval.
**Files:**
- nix/tests/test-extension-derivation.sh (modify)
- nix/tests/test-module.sh (modify)
- nix/tests/test-package-derivation.sh (create)
- nix/tests/test-flake-structure.sh (create)
- nix/tests/run-all.sh (create)
- nix/tests/test-module.nix (create)
**Acceptance:**
- All shell tests pass without nix binary
- Conditional nix eval blocks run when nix is available
- package.nix has structural validation
- Flake structure has structural validation
- run-all.sh aggregates all test scripts
- No regressions in existing tests
**Dependencies:** None

## Parallelism Summary
- 3 lanes, 1 parallel batch (all independent)
- No file conflicts between lanes

## File Change Summary

| Lane | Creates | Modifies |
|------|---------|----------|
| 1 | tests/move_window_test.rs | src/tiling/engine.rs, tests/menu_engine_test.rs |
| 2 | src/gnome/zbus_proxy.rs, tests/zbus_proxy_test.rs | src/gnome/dbus_proxy.rs, src/gnome/mod.rs, src/main.rs, src/daemon.rs |
| 3 | nix/tests/test-package-derivation.sh, nix/tests/test-flake-structure.sh, nix/tests/run-all.sh, nix/tests/test-module.nix | nix/tests/test-extension-derivation.sh, nix/tests/test-module.sh |
