# Tiler — Requirements Specification

## 1. Overview

### 1.1 Purpose
Tiler is a Rust-based tiling window manager utility for GNOME on Wayland (NixOS, musl static linked). It automatically tiles new windows onto a dedicated stack screen, and provides a keyboard-driven floating menu for moving and laying out windows across multiple monitors and virtual desktops.

### 1.2 Scope

**In scope:**
- Automatic window tiling on a dedicated stack screen
- Keyboard-driven floating menu for window management
- Layout enforcement (windows snap back to assigned positions)
- Multi-monitor support (2-3 monitors)
- Per-virtual-desktop independent tiling state
- Moving windows between monitors with automatic displacement
- Four layout presets for non-stack monitors
- Unix socket IPC between daemon and CLI
- NixOS flake with full NixOS module (package, systemd user service, keybinding configuration)
- TOML configuration file with NixOS module integration

**Out of scope:**
- Window animations
- Drag-and-drop window rearrangement
- Multi-user support
- Per-application rules or configurations
- Touchscreen or gesture support
- X11 support (Wayland only)
- State persistence across daemon restarts

### 1.3 Users & Stakeholders
Single desktop user running GNOME on Wayland with NixOS. No multi-user scenarios. The user manages windows across 2-3 monitors with multiple virtual desktops.

### 1.4 Glossary
- **Stack screen**: The leftmost monitor, where all new windows are placed and tiled in a vertical stack layout.
- **Hot window**: The window that had keyboard focus at the moment the global keybinding was pressed.
- **Enforcement mode**: When active, the tiler snaps windows back to their assigned tile positions if the user manually moves or resizes them.
- **Layout preset**: One of four predefined window arrangements for a non-stack monitor: fullscreen, side-by-side, top-bottom, or quadrants.
- **Toplevel window**: A primary application window (not a dialog, popup, splash screen, or notification).
- **Virtual desktop**: A GNOME workspace. Each virtual desktop maintains its own independent tiling state.
- **Displacement**: When a window is moved to a monitor via Shift+N, all existing windows on that monitor are moved back to the stack screen.

## 2. Functional Requirements

### 2.1 Daemon & Lifecycle

#### FR-001: Daemon Mode
**Description:** The system must run as a long-lived daemon process that listens for GNOME Wayland window events (window open, close, focus change, workspace change).
**Acceptance Criteria:**
- [ ] Daemon starts via systemd user service or manual invocation
- [ ] Daemon connects to GNOME Wayland and receives window lifecycle events
- [ ] Daemon maintains an in-memory model of all toplevel windows, their positions, sizes, and assigned monitors
- [ ] Daemon exits cleanly on SIGTERM/SIGINT
**Priority**: Must-have

#### FR-002: CLI Command Mode
**Description:** The same binary must support a command mode (based on CLI arguments) that communicates with the running daemon via a Unix domain socket.
**Acceptance Criteria:**
- [ ] Running the binary with a command argument (e.g., `tiler menu`) sends a command to the daemon and exits
- [ ] CLI returns a non-zero exit code and an error message if the daemon is not running
- [ ] CLI operations complete within 200ms
**Priority**: Must-have

#### FR-003: Unix Socket IPC
**Description:** The daemon must listen on a Unix domain socket for commands from the CLI.
**Acceptance Criteria:**
- [ ] Socket is created at a well-known path (e.g., `$XDG_RUNTIME_DIR/tiler.sock`)
- [ ] Daemon accepts connections and processes commands sequentially
- [ ] Socket is removed on clean daemon shutdown
**Priority**: Must-have

#### FR-004: Startup Tiling
**Description:** On daemon startup, the system must detect all existing toplevel windows on the current virtual desktop and tile them on the stack screen.
**Acceptance Criteria:**
- [ ] All existing toplevel windows on the current virtual desktop are enumerated on startup
- [ ] Windows are moved to the stack screen and arranged according to the stack layout rules (FR-008)
- [ ] Fullscreen windows are excluded from tiling (FR-007)
**Priority**: Must-have

### 2.2 Window Detection & Filtering

#### FR-005: New Window Detection
**Description:** The system must detect when a new toplevel window is created on any monitor and move it to the stack screen.
**Acceptance Criteria:**
- [ ] New toplevel windows are detected regardless of which monitor they spawn on
- [ ] Detected windows are moved to the stack screen within one event loop cycle
- [ ] The stack screen re-tiles to accommodate the new window
**Priority**: Must-have

