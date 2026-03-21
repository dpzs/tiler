use tiler::model::{Rect, Window, WindowType};
use tiler::tiling::filter::is_toplevel;

fn make_window(id: u64, window_type: WindowType, is_fullscreen: bool) -> Window {
    Window {
        id,
        title: format!("window-{}", id),
        app_class: "test".to_string(),
        monitor_id: 0,
        tile_position: Rect { x: 0, y: 0, width: 100, height: 100 },
        virtual_desktop_id: 0,
        is_fullscreen,
        window_type,
    }
}

// ---------------------------------------------------------------------------
// Normal windows are toplevel
// ---------------------------------------------------------------------------

#[test]
fn test_normal_window_is_toplevel() {
    let w = make_window(1, WindowType::Normal, false);
    assert!(is_toplevel(&w), "normal non-fullscreen window is toplevel");
}

// ---------------------------------------------------------------------------
// Fullscreen windows are NOT toplevel
// ---------------------------------------------------------------------------

#[test]
fn test_fullscreen_window_not_toplevel() {
    let w = make_window(2, WindowType::Normal, true);
    assert!(!is_toplevel(&w), "fullscreen window should be filtered out");
}

// ---------------------------------------------------------------------------
// Dialog windows are NOT toplevel
// ---------------------------------------------------------------------------

#[test]
fn test_dialog_not_toplevel() {
    let w = make_window(3, WindowType::Dialog, false);
    assert!(!is_toplevel(&w), "dialog should be filtered out");
}

// ---------------------------------------------------------------------------
// Popup windows are NOT toplevel
// ---------------------------------------------------------------------------

#[test]
fn test_popup_not_toplevel() {
    let w = make_window(4, WindowType::Popup, false);
    assert!(!is_toplevel(&w), "popup should be filtered out");
}

// ---------------------------------------------------------------------------
// Splash windows are NOT toplevel
// ---------------------------------------------------------------------------

#[test]
fn test_splash_not_toplevel() {
    let w = make_window(5, WindowType::Splash, false);
    assert!(!is_toplevel(&w), "splash should be filtered out");
}

// ---------------------------------------------------------------------------
// Filtering a list of windows
// ---------------------------------------------------------------------------

#[test]
fn test_filter_mixed_windows() {
    let windows = vec![
        make_window(1, WindowType::Normal, false),
        make_window(2, WindowType::Dialog, false),
        make_window(3, WindowType::Normal, true),  // fullscreen
        make_window(4, WindowType::Popup, false),
        make_window(5, WindowType::Normal, false),
    ];

    let toplevel: Vec<u64> = windows.iter()
        .filter(|w| is_toplevel(w))
        .map(|w| w.id)
        .collect();

    assert_eq!(toplevel, vec![1, 5], "only normal non-fullscreen windows");
}
