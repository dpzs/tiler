use std::collections::HashMap;

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct Rect {
    pub x: i32,
    pub y: i32,
    /// Signed to match Wayland/X11 geometry APIs
    pub width: i32,
    /// Signed to match Wayland/X11 geometry APIs
    pub height: i32,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
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
