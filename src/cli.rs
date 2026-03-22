use clap::{Parser, Subcommand};

/// Tiling window manager for GNOME on Wayland
#[derive(Parser, Debug)]
#[command(name = "tiler", version)]
pub struct Cli {
    #[command(subcommand)]
    pub command: Commands,
}

#[derive(Subcommand, Debug)]
pub enum Commands {
    /// Run the tiling daemon
    Daemon,
    /// Open the floating menu
    Menu,
    /// Query daemon status
    Status,
    /// Apply a layout to a monitor (1=fullscreen, 2=side-by-side, 3=top-bottom, 4=quadrants)
    Apply {
        /// Monitor number (1-based, as shown in menu)
        monitor: u32,
        /// Layout number: 1=fullscreen, 2=side-by-side, 3=top-bottom, 4=quadrants
        layout: u8,
    },
    /// List all windows with their positions
    Windows,
}
