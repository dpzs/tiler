#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Event {
    WindowOpened {
        window_id: u64,
        title: String,
        app_class: String,
        monitor_id: u32,
    },
    WindowClosed {
        window_id: u64,
    },
    WindowFocusChanged {
        window_id: u64,
    },
    WorkspaceChanged {
        workspace_id: u32,
    },
    WindowFullscreenChanged {
        window_id: u64,
        is_fullscreen: bool,
    },
    WindowGeometryChanged {
        window_id: u64,
        x: i32,
        y: i32,
        width: i32,
        height: i32,
    },
    MenuKeyPressed {
        key: String,
        modifiers: String,
    },
}
