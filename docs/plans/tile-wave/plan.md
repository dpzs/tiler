# devFlow2 Plan — tile-wave

SKIP_DEVFLOW_PLAN

## Architecture Overview

Tiler is split into two components:
1. **Rust daemon/CLI** — Pure logic, state management, tiling algorithms, IPC, D-Bus client
2. **GNOME Shell Extension** — JavaScript, runs inside GNOME Shell, provides D-Bus API for window management + menu overlay rendering

Communication: Rust daemon <-> GNOME Shell Extension via D-Bus (`org.gnome.Shell.Extensions.Tiler`)

All Rust dependencies are pure-Rust (no C linking needed), making musl static linking trivial.

## Dependency Graph

```
Lane 1: Project Bootstrap
   │
   ├──────────────────────────────┐
   v                              v
Lane 2: Data Model &           Lane 3: IPC &
Layout Algorithms              Configuration
(parallel)                     (parallel)
   │                              │
   │         Lane 4: GNOME Shell  │
   │         Extension (parallel) │
   │              │               │
   v              v               v
Lane 5: Tiling Engine & Daemon Integration
   │
   v
Lane 6: NixOS Module & Final Integration
```

## Lanes

### Lane 1: Project Bootstrap (Foundation)
**Goal:** Working Cargo project that builds a static musl binary via Nix flake.
**Files:** Cargo.toml, flake.nix, nix/package.nix, src/main.rs (skeleton), src/lib.rs
**Acceptance:**
- `cargo build` succeeds
- `nix build` produces a static musl binary
- `ldd` reports "not a dynamic executable"
- devShell works
**FRs:** FR-020
**Dependencies:** None

### Lane 2: Data Model & Layout Algorithms (Pure Rust)
**Goal:** All data types and pure layout computation functions with full test coverage.
**Files:**
- src/model/mod.rs, window.rs, monitor.rs, desktop.rs, layout.rs
- src/tiling/mod.rs, stack.rs, preset.rs
- src/menu/mod.rs, state.rs
**Acceptance:**
- Window, Monitor, VirtualDesktop, LayoutPreset types defined
- Stack layout algorithm: given N windows + screen rect, returns positions (FR-008)
- Layout presets: fullscreen, side-by-side, top-bottom, quadrants (FR-014, FR-015)
- Menu state machine: Overview -> ZoomedIn -> Closed with all key transitions (FR-012-FR-017)
- Window filtering predicate: toplevel vs dialog vs fullscreen (FR-006, FR-007)
- Virtual desktop state isolation (FR-011)
- Property-based tests for layout algorithms
- Unit tests for menu state machine (all transitions)
**FRs:** FR-006, FR-007, FR-008, FR-011, FR-012-FR-017 (state logic)
**Dependencies:** Lane 1

### Lane 3: IPC & Configuration
**Goal:** Working Unix socket IPC between CLI and daemon, plus TOML config parsing.
**Files:**
- src/ipc/mod.rs, protocol.rs, server.rs, client.rs
- src/config/mod.rs, schema.rs
- src/cli.rs
- src/main.rs (CLI dispatch)
**Acceptance:**
- Command/response protocol defined and serde-serializable (FR-003)
- Unix socket server accepts connections at $XDG_RUNTIME_DIR/tiler.sock (FR-003)
- CLI client connects, sends commands, receives responses (FR-002)
- CLI returns non-zero exit code if daemon unreachable (FR-002)
- TOML config parsed from ~/.config/tiler/config.toml with defaults (FR-019)
- Integration tests: IPC roundtrip over real Unix socket
**FRs:** FR-002, FR-003, FR-019
**Dependencies:** Lane 1

