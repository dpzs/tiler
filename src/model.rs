use std::collections::HashMap;

use serde::{Deserialize, Serialize};

pub struct Rect {
    pub x: i32,
    pub y: i32,
    /// Signed to match Wayland/X11 geometry APIs
    pub width: i32,
    /// Signed to match Wayland/X11 geometry APIs
    pub height: i32,
}

pub enum LayoutPreset {
    Fullscreen,
    SideBySide,
    TopBottom,
    Quadrants,
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Window {
    pub id: u64,
    pub title: String,
    pub app_class: String,
    pub monitor_id: u32,
    pub tile_position: Rect,
    pub virtual_desktop_id: u32,
    pub is_fullscreen: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Monitor {
    pub id: u32,
    pub name: String,
    /// Left-to-right display order index
    pub position: u32,
    pub width: u32,
    pub height: u32,
    pub is_stack: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct VirtualDesktop {
    pub id: u32,
    pub layout_presets: HashMap<u32, LayoutPreset>,
    /// Per-monitor enforcement: true = layout is enforced, false = relaxed
    pub enforcement_modes: HashMap<u32, bool>,
    pub stack_windows: Vec<u64>,
}

impl VirtualDesktop {
    pub fn new(id: u32) -> Self {
        VirtualDesktop {
            id,
            layout_presets: HashMap::new(),
            enforcement_modes: HashMap::new(),
            stack_windows: Vec::new(),
        }
    }

    pub fn set_layout(&mut self, monitor_id: u32, preset: LayoutPreset) {
        self.layout_presets.insert(monitor_id, preset);
    }

    pub fn get_layout(&self, monitor_id: u32) -> Option<LayoutPreset> {
        self.layout_presets.get(&monitor_id).copied()
    }

    pub fn set_enforcement(&mut self, monitor_id: u32, enforced: bool) {
        self.enforcement_modes.insert(monitor_id, enforced);
    }

    pub fn is_enforced(&self, monitor_id: u32) -> bool {
        self.enforcement_modes.get(&monitor_id).copied().unwrap_or(false)
    }

    /// Add a window to the front of the stack. If already present, move it to front.
    pub fn push_window(&mut self, window_id: u64) {
        self.stack_windows.retain(|&id| id != window_id);
        self.stack_windows.insert(0, window_id);
    }

    /// Add a window to the back of the stack. If already present, move it to back.
    pub fn append_window(&mut self, window_id: u64) {
        self.stack_windows.retain(|&id| id != window_id);
        self.stack_windows.push(window_id);
    }

    pub fn remove_window(&mut self, window_id: u64) {
        self.stack_windows.retain(|&id| id != window_id);
    }
}
