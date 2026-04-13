//! Tiling engine: coordinates window tracking, layout enforcement, and menu state.

use std::collections::{HashMap, HashSet};
use std::time::{Duration, Instant};

use tracing::{debug, info, warn};

use crate::config::StackScreenPosition;
use crate::gnome::dbus_proxy::{GnomeProxy, MonitorInfo, ProxyResult};
use crate::menu::state::{MenuAction, MenuInput, MenuState};
use crate::model::{LayoutPreset, Rect, VirtualDesktop};
use crate::tiling::preset::{apply_fullscreen, apply_quadrants, apply_side_by_side, apply_top_bottom};
use crate::tiling::stack::stack_layout;

/// Tracks a window known to the engine.
struct TrackedWindow {
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
    stack_position: StackScreenPosition,
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
    /// Workspaces that have been tiled at least once. On first visit we tile
    /// the stack; subsequent switches do NOT reflow windows.
    visited_workspaces: HashSet<u32>,
}

impl<P: GnomeProxy> TilingEngine<P> {
    /// Create a new engine. Call [`startup`](Self::startup) before dispatching events.
    ///
    /// `stack_position` determines which monitor (by position) is used as the
    /// stack tiling screen. The actual index is resolved during `startup()`
    /// once monitors are loaded.
    pub fn new(proxy: P, stack_position: StackScreenPosition) -> Self {
        Self {
            proxy,
            stack_position,
            stack_screen_index: 0,
            monitors: Vec::new(),
            windows: HashMap::new(),
            desktops: HashMap::new(),
            active_workspace: 0,
            focused_window_id: None,
            menu: MenuState::Closed,
            is_tiling: false,
            last_tiling_end: None,
            visited_workspaces: HashSet::new(),
        }
    }

    /// Returns a reference to the underlying compositor proxy.
    #[must_use]
    pub fn proxy(&self) -> &P {
        &self.proxy
    }

    /// Returns a mutable reference to the underlying compositor proxy.
    #[must_use]
    pub fn proxy_mut(&mut self) -> &mut P {
        &mut self.proxy
    }

    /// Returns `true` while the engine is actively repositioning windows.
    #[must_use]
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

    /// Returns a mutable reference to the virtual desktop for `ws`, creating it if absent.
    pub fn desktop_mut(&mut self, ws: u32) -> &mut VirtualDesktop {
        self.desktops
            .entry(ws)
            .or_insert_with(|| VirtualDesktop::new(ws))
    }

    /// Returns a read-only reference to the virtual desktop for `ws`, or `None`
    /// if it has not been created yet.
    pub fn desktop_ref(&self, ws: u32) -> Option<&VirtualDesktop> {
        self.desktops.get(&ws)
    }

    /// The stack screen's monitor ID.
    #[allow(clippy::cast_possible_truncation)]
    fn stack_monitor_id(&self) -> u32 {
        self.stack_screen_index as u32
    }

    fn stack_screen_rect(&self) -> Option<Rect> {
        self.monitors.get(self.stack_screen_index).map(|m| Rect {
            x: m.x,
            y: m.y,
            width: m.width,
            height: m.height,
        })
    }

    /// Look up a monitor's geometry by ID.
    fn monitor_rect(&self, monitor_id: u32) -> Option<Rect> {
        self.monitors.iter().find(|m| m.id == monitor_id).map(|m| Rect {
            x: m.x,
            y: m.y,
            width: m.width,
            height: m.height,
        })
    }

    /// Collect window IDs from the desktop's stack that are currently tracked
    /// on `monitor_id`, preserving stack order.
    fn windows_on_monitor(&self, workspace_id: u32, monitor_id: u32) -> Vec<u64> {
        self.desktops
            .get(&workspace_id)
            .map(|d| {
                d.stack_windows
                    .iter()
                    .filter(|&&wid| {
                        self.windows
                            .get(&wid)
                            .is_some_and(|w| w.monitor_id == monitor_id)
                    })
                    .copied()
                    .collect()
            })
            .unwrap_or_default()
    }

    /// Compute layout positions for a preset.
    fn compute_positions(
        preset: LayoutPreset,
        window_ids: &[u64],
        rect: Rect,
    ) -> Vec<(u64, Rect)> {
        match preset {
            LayoutPreset::Fullscreen => apply_fullscreen(window_ids, rect),
            LayoutPreset::SideBySide => apply_side_by_side(window_ids, rect),
            LayoutPreset::TopBottom => apply_top_bottom(window_ids, rect),
            LayoutPreset::Quadrants => apply_quadrants(window_ids, rect),
        }
    }

