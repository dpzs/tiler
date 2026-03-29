//! Tiling engine: coordinates window tracking, layout enforcement, and menu state.

use std::collections::HashMap;
use std::time::{Duration, Instant};

use tracing::{debug, info, warn};

use crate::gnome::dbus_proxy::{GnomeProxy, MonitorInfo, ProxyResult};
use crate::menu::state::{MenuAction, MenuInput, MenuState};
use crate::model::{LayoutPreset, Rect, VirtualDesktop};
use crate::tiling::preset::{apply_fullscreen, apply_quadrants, apply_side_by_side, apply_top_bottom};
use crate::tiling::stack::stack_layout;

/// Tracks a window known to the engine.
struct TrackedWindow {
    #[allow(dead_code)]
    id: u64,
    workspace_id: u32,
    monitor_id: u32,
    is_fullscreen: bool,
    is_toplevel: bool,
}

/// Core tiling engine.
///
/// Owns all window state, per-workspace virtual desktops, and the menu state
/// machine. Drives layout enforcement and responds to compositor events
/// forwarded by the daemon.
pub struct TilingEngine<P: GnomeProxy> {
    proxy: P,
    stack_screen_index: usize,
    monitors: Vec<MonitorInfo>,
    windows: HashMap<u64, TrackedWindow>,
    desktops: HashMap<u32, VirtualDesktop>,
    active_workspace: u32,
    focused_window_id: Option<u64>,
    menu: MenuState,
    is_tiling: bool,
    /// Timestamp of the last tiling batch completion. Geometry-change signals
    /// arriving within a grace period after this are suppressed to avoid
    /// snap-back from async Mutter events.
    last_tiling_end: Option<Instant>,
}

impl<P: GnomeProxy> TilingEngine<P> {
    /// Create a new engine. Call [`startup`](Self::startup) before dispatching events.
    ///
    /// `stack_screen_index` is the index into the monitor list used as the
    /// stack tiling screen.
    pub fn new(proxy: P, stack_screen_index: usize) -> Self {
        Self {
            proxy,
            stack_screen_index,
            monitors: Vec::new(),
            windows: HashMap::new(),
            desktops: HashMap::new(),
            active_workspace: 0,
            focused_window_id: None,
            menu: MenuState::Closed,
            is_tiling: false,
            last_tiling_end: None,
        }
    }

    /// Returns a reference to the underlying compositor proxy.
    pub fn proxy(&self) -> &P {
        &self.proxy
    }

    /// Returns a mutable reference to the underlying compositor proxy.
    pub fn proxy_mut(&mut self) -> &mut P {
        &mut self.proxy
    }

    /// Returns `true` while the engine is actively repositioning windows.
    pub fn is_tiling(&self) -> bool {
        self.is_tiling
    }

    /// Set the tiling guard. While `true`, geometry-change events are suppressed.
    pub fn set_tiling(&mut self, value: bool) {
        self.is_tiling = value;
    }

    /// Clear the post-tiling grace period so geometry enforcement resumes
    /// immediately. Useful in tests where there is no real async delay.
    pub fn clear_tiling_grace(&mut self) {
        self.last_tiling_end = None;
    }

    fn desktop(&mut self, ws: u32) -> &mut VirtualDesktop {
        self.desktops
            .entry(ws)
            .or_insert_with(|| VirtualDesktop::new(ws))
    }

    /// Returns a mutable reference to the virtual desktop for `ws`, creating it if absent.
    pub fn desktop_mut(&mut self, ws: u32) -> &mut VirtualDesktop {
        self.desktops
            .entry(ws)
            .or_insert_with(|| VirtualDesktop::new(ws))
    }

    fn stack_screen_rect(&self) -> Option<Rect> {
        self.monitors.get(self.stack_screen_index).map(|m| Rect {
            x: m.x,
            y: m.y,
            width: m.width,
            height: m.height,
        })
    }

