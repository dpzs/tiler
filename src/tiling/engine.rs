use std::collections::HashMap;

use crate::gnome::dbus_proxy::{GnomeProxy, MonitorInfo, ProxyResult};
use crate::model::{Rect, VirtualDesktop};
use crate::tiling::stack::stack_layout;

/// Tracks a window known to the engine.
struct TrackedWindow {
    #[allow(dead_code)]
    id: u64,
    workspace_id: u32,
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

    pub fn active_workspace(&self) -> u32 {
        self.active_workspace
    }

    pub fn desktop_ref(&self, ws: u32) -> Option<&VirtualDesktop> {
        self.desktops.get(&ws)
    }
}
