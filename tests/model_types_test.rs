use std::collections::HashMap;
use tiler::model::{LayoutPreset, Monitor, Rect, VirtualDesktop, Window};

// ---------------------------------------------------------------------------
// Rect
// ---------------------------------------------------------------------------

#[test]
fn test_rect_construction() {
    // Arrange / Act
    let r = Rect {
        x: 10,
        y: 20,
        width: 800,
        height: 600,
    };

    // Assert
    assert_eq!(r.x, 10);
    assert_eq!(r.y, 20);
    assert_eq!(r.width, 800);
    assert_eq!(r.height, 600);
}

#[test]
fn test_rect_zero_coordinates() {
    // Arrange / Act
    let r = Rect {
        x: 0,
        y: 0,
        width: 0,
        height: 0,
    };

    // Assert
    assert_eq!(r.x, 0);
    assert_eq!(r.y, 0);
    assert_eq!(r.width, 0);
    assert_eq!(r.height, 0);
}

#[test]
fn test_rect_negative_coordinates() {
    // Arrange / Act
    let r = Rect {
        x: -100,
        y: -200,
        width: 1920,
        height: 1080,
    };

    // Assert
    assert_eq!(r.x, -100);
    assert_eq!(r.y, -200);
    assert_eq!(r.width, 1920);
    assert_eq!(r.height, 1080);
}

#[test]
fn test_rect_clone_and_copy() {
    // Arrange
    let r1 = Rect {
        x: 5,
        y: 10,
        width: 100,
        height: 200,
    };

    // Act
    let r2 = r1; // Copy
    let r3 = r1.clone(); // Clone

    // Assert — all three are equal and independent
    assert_eq!(r1, r2);
    assert_eq!(r1, r3);
}

#[test]
fn test_rect_equality() {
    // Arrange
    let a = Rect { x: 1, y: 2, width: 3, height: 4 };
    let b = Rect { x: 1, y: 2, width: 3, height: 4 };
    let c = Rect { x: 9, y: 2, width: 3, height: 4 };

    // Assert
    assert_eq!(a, b);
    assert_ne!(a, c);
}

#[test]
fn test_rect_json_roundtrip() {
    // Arrange
    let r = Rect {
        x: -50,
        y: 100,
        width: 1920,
        height: 1080,
    };

    // Act
    let json = serde_json::to_string(&r).expect("serialize Rect");
    let deserialized: Rect = serde_json::from_str(&json).expect("deserialize Rect");

    // Assert
    assert_eq!(r, deserialized);
}

#[test]
fn test_rect_debug_format() {
    // Arrange
    let r = Rect { x: 0, y: 0, width: 10, height: 20 };

    // Act
    let debug = format!("{:?}", r);

    // Assert — just confirm Debug is implemented and contains field names
    assert!(debug.contains("Rect"), "Debug output should contain type name: {}", debug);
}

// ---------------------------------------------------------------------------
// LayoutPreset
// ---------------------------------------------------------------------------

#[test]
fn test_layout_preset_all_variants_exist() {
    // Arrange / Act
    let presets = [
        LayoutPreset::Fullscreen,
        LayoutPreset::SideBySide,
        LayoutPreset::TopBottom,
        LayoutPreset::Quadrants,
    ];

    // Assert — all four are distinct
    for i in 0..presets.len() {
        for j in (i + 1)..presets.len() {
            assert_ne!(
                presets[i], presets[j],
                "variants {:?} and {:?} should be distinct",
                presets[i], presets[j]
            );
        }
    }
}

#[test]
fn test_layout_preset_copy_and_clone() {
    // Arrange
    let p1 = LayoutPreset::SideBySide;

    // Act
    let p2 = p1; // Copy
    let p3 = p1.clone(); // Clone

    // Assert
    assert_eq!(p1, p2);
    assert_eq!(p1, p3);
}

#[test]
fn test_layout_preset_json_roundtrip() {
    // Arrange
    let variants = [
        LayoutPreset::Fullscreen,
        LayoutPreset::SideBySide,
        LayoutPreset::TopBottom,
        LayoutPreset::Quadrants,
    ];

    for variant in &variants {
        // Act
        let json = serde_json::to_string(variant).expect("serialize LayoutPreset");
        let deserialized: LayoutPreset =
            serde_json::from_str(&json).expect("deserialize LayoutPreset");

        // Assert
        assert_eq!(*variant, deserialized, "round-trip failed for {:?}", variant);
    }
}

// ---------------------------------------------------------------------------
// Window
// ---------------------------------------------------------------------------

#[test]
fn test_window_construction() {
    // Arrange / Act
    let w = Window {
        id: 42,
        title: "Firefox".to_string(),
        app_class: "firefox".to_string(),
        monitor_id: 1,
        tile_position: Rect { x: 0, y: 0, width: 960, height: 1080 },
        virtual_desktop_id: 0,
        is_fullscreen: false,
    };

    // Assert
    assert_eq!(w.id, 42);
    assert_eq!(w.title, "Firefox");
    assert_eq!(w.app_class, "firefox");
    assert_eq!(w.monitor_id, 1);
    assert_eq!(w.tile_position.width, 960);
    assert_eq!(w.virtual_desktop_id, 0);
    assert!(!w.is_fullscreen);
}

#[test]
fn test_window_clone() {
    // Arrange
    let w1 = Window {
        id: 1,
        title: "Terminal".to_string(),
        app_class: "gnome-terminal".to_string(),
        monitor_id: 0,
        tile_position: Rect { x: 0, y: 0, width: 1920, height: 1080 },
        virtual_desktop_id: 0,
        is_fullscreen: true,
    };

    // Act
    let w2 = w1.clone();

    // Assert
    assert_eq!(w1, w2);
}