    /// Tile all stack windows for a given workspace.
    async fn tile_stack(&mut self, workspace_id: u32) -> ProxyResult<()> {
        let screen = match self.stack_screen_rect() {
            Some(r) => r,
            None => {
                warn!(workspace_id, "tile_stack: no stack screen rect, skipping");
                return Ok(());
            }
        };

        self.is_tiling = true;

        let stack_monitor = self.stack_screen_index as u32;
        let window_ids: Vec<u64> = self
            .desktops
            .get(&workspace_id)
            .map(|d| {
                d.stack_windows
                    .iter()
                    .filter(|&&wid| {
                        self.windows
                            .get(&wid)
                            .is_some_and(|w| w.monitor_id == stack_monitor)
                    })
                    .copied()
                    .collect()
            })
            .unwrap_or_default();

        debug!(
            workspace_id,
            stack_monitor,
            window_count = window_ids.len(),
            ?window_ids,
            "tile_stack"
        );

        let positions = stack_layout(&window_ids, screen);

        for (id, rect) in &positions {
            debug!(window_id = id, x = rect.x, y = rect.y, w = rect.width, h = rect.height, "  stack -> move_resize");
            self.proxy
                .move_resize_window(*id, rect.x, rect.y, rect.width, rect.height)
                .await?;
        }

        // Raise stack windows so the first (newest) is on top
        for (id, _) in positions.iter().rev() {
            self.proxy.raise_window(*id).await?;
        }

        self.is_tiling = false;
        self.last_tiling_end = Some(Instant::now());
        Ok(())
    }

    /// Check if a window type string represents a toplevel window.
    fn is_toplevel_type(wtype: &str) -> bool {
        wtype == "toplevel"
    }

    /// Initialize the engine: load monitors, enumerate existing windows, tile.
    pub async fn startup(&mut self) -> ProxyResult<()> {
        self.monitors = self.proxy.get_monitors().await?;
        self.active_workspace = self.proxy.get_active_workspace().await?;
        info!(
            monitors = self.monitors.len(),
            active_workspace = self.active_workspace,
            stack_screen = self.stack_screen_index,
            "engine startup: loaded monitors"
        );
        for (i, m) in self.monitors.iter().enumerate() {
            info!(index = i, id = m.id, x = m.x, y = m.y, w = m.width, h = m.height, "  monitor");
        }

        let windows = self.proxy.list_windows().await?;
        info!(count = windows.len(), "engine startup: enumerating existing windows");
        for w in windows {
            let wtype = self.proxy.get_window_type(w.id).await?;
            let is_fs = self.proxy.is_fullscreen(w.id).await?;
            let is_tl = Self::is_toplevel_type(&wtype);

            let tracked = TrackedWindow {
                id: w.id,
                workspace_id: w.workspace_id,
                monitor_id: w.monitor_id,
                is_fullscreen: is_fs,
                is_toplevel: is_tl,
            };

            if is_tl && !is_fs {
                self.desktop(w.workspace_id).append_window(w.id);
            }

            self.windows.insert(w.id, tracked);
        }

        self.tile_stack(self.active_workspace).await?;
        Ok(())
    }

    /// Handle a new window opening.
    pub async fn handle_window_opened(
        &mut self,
        window_id: u64,
        _title: String,
        _app_class: String,
        monitor_id: u32,
    ) -> ProxyResult<()> {
        let wtype = self.proxy.get_window_type(window_id).await?;
        let is_fs = self.proxy.is_fullscreen(window_id).await?;
        let is_tl = Self::is_toplevel_type(&wtype);

        info!(
            window_id, monitor_id, %wtype, is_tl, is_fs,
            ws = self.active_workspace,
            "window opened"
        );

        let tracked = TrackedWindow {
            id: window_id,
            workspace_id: self.active_workspace,
            monitor_id,
            is_fullscreen: is_fs,
            is_toplevel: is_tl,
        };

        self.windows.insert(window_id, tracked);

        if is_tl && !is_fs {
            let ws = self.active_workspace;
            self.desktop(ws).push_window(window_id);
            if monitor_id == self.stack_screen_index as u32 {
                info!(window_id, "window on stack screen, retiling stack");
                self.tile_stack(ws).await?;
            }
            if let Some(preset) = self.desktops.get(&ws).and_then(|d| d.get_layout(monitor_id)) {
                info!(window_id, monitor_id, ?preset, "monitor has layout preset, re-applying");
                self.apply_layout_to_monitor(ws, monitor_id).await?;
            }
        } else {
            debug!(window_id, is_tl, is_fs, "window not tileable, skipping");
        }

        Ok(())
    }