#### FR-006: Non-Toplevel Window Exclusion
**Description:** The system must ignore non-toplevel windows (dialogs, popups, file pickers, notifications, splash screens, transient windows).
**Acceptance Criteria:**
- [ ] Dialog windows are not moved or tiled
- [ ] Popup and transient windows are not moved or tiled
- [ ] Notification windows are not moved or tiled
- [ ] Splash screens are not moved or tiled
**Priority**: Must-have

#### FR-007: Fullscreen Window Exclusion
**Description:** The system must not move or tile windows that are in fullscreen mode.
**Acceptance Criteria:**
- [ ] Windows in fullscreen mode are excluded from all tiling operations
- [ ] If a tiled window enters fullscreen, it is removed from the tiling model until it exits fullscreen
- [ ] If a fullscreen window exits fullscreen, it is re-added to the tiling model and moved to the stack screen
**Priority**: Must-have

### 2.3 Stack Screen Layout

#### FR-008: Vertical Stack Layout
**Description:** The stack screen (always the leftmost monitor) must arrange windows in a vertical stack, with a maximum of 5 windows per column. When more than 5 windows are present, additional columns are added.
**Acceptance Criteria:**
- [ ] With 1-5 windows: single column, windows stacked vertically with equal height, full width of the screen
- [ ] With 6-10 windows: two columns, each column up to 5 windows stacked vertically, columns split the screen width equally
- [ ] With 11-15 windows: three columns, same pattern
- [ ] Columns continue to be added with no upper limit
- [ ] The newest window (by creation time) is placed at the top of the first column, with older windows below
**Priority**: Must-have

#### FR-009: Stack Auto Re-tile on Window Close
**Description:** When a window is closed on the stack screen, the remaining windows must be re-arranged automatically.
**Acceptance Criteria:**
- [ ] Remaining windows re-tile to fill the available space within one event loop cycle
- [ ] Column count decreases if the total number of windows permits (e.g., closing a window to go from 6 to 5 collapses from two columns to one)
**Priority**: Must-have

### 2.4 Non-Stack Screen Auto Re-tile

#### FR-010: Auto Re-tile on Window Close (Non-Stack Screens)
**Description:** When a window is closed on a non-stack monitor, the remaining windows on that monitor must be re-arranged automatically according to the current layout preset for that monitor.
**Acceptance Criteria:**
- [ ] Remaining windows re-tile according to the active layout preset for that monitor
- [ ] If no layout preset has been set, remaining windows are left in their current positions
- [ ] Empty positions in a layout remain empty (no windows are pulled from the stack to fill them)
**Priority**: Must-have

### 2.5 Virtual Desktop Isolation

#### FR-011: Per-Desktop Independent State
**Description:** Each virtual desktop (GNOME workspace) must maintain its own independent tiling state. The tiler must only operate on windows belonging to the current virtual desktop.
**Acceptance Criteria:**
- [ ] Switching virtual desktops does not affect the tiling state of other desktops
- [ ] Each desktop has its own stack screen state and layout preset state per monitor
- [ ] Windows created on a specific desktop are tracked under that desktop's state
- [ ] Window events on non-active desktops are queued and processed when that desktop becomes active, or tracked passively
**Priority**: Must-have

### 2.6 Floating Menu

#### FR-012: Menu Trigger
**Description:** When the CLI receives a menu command (triggered by the global keybinding), the daemon must display a floating overlay menu in the center of the primary screen.
**Acceptance Criteria:**
- [ ] The menu appears as a Wayland overlay surface centered on the primary screen
- [ ] The menu renders a visual representation of all connected monitors, numbered left to right starting at 1
- [ ] The menu captures keyboard input exclusively while open
- [ ] The menu is visually distinct from normal application windows (overlay/floating appearance)
**Priority**: Must-have

#### FR-013: Menu — Move Window to Monitor (Shift+N)
**Description:** While the menu is in the overview state, pressing Shift+<monitor number> must move the hot window to that monitor, fill the screen, and displace all existing windows on that monitor to the stack screen.
**Acceptance Criteria:**
- [ ] The hot window (focused window at keybinding press time) is moved to the target monitor
- [ ] The hot window is resized to fill the entire target monitor
- [ ] All toplevel windows previously on the target monitor are moved to the stack screen
- [ ] The stack screen re-tiles to accommodate displaced windows
- [ ] The menu closes after this action
**Priority**: Must-have