    /// Mark tiling as complete and record the timestamp for the grace period.
    fn finish_tiling(&mut self) {
        self.is_tiling = false;
        self.last_tiling_end = Some(Instant::now());
    }

    /// Remove orphaned window IDs from the desktop's `stack_windows` that are
    /// no longer present in the engine's tracked windows map.
    fn prune_desktop(&mut self, workspace_id: u32) {
        let live_ids: std::collections::HashSet<u64> = self.windows.keys().copied().collect();
        if let Some(desktop) = self.desktops.get_mut(&workspace_id) {
            desktop.prune_orphaned_windows(&live_ids);
        }
    }

    /// Tile all stack windows for a given workspace.
    async fn tile_stack(&mut self, workspace_id: u32) -> ProxyResult<()> {
        let Some(screen) = self.stack_screen_rect() else {
            warn!(workspace_id, "tile_stack: no stack screen rect, skipping");
            return Ok(());
        };

        self.is_tiling = true;
        self.prune_desktop(workspace_id);

        let window_ids = self.windows_on_monitor(workspace_id, self.stack_monitor_id());

        debug!(
            workspace_id,
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

        // Excess windows beyond the grid capacity are stashed at full screen
        // size behind the tiled windows so they don't float at stale positions.
        let positioned_ids: Vec<u64> = positions.iter().map(|(id, _)| *id).collect();
        for &wid in &window_ids {
            if !positioned_ids.contains(&wid) {
                debug!(window_id = wid, "  stack -> stash excess behind layout");
                self.proxy
                    .move_resize_window(wid, screen.x, screen.y, screen.width, screen.height)
                    .await?;
            }
        }

        // Raise stack windows so the first (newest) is on top
        for (id, _) in positions.iter().rev() {
            self.proxy.raise_window(*id).await?;
        }

        self.finish_tiling();
        Ok(())
    }

    /// Check if a window type string represents a toplevel window.
    fn is_toplevel_type(wtype: &str) -> bool {
        wtype == "toplevel"
    }

    /// Move a window to the stack screen, update tracking, and retile.
    async fn move_to_stack(&mut self, window_id: u64, workspace_id: u32) -> ProxyResult<()> {
        let Some(rect) = self.stack_screen_rect() else {
            return Ok(());
        };
        self.proxy
            .move_resize_window(window_id, rect.x, rect.y, rect.width, rect.height)
            .await?;
        let stack_id = self.stack_monitor_id();
        if let Some(w) = self.windows.get_mut(&window_id) {
            w.monitor_id = stack_id;
        }
        self.tile_stack(workspace_id).await
    }

    /// Initialize the engine: load monitors, enumerate existing windows, tile.
    pub async fn startup(&mut self) -> ProxyResult<()> {
        self.monitors = self.proxy.get_monitors().await?;

        if self.monitors.is_empty() {
            return Err("no monitors detected — cannot initialize tiling engine".into());
        }

        // Resolve the stack screen index from the configured position.
        self.stack_screen_index = self
            .stack_position
            .resolve_index(&self.monitors)
            .ok_or("monitors list is non-empty but resolve_index returned None")?;

        self.active_workspace = self.proxy.get_active_workspace().await?;
        info!(
            monitors = self.monitors.len(),
            active_workspace = self.active_workspace,
            stack_screen = self.stack_screen_index,
            stack_position = ?self.stack_position,
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

            if is_tl && !is_fs {
                self.desktop_mut(w.workspace_id).append_window(w.id);
            }

            self.windows.insert(w.id, TrackedWindow {
                workspace_id: w.workspace_id,
                monitor_id: w.monitor_id,
                is_fullscreen: is_fs,
                is_toplevel: is_tl,
            });
        }

        self.visited_workspaces.insert(self.active_workspace);
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

        self.windows.insert(window_id, TrackedWindow {
            workspace_id: self.active_workspace,
            monitor_id,
            is_fullscreen: is_fs,
            is_toplevel: is_tl,
        });

        if !is_tl || is_fs {
            debug!(window_id, is_tl, is_fs, "window not tileable, skipping");
            return Ok(());
        }

        let ws = self.active_workspace;
        self.desktop_mut(ws).push_window(window_id);

        if let Some(preset) = self.desktops.get(&ws).and_then(|d| d.get_layout(monitor_id)) {
            // Monitor has a layout preset — re-apply it (works for both
            // stack and non-stack monitors, so presets always take priority).
            info!(window_id, monitor_id, ?preset, "monitor has layout preset, re-applying");
            self.apply_layout_to_monitor(ws, monitor_id).await?;
        } else if monitor_id == self.stack_monitor_id() {
            info!(window_id, "window on stack screen, retiling stack");
            self.tile_stack(ws).await?;
        } else {
            // Window opened on a non-stack monitor with no layout preset:
            // move it to the stack screen per the design requirement that
            // all new windows land on the stack screen.
            info!(window_id, from_monitor = monitor_id, "moving window to stack screen");
            self.move_to_stack(window_id, ws).await?;
        }

        Ok(())
    }

    /// Handle a window closing.
    pub async fn handle_window_closed(&mut self, window_id: u64) -> ProxyResult<()> {
        let Some(tracked) = self.windows.remove(&window_id) else {
            debug!(window_id, "window closed but not tracked, ignoring");
            return Ok(());
        };

        // Clear focused window if the closed window was focused, to prevent
        // move_window_to_monitor from referencing a stale ID.
        if self.focused_window_id == Some(window_id) {
            self.focused_window_id = None;
        }

        let ws = tracked.workspace_id;
        let monitor_id = tracked.monitor_id;
        info!(window_id, ws, monitor_id, "window closed, removing from desktop");
        self.desktop_mut(ws).remove_window(window_id);

        if !tracked.is_toplevel || tracked.is_fullscreen {
            return Ok(());
        }

        // Only retile the surfaces that actually changed.
        // If the monitor has a layout preset, apply that; otherwise fall back
        // to stack tiling when the window was on the stack screen. This avoids
        // conflicting layouts when a preset is set on the stack monitor.
        let stack_id = self.stack_monitor_id();
        if let Some(preset) = self.desktops.get(&ws).and_then(|d| d.get_layout(monitor_id)) {
            info!(monitor_id, ?preset, "monitor has layout preset, re-applying after close");
            self.apply_layout_to_monitor(ws, monitor_id).await?;
        } else if monitor_id == stack_id {
            self.tile_stack(ws).await?;
        }

        Ok(())
    }

    /// Handle workspace change.
    ///
    /// Windows should already be in position from when they were originally
    /// tiled. GNOME Shell manages workspace visibility. We only tile on the
    /// first visit to a workspace (to handle windows that existed before the
    /// daemon started).
    pub async fn handle_workspace_changed(&mut self, workspace_id: u32) -> ProxyResult<()> {
        info!(from = self.active_workspace, to = workspace_id, "workspace changed");
        self.active_workspace = workspace_id;

        if !self.visited_workspaces.contains(&workspace_id) {
            self.visited_workspaces.insert(workspace_id);
            info!(workspace_id, "first visit to workspace, tiling stack");
            self.tile_stack(workspace_id).await?;
        }

        Ok(())
    }

    /// Handle fullscreen state change.
    pub async fn handle_fullscreen_changed(
        &mut self,
        window_id: u64,
        is_fullscreen: bool,
    ) -> ProxyResult<()> {
        let (ws, monitor_id) = match self.windows.get_mut(&window_id) {
            Some(w) => {
                w.is_fullscreen = is_fullscreen;
                (w.workspace_id, w.monitor_id)
            }
            None => return Ok(()),
        };

        if is_fullscreen {
            self.desktop_mut(ws).remove_window(window_id);
        } else {
            let is_tl = self.windows.get(&window_id).is_some_and(|w| w.is_toplevel);
            if is_tl {
                self.desktop_mut(ws).push_window(window_id);
            }
        }

        let stack_id = self.stack_monitor_id();
        if monitor_id == stack_id {
            self.tile_stack(ws).await?;
        }
        if self.desktops.get(&ws).and_then(|d| d.get_layout(monitor_id)).is_some() {
            self.apply_layout_to_monitor(ws, monitor_id).await?;
        }
        Ok(())
    }

    /// Handle a window geometry change event.
    ///
    /// If layout enforcement is active on the window's monitor and the window's
    /// current geometry differs from its expected layout position, snap it back.
    #[allow(clippy::items_after_statements)]
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

        let Some(w) = self.windows.get(&window_id) else {
            return Ok(());
        };
        let (workspace_id, monitor_id) = (w.workspace_id, w.monitor_id);

        let Some(desktop) = self.desktops.get(&workspace_id) else {
            return Ok(());
        };

        if !desktop.is_enforced(monitor_id) {
            return Ok(());
        }

        let Some(preset) = desktop.get_layout(monitor_id) else {
            return Ok(());
        };

        let Some(monitor_rect) = self.monitor_rect(monitor_id) else {
            return Ok(());
        };

        let window_ids = self.windows_on_monitor(workspace_id, monitor_id);
        let positions = Self::compute_positions(preset, &window_ids, monitor_rect);

        // Find this window's expected position
        let Some((_, expected)) = positions.iter().find(|(id, _)| *id == window_id) else {
            return Ok(());
        };
        let expected = *expected;

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
    #[must_use]
    pub fn menu_state(&self) -> MenuState {
        self.menu
    }

    /// Override the menu state directly (used by CLI commands to bypass the menu UI).
    pub fn set_menu_state(&mut self, state: MenuState) {
        self.menu = state;
    }

    /// Process a menu input, transitioning state and executing any resulting action.
    #[allow(clippy::too_many_lines)]
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

        // ZoomIn: show zoomed view for a specific monitor.
        // Stack screen is always a vertical stack — skip the layout picker
        // and move the focused window to the stack directly.
        if let Some(MenuAction::ZoomIn(monitor_id)) = action {
            if monitor_id == self.stack_monitor_id() {
                self.menu = MenuState::Closed;
                self.proxy.hide_menu().await?;
                if let Some(wid) = self.focused_window_id {
                    let ws = self.active_workspace;
                    self.move_to_stack(wid, ws).await?;
                }
                return Ok(());
            }
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

                    // Move focused window to the target monitor first (if
                    // it is not already there) so the layout places it in
                    // the primary slot.
                    if let Some(wid) = self.focused_window_id {
                        if let Some(w) = self.windows.get(&wid) {
                            let source = w.monitor_id;
                            if source != monitor_id {
                                if let Some(rect) = self.monitor_rect(monitor_id) {
                                    self.is_tiling = true;
                                    self.proxy
                                        .move_resize_window(wid, rect.x, rect.y, rect.width, rect.height)
                                        .await?;
                                    self.proxy.raise_window(wid).await?;
                                    self.finish_tiling();
                                }
                                if let Some(w) = self.windows.get_mut(&wid) {
                                    w.monitor_id = monitor_id;
                                }
                                // Push to front so it gets the primary layout slot
                                self.desktop_mut(ws).push_window(wid);
                                // Retile source if it was the stack screen
                                if source == self.stack_monitor_id() {
                                    self.tile_stack(ws).await?;
                                }
                            }
                        }
                    }

                    self.desktop_mut(ws).set_layout(monitor_id, preset);
                    self.apply_layout_to_monitor(ws, monitor_id).await?;
                }
                MenuAction::EnforceOn(monitor_id) => {
                    let ws = self.active_workspace;
                    self.desktop_mut(ws).set_enforcement(monitor_id, true);
                }
                MenuAction::EnforceOff(monitor_id) => {
                    let ws = self.active_workspace;
                    self.desktop_mut(ws).set_enforcement(monitor_id, false);
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
        self.prune_desktop(workspace_id);

        let Some(desktop) = self.desktops.get(&workspace_id) else {
            return Ok(());
        };

        let Some(preset) = desktop.get_layout(monitor_id) else {
            return Ok(());
        };

        let Some(monitor_rect) = self.monitor_rect(monitor_id) else {
            warn!(monitor_id, "apply_layout: monitor not found");
            return Ok(());
        };

        let window_ids = self.windows_on_monitor(workspace_id, monitor_id);

        if window_ids.is_empty() {
            debug!(workspace_id, monitor_id, ?preset, "apply_layout: no windows, skipping");
            return Ok(());
        }

        self.is_tiling = true;

        info!(
            workspace_id, monitor_id, ?preset,
            window_count = window_ids.len(), ?window_ids,
            "apply_layout_to_monitor"
        );

        let positions = Self::compute_positions(preset, &window_ids, monitor_rect);

        // Move positioned windows first, then raise them in order so the
        // first-in-layout window ends up on top.
        for (id, rect) in &positions {
            debug!(window_id = id, x = rect.x, y = rect.y, w = rect.width, h = rect.height, "  layout -> move_resize");
            self.proxy
                .move_resize_window(*id, rect.x, rect.y, rect.width, rect.height)
                .await?;
        }

        // Excess windows not assigned a layout slot: return them to the
        // stack screen (per mission rule) instead of stashing behind.
        let positioned_ids: Vec<u64> = positions.iter().map(|(id, _)| *id).collect();
        let excess: Vec<u64> = window_ids
            .iter()
            .filter(|&&wid| !positioned_ids.contains(&wid))
            .copied()
            .collect();

        let stack_id = self.stack_monitor_id();
        let is_on_stack = monitor_id == stack_id;
        for &wid in &excess {
            if is_on_stack {
                // On the stack screen itself, stash behind (can't return to self)
                debug!(window_id = wid, "  layout -> stash excess behind layout");
                self.proxy
                    .move_resize_window(wid, monitor_rect.x, monitor_rect.y, monitor_rect.width, monitor_rect.height)
                    .await?;
            } else if let Some(stack_rect) = self.stack_screen_rect() {
                debug!(window_id = wid, "  layout -> return excess to stack screen");
                self.proxy
                    .move_resize_window(wid, stack_rect.x, stack_rect.y, stack_rect.width, stack_rect.height)
                    .await?;
                if let Some(w) = self.windows.get_mut(&wid) {
                    w.monitor_id = stack_id;
                }
            }
        }

        // Raise positioned windows in reverse order so the first window in
        // the layout ends up on top of the stacking order.
        for (id, _) in positions.iter().rev() {
            self.proxy.raise_window(*id).await?;
        }

        self.finish_tiling();

        // If excess windows were returned to the stack, retile it
        if !is_on_stack && !excess.is_empty() {
            self.tile_stack(workspace_id).await?;
        }

        Ok(())
    }

    /// Returns the currently active workspace ID.
    #[must_use]
    pub fn active_workspace(&self) -> u32 {
        self.active_workspace
    }

    /// Move the currently focused window to the target monitor.
    ///
    /// No-op if no window is focused or the target monitor does not exist.
    /// Calls `proxy.move_resize_window` to fill the target monitor, updates
    /// the tracked window's `monitor_id`, and retiles affected monitors.
    pub async fn move_window_to_monitor(&mut self, target_monitor: u32) -> ProxyResult<()> {
        let Some(window_id) = self.focused_window_id else {
            return Ok(());
        };

        let Some(target_rect) = self.monitor_rect(target_monitor) else {
            return Ok(());
        };

        let Some(w) = self.windows.get(&window_id) else {
            return Ok(());
        };
        let (workspace_id, source_monitor) = (w.workspace_id, w.monitor_id);

        // No-op when the window is already on the target monitor.
        if source_monitor == target_monitor {
            debug!(window_id, target_monitor, "window already on target monitor, skipping move");
            return Ok(());
        }

        // Move the window to fill the target monitor and raise it
        self.proxy
            .move_resize_window(window_id, target_rect.x, target_rect.y, target_rect.width, target_rect.height)
            .await?;
        self.proxy.raise_window(window_id).await?;

        // Update tracked window's monitor_id
        if let Some(w) = self.windows.get_mut(&window_id) {
            w.monitor_id = target_monitor;
        }

        // Retile stack screen if either source or target is the stack screen
        let stack_id = self.stack_monitor_id();
        if source_monitor == stack_id || target_monitor == stack_id {
            self.tile_stack(workspace_id).await?;
        }

        // Re-apply layout preset on the target monitor if one is set
        if self.desktops.get(&workspace_id).and_then(|d| d.get_layout(target_monitor)).is_some() {
            self.apply_layout_to_monitor(workspace_id, target_monitor).await?;
        }

        Ok(())
    }

    /// Returns the currently focused window ID, or `None` if no window has focus.
    #[must_use]
    pub fn focused_window_id(&self) -> Option<u64> {
        self.focused_window_id
    }

    /// Updates the focused window ID. Called when the compositor reports a focus change.
    ///
    /// Stores any window ID unconditionally. A focus signal may arrive before
    /// the corresponding `WindowOpened` event due to D-Bus signal ordering, so
    /// filtering untracked IDs here would cause missed focus updates. The
    /// `move_window_to_monitor` method already guards against untracked windows.
    pub fn handle_focus_changed(&mut self, window_id: u64) {
        self.focused_window_id = Some(window_id);
    }
}