    /// Handle a window closing.
    pub async fn handle_window_closed(&mut self, window_id: u64) -> ProxyResult<()> {
        let tracked = match self.windows.remove(&window_id) {
            Some(t) => t,
            None => {
                debug!(window_id, "window closed but not tracked, ignoring");
                return Ok(());
            }
        };

        let ws = tracked.workspace_id;
        let monitor_id = tracked.monitor_id;
        info!(window_id, ws, monitor_id, "window closed, removing from desktop");
        self.desktop(ws).remove_window(window_id);

        if tracked.is_toplevel && !tracked.is_fullscreen {
            self.tile_stack(ws).await?;
            if let Some(preset) = self.desktops.get(&ws).and_then(|d| d.get_layout(monitor_id)) {
                info!(monitor_id, ?preset, "monitor has layout preset, re-applying after close");
                self.apply_layout_to_monitor(ws, monitor_id).await?;
            }
        }

        Ok(())
    }

    /// Handle workspace change.
    pub async fn handle_workspace_changed(&mut self, workspace_id: u32) -> ProxyResult<()> {
        info!(from = self.active_workspace, to = workspace_id, "workspace changed");
        self.active_workspace = workspace_id;
        self.tile_stack(workspace_id).await?;
        let monitor_ids: Vec<u32> = self
            .desktops
            .get(&workspace_id)
            .map(|d| d.layout_presets.keys().copied().collect())
            .unwrap_or_default();
        if !monitor_ids.is_empty() {
            info!(?monitor_ids, "re-applying layout presets for workspace");
        }
        for mid in monitor_ids {
            self.apply_layout_to_monitor(workspace_id, mid).await?;
        }
        Ok(())
    }

    /// Handle fullscreen state change.
    pub async fn handle_fullscreen_changed(
        &mut self,
        window_id: u64,
        is_fullscreen: bool,
    ) -> ProxyResult<()> {
        let ws = match self.windows.get_mut(&window_id) {
            Some(w) => {
                w.is_fullscreen = is_fullscreen;
                w.workspace_id
            }
            None => return Ok(()),
        };

        if is_fullscreen {
            // Remove from stack
            self.desktop(ws).remove_window(window_id);
        } else {
            // Check if it should be in the stack
            let is_tl = self.windows.get(&window_id).map_or(false, |w| w.is_toplevel);
            if is_tl {
                self.desktop(ws).push_window(window_id);
            }
        }

        self.tile_stack(ws).await?;
        Ok(())
    }

    /// Handle a window geometry change event.
    ///
    /// If layout enforcement is active on the window's monitor and the window's
    /// current geometry differs from its expected layout position, snap it back.
    pub async fn handle_geometry_changed(
        &mut self,
        window_id: u64,
        x: i32,
        y: i32,
        width: i32,
        height: i32,
    ) -> ProxyResult<()> {
        if self.is_tiling {
            return Ok(());
        }

        // Grace period: suppress geometry-change signals that arrive shortly
        // after a tiling batch completes — Mutter fires these asynchronously.
        const TILING_GRACE: Duration = Duration::from_millis(500);
        if let Some(end) = self.last_tiling_end {
            if end.elapsed() < TILING_GRACE {
                return Ok(());
            }
        }

        let (workspace_id, monitor_id) = match self.windows.get(&window_id) {
            Some(w) => (w.workspace_id, w.monitor_id),
            None => return Ok(()),
        };

        let desktop = match self.desktops.get(&workspace_id) {
            Some(d) => d,
            None => return Ok(()),
        };

        if !desktop.is_enforced(monitor_id) {
            return Ok(());
        }

        let preset = match desktop.get_layout(monitor_id) {
            Some(p) => p,
            None => return Ok(()),
        };

        // Find monitor rect
        let monitor_rect = match self.monitors.iter().find(|m| m.id == monitor_id) {
            Some(m) => Rect {
                x: m.x,
                y: m.y,
                width: m.width,
                height: m.height,
            },
            None => return Ok(()),
        };

        // Collect window IDs on this monitor, preserving desktop stack order
        let window_ids: Vec<u64> = desktop
            .stack_windows
            .iter()
            .filter(|&&wid| {
                self.windows
                    .get(&wid)
                    .is_some_and(|w| w.monitor_id == monitor_id)
            })
            .copied()
            .collect();

        // Compute expected positions using the layout preset
        let positions = match preset {
            LayoutPreset::Fullscreen => apply_fullscreen(&window_ids, monitor_rect),
            LayoutPreset::SideBySide => apply_side_by_side(&window_ids, monitor_rect),
            LayoutPreset::TopBottom => apply_top_bottom(&window_ids, monitor_rect),
            LayoutPreset::Quadrants => apply_quadrants(&window_ids, monitor_rect),
        };

        // Find this window's expected position
        let expected = match positions.iter().find(|(id, _)| *id == window_id) {
            Some((_, rect)) => *rect,
            None => return Ok(()),
        };

        // If geometry already matches, no snap-back needed
        if x == expected.x && y == expected.y && width == expected.width && height == expected.height
        {
            return Ok(());
        }

        debug!(
            window_id, monitor_id,
            actual_x = x, actual_y = y, actual_w = width, actual_h = height,
            snap_x = expected.x, snap_y = expected.y, snap_w = expected.width, snap_h = expected.height,
            "enforcement snap-back"
        );
        self.proxy
            .move_resize_window(window_id, expected.x, expected.y, expected.width, expected.height)
            .await?;

        Ok(())
    }