### Lane 4: GNOME Shell Extension
**Goal:** Working GNOME Shell extension exposing D-Bus API for window management and menu overlay.
**Files:**
- extension/metadata.json
- extension/extension.js
- extension/menu.js (menu overlay rendering)
- extension/dbus.js (D-Bus service definition)
**Acceptance:**
- Extension registers D-Bus service `org.gnome.Shell.Extensions.Tiler`
- D-Bus methods: ListWindows, MoveResizeWindow, GetMonitors, GetActiveWorkspace, GetWindowType, IsFullscreen
- D-Bus signals: WindowOpened, WindowClosed, WindowFocusChanged, WorkspaceChanged, WindowFullscreenChanged, WindowGeometryChanged
- Menu overlay: ShowOverview (renders monitor layout), ShowZoomed (renders layout options), Hide
- Menu forwards key events to daemon via D-Bus signal: MenuKeyPressed(key, modifiers)
- Extension installs cleanly on GNOME 45+
**FRs:** FR-001 (integration), FR-005 (events), FR-012 (menu rendering)
**Dependencies:** Lane 1 (for D-Bus interface definition alignment)

### Lane 5: Tiling Engine & Daemon Integration
**Goal:** Fully working daemon that connects to the GNOME Shell extension, processes events, and tiles windows.
**Files:**
- src/gnome/mod.rs, dbus_proxy.rs, event.rs, monitor_discovery.rs
- src/tiling/mod.rs (update), engine.rs, enforcement.rs
- src/daemon.rs
- src/main.rs (daemon mode wiring)
**Acceptance:**
- zbus proxy traits match the extension's D-Bus API (FR-001)
- Daemon connects to extension on startup (FR-001)
- Startup tiling: enumerates existing windows, tiles on stack screen (FR-004)
- New window detection: moves to stack screen (FR-005)
- Window close: auto re-tiles stack and non-stack screens (FR-009, FR-010)
- Virtual desktop isolation: per-desktop state (FR-011)
- Menu command flow: CLI -> IPC -> daemon -> D-Bus -> extension show menu (FR-012)
- Menu actions: Shift+N move, N zoom, 1-4 layout, 9/0 enforce, Esc dismiss (FR-013-FR-017)
- Layout enforcement: snap-back on user move when enabled (FR-018)
- Signal handling: clean exit on SIGTERM/SIGINT (FR-001)
- Integration tests with mock D-Bus backend
**FRs:** FR-001, FR-004, FR-005, FR-009, FR-010, FR-011, FR-013, FR-016, FR-018
**Dependencies:** Lanes 2, 3, 4

### Lane 6: NixOS Module & Final Integration
**Goal:** Complete NixOS module, extension packaging, final integration testing.
**Files:**
- nix/module.nix
- flake.nix (update with extension packaging)
**Acceptance:**
- NixOS module installs tiler binary (FR-021)
- NixOS module installs GNOME Shell extension (FR-021)
- NixOS module creates systemd user service for daemon (FR-021)
- NixOS module provides configurable keybinding option (FR-021)
- NixOS module generates TOML config from options (FR-021)
- Full test suite passes
**FRs:** FR-021
**Dependencies:** Lane 5

## Parallelism Summary
- 6 lanes, 4 sequential batches
- Batch 1: Lane 1 (bootstrap)
- Batch 2: Lanes 2, 3, 4 (parallel — independent after bootstrap)
- Batch 3: Lane 5 (integrates outputs of 2, 3, 4)
- Batch 4: Lane 6 (NixOS module, depends on everything)

## File Change Summary

| Lane | Creates | Modifies |
|------|---------|----------|
| 1 | Cargo.toml, flake.nix, nix/package.nix, src/main.rs, src/lib.rs | - |
| 2 | src/model/*.rs, src/tiling/stack.rs, src/tiling/preset.rs, src/menu/state.rs | src/lib.rs |
| 3 | src/ipc/*.rs, src/config/*.rs, src/cli.rs | src/main.rs, src/lib.rs |
| 4 | extension/metadata.json, extension/extension.js, extension/menu.js, extension/dbus.js | - |
| 5 | src/gnome/*.rs, src/tiling/engine.rs, src/tiling/enforcement.rs, src/daemon.rs | src/main.rs, src/lib.rs |
| 6 | nix/module.nix | flake.nix |
