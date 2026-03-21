use std::collections::HashMap;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WindowInfo {
    pub id: u64,
    pub title: String,
    pub app_class: String,
    pub monitor_id: u32,
    pub workspace_id: u32,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MonitorInfo {
    pub id: u32,
    pub name: String,
    pub x: i32,
    pub y: i32,
    pub width: i32,
    pub height: i32,
}

pub type ProxyResult<T> = Result<T, Box<dyn std::error::Error + Send + Sync>>;

/// Trait abstracting the GNOME Shell extension D-Bus interface.
/// Used directly for production (zbus proxy) and via MockGnomeProxy for tests.
pub trait GnomeProxy: Send {
    fn list_windows(&self) -> impl std::future::Future<Output = ProxyResult<Vec<WindowInfo>>> + Send;
    fn move_resize_window(
        &mut self,
        window_id: u64,
        x: i32,
        y: i32,
        width: i32,
        height: i32,
    ) -> impl std::future::Future<Output = ProxyResult<()>> + Send;
    fn get_monitors(&self) -> impl std::future::Future<Output = ProxyResult<Vec<MonitorInfo>>> + Send;
    fn get_active_workspace(&self) -> impl std::future::Future<Output = ProxyResult<u32>> + Send;
    fn get_window_type(&self, window_id: u64) -> impl std::future::Future<Output = ProxyResult<String>> + Send;
    fn is_fullscreen(&self, window_id: u64) -> impl std::future::Future<Output = ProxyResult<bool>> + Send;
}

/// Mock implementation for unit testing without a real D-Bus connection.
pub struct MockGnomeProxy {
    windows: Vec<WindowInfo>,
    monitors: Vec<MonitorInfo>,
    active_workspace: u32,
    window_types: HashMap<u64, String>,
    fullscreen_states: HashMap<u64, bool>,
    move_resize_log: Vec<(u64, i32, i32, i32, i32)>,
}

impl MockGnomeProxy {
    pub fn new() -> Self {
        Self {
            windows: Vec::new(),
            monitors: Vec::new(),
            active_workspace: 0,
            window_types: HashMap::new(),
            fullscreen_states: HashMap::new(),
            move_resize_log: Vec::new(),
        }
    }

    pub fn set_windows(&mut self, windows: Vec<WindowInfo>) {
        self.windows = windows;
    }

    pub fn set_monitors(&mut self, monitors: Vec<MonitorInfo>) {
        self.monitors = monitors;
    }

    pub fn set_active_workspace(&mut self, ws: u32) {
        self.active_workspace = ws;
    }

    pub fn set_window_type(&mut self, window_id: u64, wtype: String) {
        self.window_types.insert(window_id, wtype);
    }

    pub fn set_fullscreen(&mut self, window_id: u64, fs: bool) {
        self.fullscreen_states.insert(window_id, fs);
    }

    pub fn move_resize_calls(&self) -> &[(u64, i32, i32, i32, i32)] {
        &self.move_resize_log
    }

    /// Synchronous snapshot of configured windows (for test setup helpers).
    pub fn list_windows_snapshot(&self) -> Vec<WindowInfo> {
        self.windows.clone()
    }
}

impl GnomeProxy for MockGnomeProxy {
    async fn list_windows(&self) -> ProxyResult<Vec<WindowInfo>> {
        Ok(self.windows.clone())
    }

    async fn move_resize_window(
        &mut self,
        window_id: u64,
        x: i32,
        y: i32,
        width: i32,
        height: i32,
    ) -> ProxyResult<()> {
        self.move_resize_log.push((window_id, x, y, width, height));
        Ok(())
    }

    async fn get_monitors(&self) -> ProxyResult<Vec<MonitorInfo>> {
        Ok(self.monitors.clone())
    }

    async fn get_active_workspace(&self) -> ProxyResult<u32> {
        Ok(self.active_workspace)
    }

    async fn get_window_type(&self, window_id: u64) -> ProxyResult<String> {
        Ok(self
            .window_types
            .get(&window_id)
            .cloned()
            .unwrap_or_else(|| "normal".to_string()))
    }

    async fn is_fullscreen(&self, window_id: u64) -> ProxyResult<bool> {
        Ok(self.fullscreen_states.get(&window_id).copied().unwrap_or(false))
    }
}