    /// Returns the current menu state.
    pub fn menu_state(&self) -> MenuState {
        self.menu
    }

    /// Override the menu state directly (used by CLI commands to bypass the menu UI).
    pub fn set_menu_state(&mut self, state: MenuState) {
        self.menu = state;
    }

    /// Process a menu input, transitioning state and executing any resulting action.
    pub async fn handle_menu_input(&mut self, input: MenuInput) -> ProxyResult<()> {
        let old_state = self.menu;
        let (new_state, action) = self.menu.transition(input);
        debug!(?input, ?old_state, ?new_state, ?action, "menu transition");
        self.menu = new_state;

        // Closed -> Overview: show the menu overlay
        if old_state == MenuState::Closed && new_state == MenuState::Overview {
            let monitors_json = serde_json::to_string(&self.monitors)
                .unwrap_or_else(|_| "[]".to_string());
            self.proxy.show_menu(&monitors_json).await?;
        }

        // ZoomIn: show zoomed view for a specific monitor
        if let Some(MenuAction::ZoomIn(monitor_id)) = action {
            let layouts = [
                LayoutPreset::Fullscreen,
                LayoutPreset::SideBySide,
                LayoutPreset::TopBottom,
                LayoutPreset::Quadrants,
            ];
            let layouts_json = serde_json::to_string(&layouts)
                .unwrap_or_else(|_| "[]".to_string());
            self.proxy.show_menu_zoomed(monitor_id, &layouts_json).await?;
        }

        // Any transition TO Closed from a non-Closed state: hide the menu
        if new_state == MenuState::Closed && old_state != MenuState::Closed {
            self.proxy.hide_menu().await?;
        }

        if let Some(action) = action {
            match action {
                MenuAction::Dismiss | MenuAction::ZoomIn(_) => {}
                MenuAction::MoveWindow(target_monitor) => {
                    self.move_window_to_monitor(target_monitor).await?;
                }
                MenuAction::ApplyLayout(monitor_id, digit) => {
                    let preset = match digit {
                        1 => LayoutPreset::Fullscreen,
                        2 => LayoutPreset::SideBySide,
                        3 => LayoutPreset::TopBottom,
                        4 => LayoutPreset::Quadrants,
                        _ => return Ok(()),
                    };
                    let ws = self.active_workspace;
                    self.desktop(ws).set_layout(monitor_id, preset);
                    self.apply_layout_to_monitor(ws, monitor_id).await?;
                }
                MenuAction::EnforceOn(monitor_id) => {
                    let ws = self.active_workspace;
                    self.desktop(ws).set_enforcement(monitor_id, true);
                }
                MenuAction::EnforceOff(monitor_id) => {
                    let ws = self.active_workspace;
                    self.desktop(ws).set_enforcement(monitor_id, false);
                }
            }
        }

        Ok(())
    }

