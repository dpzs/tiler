# devFlow2 Plan — swift-menu

SKIP_DEVFLOW_PLAN

## Task
Fix two critical issues preventing the tiler menu from functioning:
1. MenuKeyPressed events silently discarded (daemon.rs TODO stub)
2. No daemon→extension mechanism to show/hide menu overlay

## Architecture Decision
Add ShowMenu, ShowMenuZoomed, HideMenu as D-Bus methods on the existing extension interface.
The daemon calls these via ZbusGnomeProxy, same pattern as ListWindows/MoveResizeWindow.

## Lanes

### Lane 1: Daemon→Extension menu control (Issue 2)
**Goal:** When menu state changes, daemon tells extension to show/hide overlay.

Files:
- extension/dbus.js — Add ShowMenu/ShowMenuZoomed/HideMenu to INTERFACE_XML + implementations + setMenuOverlay()
- extension/extension.js — Wire setMenuOverlay in enable()
- src/gnome/dbus_proxy.rs — Add 3 methods to GnomeProxy trait + MockGnomeProxy with call logging
- src/gnome/zbus_proxy.rs — Add 3 methods to zbus Tiler trait + ZbusGnomeProxy impl
- src/tiling/engine.rs — Call proxy show/hide/zoom on state transitions in handle_menu_input
- tests/menu_engine_test.rs — ~10 tests for proxy calls
- tests/gnome_proxy_test.rs — 3 tests for mock methods

### Lane 2: MenuKeyPressed dispatch (Issue 1) — blocked by Lane 1
**Goal:** Parse Clutter key names into MenuInput and dispatch through engine.

Files:
- src/menu/key_parse.rs — New: parse_menu_key(key, modifiers, state) -> Option<MenuInput>
- src/menu/mod.rs — Add pub mod key_parse
- src/daemon.rs — Replace TODO with parse + dispatch
- tests/menu_key_parse_test.rs — ~12 unit tests
- tests/daemon_event_test.rs — 2-3 smoke tests

## Dependency Graph
```
Lane 1 (menu control infra)
    │
    ▼
Lane 2 (key dispatch)
```
