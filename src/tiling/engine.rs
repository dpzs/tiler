use std::collections::HashMap;

use crate::gnome::dbus_proxy::{GnomeProxy, MonitorInfo, ProxyResult};
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

pub struct TilingEngine<P: GnomeProxy> {
    proxy: P,
    stack_screen_index: usize,
    monitors: Vec<MonitorInfo>,
    windows: HashMap<u64, TrackedWindow>,
    desktops: HashMap<u32, VirtualDesktop>,
    active_workspace: u32,
}

impl<P: GnomeProxy> TilingEngine<P> {
    pub fn new(proxy: P, stack_screen_index: usize) -> Self {
        Self {
            proxy,
            stack_screen_index,
            monitors: Vec::new(),
            windows: HashMap::new(),
            desktops: HashMap::new(),
            active_workspace: 0,
        }
    }

    pub fn proxy(&self) -> &P {
        &self.proxy
    }

    pub fn proxy_mut(&mut self) -> &mut P {
        &mut self.proxy
    }

    fn desktop(&mut self, ws: u32) -> &mut VirtualDesktop {
        self.desktops
            .entry(ws)
            .or_insert_with(|| VirtualDesktop::new(ws))
    }

    /// Public mutable access to a virtual desktop, creating it if absent.
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
            None => return Ok(()),
        };

        // Collect tileable window IDs for this workspace
        let window_ids: Vec<u64> = self
            .desktops
            .get(&workspace_id)
            .map(|d| d.stack_windows.clone())
            .unwrap_or_default();

        let positions = stack_layout(&window_ids, screen);

        for (id, rect) in positions {
            self.proxy
                .move_resize_window(id, rect.x, rect.y, rect.width, rect.height)
                .await?;
        }

        Ok(())
    }

    /// Check if a window type string represents a toplevel window.
    fn is_toplevel_type(wtype: &str) -> bool {
        wtype == "normal"
    }

    /// Initialize the engine: load monitors, enumerate existing windows, tile.
    pub async fn startup(&mut self) -> ProxyResult<()> {
        self.monitors = self.proxy.get_monitors().await?;
        self.active_workspace = self.proxy.get_active_workspace().await?;

        let windows = self.proxy.list_windows().await?;
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
    ) -> ProxyResult<()> {
        let wtype = self.proxy.get_window_type(window_id).await?;
        let is_fs = self.proxy.is_fullscreen(window_id).await?;
        let is_tl = Self::is_toplevel_type(&wtype);

        let tracked = TrackedWindow {
            id: window_id,
            workspace_id: self.active_workspace,
            monitor_id: self.stack_screen_index as u32,
            is_fullscreen: is_fs,
            is_toplevel: is_tl,
        };

        self.windows.insert(window_id, tracked);

        if is_tl && !is_fs {
            self.desktop(self.active_workspace)
                .push_window(window_id);
            self.tile_stack(self.active_workspace).await?;
        }

        Ok(())
    }

    /// Handle a window closing.
    pub async fn handle_window_closed(&mut self, window_id: u64) -> ProxyResult<()> {
        let tracked = match self.windows.remove(&window_id) {
            Some(t) => t,
            None => return Ok(()),
        };

        let ws = tracked.workspace_id;
        self.desktop(ws).remove_window(window_id);

        if tracked.is_toplevel && !tracked.is_fullscreen {
            self.tile_stack(ws).await?;
        }

        Ok(())
    }

    /// Handle workspace change.
    pub async fn handle_workspace_changed(&mut self, workspace_id: u32) -> ProxyResult<()> {
        self.active_workspace = workspace_id;
        self.tile_stack(workspace_id).await?;
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
        // If window is not tracked, nothing to do
        let (workspace_id, monitor_id) = match self.windows.get(&window_id) {
            Some(w) => (w.workspace_id, w.monitor_id),
            None => return Ok(()),
        };

        // Check enforcement on this desktop/monitor
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

        // Snap the window back to its expected position
        self.proxy
            .move_resize_window(window_id, expected.x, expected.y, expected.width, expected.height)
            .await?;

        Ok(())
    }

    pub fn active_workspace(&self) -> u32 {
        self.active_workspace
    }

    pub fn desktop_ref(&self, ws: u32) -> Option<&VirtualDesktop> {
        self.desktops.get(&ws)
    }
}