#### FR-014: Menu — Select Monitor (N)
**Description:** While the menu is in the overview state, pressing a monitor number (without Shift) must zoom into that monitor, showing the available layout presets.
**Acceptance Criteria:**
- [ ] The menu transitions to a zoomed-in view of the selected monitor
- [ ] The zoomed view displays 4 layout options, numbered 1-4:
  - 1: Fullscreen (single window fills the monitor)
  - 2: Side-by-side (two windows, left/right split, equal width)
  - 3: Top-bottom (two windows, top/bottom split, equal height)
  - 4: Quadrants (four windows in a 2x2 grid)
- [ ] The zoomed view also displays enforcement toggles: 9 = enforce on, 0 = enforce off
**Priority**: Must-have

#### FR-015: Menu — Apply Layout (1-4)
**Description:** While the menu is in the zoomed-in state for a monitor, pressing a number 1-4 must apply the corresponding layout preset to the windows currently on that monitor.
**Acceptance Criteria:**
- [ ] Layout is applied only to toplevel windows already on the selected monitor
- [ ] If there are fewer windows than the layout requires, empty positions remain empty (no windows are pulled from elsewhere)
- [ ] If there are more windows than the layout accommodates, excess windows remain in their current positions
- [ ] The menu closes immediately after a layout is applied
**Priority**: Must-have

#### FR-016: Menu — Toggle Enforcement (9/0)
**Description:** While the menu is in the zoomed-in state, pressing 9 must enable enforcement mode and pressing 0 must disable it, for that monitor on the current virtual desktop.
**Acceptance Criteria:**
- [ ] Pressing 9 enables enforcement: any user attempt to manually move or resize a tiled window on that monitor causes the window to snap back to its assigned position
- [ ] Pressing 0 disables enforcement: the user can freely move and resize windows on that monitor
- [ ] Enforcement is per-monitor, per-virtual-desktop
- [ ] Enforcement defaults to enabled on all monitors
- [ ] The menu closes after toggling enforcement
**Priority**: Must-have

#### FR-017: Menu — Dismiss (Esc)
**Description:** Pressing Esc at any point while the menu is open must close the menu immediately without performing any action.
**Acceptance Criteria:**
- [ ] Pressing Esc in the overview state closes the menu
- [ ] Pressing Esc in the zoomed-in state closes the menu entirely (does not go back to overview)
- [ ] No tiling changes are made when Esc is pressed
**Priority**: Must-have

### 2.7 Layout Enforcement

#### FR-018: Window Position Enforcement
**Description:** When enforcement mode is active for a monitor, the system must prevent users from manually repositioning or resizing tiled windows by snapping them back to their assigned positions.
**Acceptance Criteria:**
- [ ] When a tiled window is moved or resized by the user, it is snapped back to its assigned tile position
- [ ] Snap-back occurs within one event loop cycle of detecting the move/resize
- [ ] Enforcement mode is enabled by default for all monitors
- [ ] Enforcement mode can be toggled per-monitor via the menu (FR-016)
**Priority**: Must-have

### 2.8 Configuration

#### FR-019: TOML Configuration File
**Description:** The system must read configuration from a TOML file at `~/.config/tiler/config.toml`.
**Acceptance Criteria:**
- [ ] The daemon reads configuration on startup from `~/.config/tiler/config.toml`
- [ ] If the configuration file does not exist, the daemon starts with default values
- [ ] The configuration file supports at minimum: stack screen position (always leftmost for now, future-proofing the config key)
**Priority**: Must-have

### 2.9 NixOS Integration

#### FR-020: Nix Flake Package
**Description:** The project must provide a Nix flake that builds the Rust binary with musl static linking.
**Acceptance Criteria:**
- [ ] `nix build` produces a statically linked binary using musl
- [ ] The binary has no dynamic library dependencies
- [ ] The flake includes a devShell for development
**Priority**: Must-have

#### FR-021: NixOS Module
**Description:** The flake must include a NixOS module that installs the package, sets up a systemd user service for the daemon, and provides options for keybinding configuration.
**Acceptance Criteria:**
- [ ] The NixOS module installs the tiler binary
- [ ] The module creates a systemd user service that starts the daemon on login
- [ ] The module provides a configurable option for the global keybinding that triggers the menu
- [ ] The module can generate the TOML configuration file from NixOS options
**Priority**: Must-have

## 3. Non-Functional Requirements

### 3.1 Performance

#### NFR-001: Menu Responsiveness
**Description:** The floating menu must appear and respond to input within 200ms.
**Acceptance Criteria:**
- [ ] The menu must appear within 200ms of the CLI command being invoked
- [ ] Layout changes must be applied within 200ms of the user pressing a key
- [ ] Window moves (Shift+N) must complete within 200ms
**Priority**: Must-have

