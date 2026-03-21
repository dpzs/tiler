use clap::{Parser, Subcommand};

/// Tiling window manager for GNOME on Wayland
#[derive(Parser, Debug)]
#[command(name = "tiler")]
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
}