#[test]
fn test_window_json_roundtrip() {
    // Arrange
    let w = Window {
        id: 99,
        title: "Code Editor".to_string(),
        app_class: "code".to_string(),
        monitor_id: 2,
        tile_position: Rect { x: 960, y: 0, width: 960, height: 1080 },
        virtual_desktop_id: 1,
        is_fullscreen: false,
    };

    // Act
    let json = serde_json::to_string(&w).expect("serialize Window");
    let deserialized: Window = serde_json::from_str(&json).expect("deserialize Window");

    // Assert
    assert_eq!(w, deserialized);
}

// ---------------------------------------------------------------------------
// Monitor
// ---------------------------------------------------------------------------

#[test]
fn test_monitor_construction() {
    // Arrange / Act
    let m = Monitor {
        id: 0,
        name: "DP-1".to_string(),
        position: 0,
        width: 2560,
        height: 1440,
        is_stack: false,
    };

    // Assert
    assert_eq!(m.id, 0);
    assert_eq!(m.name, "DP-1");
    assert_eq!(m.position, 0);
    assert_eq!(m.width, 2560);
    assert_eq!(m.height, 1440);
    assert!(!m.is_stack);
}

#[test]
fn test_monitor_stack_flag() {
    // Arrange / Act
    let m = Monitor {
        id: 1,
        name: "eDP-1".to_string(),
        position: 1,
        width: 1920,
        height: 1080,
        is_stack: true,
    };

    // Assert
    assert!(m.is_stack);
}

#[test]
fn test_monitor_json_roundtrip() {
    // Arrange
    let m = Monitor {
        id: 2,
        name: "HDMI-1".to_string(),
        position: 2,
        width: 3840,
        height: 2160,
        is_stack: false,
    };

    // Act
    let json = serde_json::to_string(&m).expect("serialize Monitor");
    let deserialized: Monitor = serde_json::from_str(&json).expect("deserialize Monitor");

    // Assert
    assert_eq!(m, deserialized);
}

// ---------------------------------------------------------------------------
// VirtualDesktop
// ---------------------------------------------------------------------------

#[test]
fn test_virtual_desktop_construction() {
    // Arrange
    let mut layout_presets = HashMap::new();
    layout_presets.insert(0, LayoutPreset::SideBySide);
    layout_presets.insert(1, LayoutPreset::Fullscreen);

    let mut enforcement_modes = HashMap::new();
    enforcement_modes.insert(0, true);
    enforcement_modes.insert(1, false);

    let stack_windows = vec![100, 200, 300];

    // Act
    let vd = VirtualDesktop {
        id: 1,
        layout_presets: layout_presets.clone(),
        enforcement_modes: enforcement_modes.clone(),
        stack_windows: stack_windows.clone(),
    };

    // Assert
    assert_eq!(vd.id, 1);
    assert_eq!(vd.layout_presets.len(), 2);
    assert_eq!(vd.layout_presets[&0], LayoutPreset::SideBySide);
    assert_eq!(vd.layout_presets[&1], LayoutPreset::Fullscreen);
    assert_eq!(vd.enforcement_modes[&0], true);
    assert_eq!(vd.enforcement_modes[&1], false);
    assert_eq!(vd.stack_windows, vec![100, 200, 300]);
}

#[test]
fn test_virtual_desktop_empty() {
    // Arrange / Act
    let vd = VirtualDesktop {
        id: 0,
        layout_presets: HashMap::new(),
        enforcement_modes: HashMap::new(),
        stack_windows: Vec::new(),
    };

    // Assert
    assert_eq!(vd.id, 0);
    assert!(vd.layout_presets.is_empty());
    assert!(vd.enforcement_modes.is_empty());
    assert!(vd.stack_windows.is_empty());
}

#[test]
fn test_virtual_desktop_stack_window_order() {
    // Arrange — newest window first
    let vd = VirtualDesktop {
        id: 0,
        layout_presets: HashMap::new(),
        enforcement_modes: HashMap::new(),
        stack_windows: vec![300, 200, 100],
    };

    // Assert — order is preserved (newest first)
    assert_eq!(vd.stack_windows[0], 300, "newest window should be first");
    assert_eq!(vd.stack_windows[2], 100, "oldest window should be last");
}

#[test]
fn test_virtual_desktop_json_roundtrip() {
    // Arrange
    let mut layout_presets = HashMap::new();
    layout_presets.insert(0, LayoutPreset::Quadrants);

    let mut enforcement_modes = HashMap::new();
    enforcement_modes.insert(0, true);

    let vd = VirtualDesktop {
        id: 5,
        layout_presets,
        enforcement_modes,
        stack_windows: vec![10, 20],
    };

    // Act
    let json = serde_json::to_string(&vd).expect("serialize VirtualDesktop");
    let deserialized: VirtualDesktop =
        serde_json::from_str(&json).expect("deserialize VirtualDesktop");

    // Assert
    assert_eq!(vd, deserialized);
}

#[test]
fn test_virtual_desktop_clone() {
    // Arrange
    let mut layout_presets = HashMap::new();
    layout_presets.insert(0, LayoutPreset::TopBottom);

    let vd1 = VirtualDesktop {
        id: 3,
        layout_presets,
        enforcement_modes: HashMap::new(),
        stack_windows: vec![1, 2, 3],
    };

    // Act
    let vd2 = vd1.clone();

    // Assert
    assert_eq!(vd1, vd2);
}