#### NFR-002: Event Processing Latency
**Description:** Window event processing (new window detection, close detection) must not introduce perceptible delay.
**Acceptance Criteria:**
- [ ] New windows must be moved to the stack screen within 200ms of creation
- [ ] Re-tiling after window close must complete within 200ms
**Priority**: Must-have

### 3.2 Reliability

#### NFR-003: Graceful Degradation
**Description:** The system must handle error conditions without crashing.
**Acceptance Criteria:**
- [ ] If the Wayland compositor connection is lost, the daemon must log the error and exit cleanly
- [ ] If a window cannot be moved (e.g., it was closed between detection and move), the daemon must skip it and continue
- [ ] If the Unix socket cannot be created, the daemon must report the error and exit with a non-zero code
**Priority**: Must-have

### 3.3 Build & Distribution

#### NFR-004: Static Binary
**Description:** The release binary must be statically linked using musl with no runtime dependencies.
**Acceptance Criteria:**
- [ ] `ldd` reports "not a dynamic executable" or "statically linked"
- [ ] Binary runs on any x86_64 Linux system without additional libraries
**Priority**: Must-have

## 4. Data Model

### 4.1 Entities
- **Window**: ID, title, app class, monitor assignment, tile position (x, y, width, height), virtual desktop ID, is_fullscreen flag
- **Monitor**: ID, name, position (left-to-right index), resolution, is_stack flag
- **VirtualDesktop**: ID, per-monitor layout preset, per-monitor enforcement mode, stack state (ordered list of window IDs)
- **LayoutPreset**: Enum (Fullscreen, SideBySide, TopBottom, Quadrants)

### 4.2 Data Lifecycle
All data is held in memory only. No persistence to disk. On daemon restart, all state is rebuilt by enumerating current windows.

## 5. Integration Points

### 5.1 GNOME Wayland Compositor
- **Direction:** Bidirectional
- **Protocol:** GNOME Wayland protocols / D-Bus interfaces to Mutter/GNOME Shell
- **Contract:** Must support: enumerate windows, move/resize windows, detect window open/close/focus events, detect workspace changes, determine window type (toplevel vs. dialog), detect fullscreen state
- **Authentication:** None (local compositor connection)
- **Failure handling:** If the compositor connection is lost, log error and exit cleanly

### 5.2 Unix Domain Socket (Internal IPC)
- **Direction:** Bidirectional (CLI → Daemon commands, Daemon → CLI responses)
- **Protocol:** Unix domain socket with a fixed-format command/response protocol
- **Contract:** Commands: `menu` (open the floating menu). Responses: success/error status.
- **Authentication:** File-system permissions (socket owned by the user)
- **Failure handling:** CLI exits with error if daemon is unreachable

## 6. Constraints
- Must be written in Rust
- Must target GNOME on Wayland only (no X11 support)
- Must compile with musl for static linking on NixOS
- Must be packaged as a Nix flake
- Single-binary architecture (daemon mode and CLI mode in the same binary)
- No runtime dependencies beyond the Wayland compositor

## 7. Assumptions
- **GNOME on Wayland exposes sufficient APIs for window management.** GNOME's Wayland implementation provides the ability to enumerate, move, resize, and detect lifecycle events for toplevel windows. If false, a GNOME Shell extension companion may be required.
- **The user has a static monitor configuration during a session.** Monitors are not frequently connected/disconnected. Monitor hot-plug is a nice-to-have, not a requirement. If false, hot-plug detection must be added.
- **The leftmost monitor is always the stack screen.** The user does not need to change which monitor is the stack screen at runtime. If false, the config and menu must support stack screen selection.
- **2-3 monitors is the target configuration.** The menu numbering and layout design targets this range. If false, the menu design must scale to more monitors.
- **NixOS is the only supported distribution.** The binary is statically linked and portable, but the NixOS module and flake are the only supported installation method. If false, additional packaging (e.g., Arch AUR, Debian .deb) must be added.

## 8. Open Questions
- **GNOME Wayland window management API:** Which specific protocol or API provides the needed window management capabilities on GNOME Wayland? Options include GNOME Shell D-Bus eval interface, a companion GNOME Shell extension, or the Mutter D-Bus interface. This must be determined during architecture/design and may constrain the implementation approach. **Blocks:** FR-001, FR-005 architecture decisions. **Who can answer:** Developer investigation / GNOME documentation.
- **Monitor hot-plug support (v2):** Handling monitor connect/disconnect at runtime is a nice-to-have. The specific behavior when a monitor is lost (where do its windows go?) needs definition if this is pursued. **Blocks:** Nothing in v1. **Who can answer:** Product decision.