    /// Apply the current layout preset for a monitor, positioning all windows.
    async fn apply_layout_to_monitor(
        &mut self,
        workspace_id: u32,
        monitor_id: u32,
    ) -> ProxyResult<()> {
        let desktop = match self.desktops.get(&workspace_id) {
            Some(d) => d,
            None => return Ok(()),
        };

        let preset = match desktop.get_layout(monitor_id) {
            Some(p) => p,
            None => return Ok(()),
        };

        let monitor_rect = match self.monitors.iter().find(|m| m.id == monitor_id) {
            Some(m) => Rect {
                x: m.x,
                y: m.y,
                width: m.width,
                height: m.height,
            },
            None => {
                warn!(monitor_id, "apply_layout: monitor not found");
                return Ok(());
            }
        };

        self.is_tiling = true;

        let window_ids: Vec<u64> = desktop
            .stack_windows
            .iter()
            .filter(|&&wid| {
                self.windows
                    .get(&wid)
                    .is_some_and(|w| w.monitor_id == monitor_id)
            })
            .copied()
            .collect();

        info!(
            workspace_id, monitor_id, ?preset,
            window_count = window_ids.len(), ?window_ids,
            "apply_layout_to_monitor"
        );

        let positions = match preset {
            LayoutPreset::Fullscreen => apply_fullscreen(&window_ids, monitor_rect),
            LayoutPreset::SideBySide => apply_side_by_side(&window_ids, monitor_rect),
            LayoutPreset::TopBottom => apply_top_bottom(&window_ids, monitor_rect),
            LayoutPreset::Quadrants => apply_quadrants(&window_ids, monitor_rect),
        };

        // Move positioned windows first, then raise them in order so the
        // first-in-layout window ends up on top.
        for (id, rect) in &positions {
            debug!(window_id = id, x = rect.x, y = rect.y, w = rect.width, h = rect.height, "  layout -> move_resize");
            self.proxy
                .move_resize_window(*id, rect.x, rect.y, rect.width, rect.height)
                .await?;
        }

        // Excess windows (not assigned a slot) get stacked behind the layout
        // by moving them to fill the monitor — this prevents them from
        // floating at stale positions and visually overlapping the layout.
        let positioned_ids: Vec<u64> = positions.iter().map(|(id, _)| *id).collect();
        for &wid in &window_ids {
            if !positioned_ids.contains(&wid) {
                debug!(window_id = wid, "  layout -> stash excess behind layout");
                self.proxy
                    .move_resize_window(wid, monitor_rect.x, monitor_rect.y, monitor_rect.width, monitor_rect.height)
                    .await?;
            }
        }

        // Raise positioned windows in reverse order so the first window in
        // the layout ends up on top of the stacking order.
        for (id, _) in positions.iter().rev() {
            self.proxy.raise_window(*id).await?;
        }

        self.is_tiling = false;
        self.last_tiling_end = Some(Instant::now());
        Ok(())
    }

    /// Returns the currently active workspace ID.
    pub fn active_workspace(&self) -> u32 {
        self.active_workspace
    }

    /// Returns a read-only reference to the virtual desktop for `ws`, or `None` if it
    /// has not been created yet.
    /// Move the currently focused window to the target monitor.
    ///
    /// No-op if no window is focused or the target monitor does not exist.
    /// Calls `proxy.move_resize_window` to fill the target monitor, updates
    /// the tracked window's `monitor_id`, moves it between desktop stacks,
    /// and retiles the stack screen.
    pub async fn move_window_to_monitor(&mut self, target_monitor: u32) -> ProxyResult<()> {
        // Get focused window, no-op if None
        let window_id = match self.focused_window_id {
            Some(id) => id,
            None => return Ok(()),
        };

        // Look up target monitor geometry, no-op if not found
        let target_rect = match self.monitors.iter().find(|m| m.id == target_monitor) {
            Some(m) => (m.x, m.y, m.width, m.height),
            None => return Ok(()),
        };

        // Get the window's current workspace and source monitor
        let (workspace_id, source_monitor) = match self.windows.get(&window_id) {
            Some(w) => (w.workspace_id, w.monitor_id),
            None => return Ok(()),
        };

        // Move the window to fill the target monitor and raise it
        self.proxy
            .move_resize_window(window_id, target_rect.0, target_rect.1, target_rect.2, target_rect.3)
            .await?;
        self.proxy.raise_window(window_id).await?;

        // Update tracked window's monitor_id
        if let Some(w) = self.windows.get_mut(&window_id) {
            w.monitor_id = target_monitor;
        }

        // Retile stack screen if either source or target is the stack screen
        let stack_monitor = self.stack_screen_index as u32;
        if source_monitor == stack_monitor || target_monitor == stack_monitor {
            self.tile_stack(workspace_id).await?;
        }

        Ok(())
    }

    /// Returns the currently focused window ID, or `None` if no window has focus.
    pub fn focused_window_id(&self) -> Option<u64> {
        self.focused_window_id
    }

    /// Updates the focused window ID. Called when the compositor reports a focus change.
    pub fn handle_focus_changed(&mut self, window_id: u64) {
        self.focused_window_id = Some(window_id);
    }

    pub fn desktop_ref(&self, ws: u32) -> Option<&VirtualDesktop> {
        self.desktops.get(&ws)
    }
}
