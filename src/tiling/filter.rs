use crate::model::{Window, WindowType};

/// Returns true if the window should be included in tiling layout.
/// Filters out dialogs, popups, splash screens, and fullscreen windows.
pub fn is_toplevel(window: &Window) -> bool {
    window.window_type == WindowType::Normal && !window.is_fullscreen
}
