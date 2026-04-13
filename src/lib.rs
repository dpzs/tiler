// Suppress pedantic doc lints project-wide — these are internal APIs and the
// overhead of `# Errors` / `# Panics` sections is not justified here.
#![allow(clippy::missing_errors_doc, clippy::missing_panics_doc)]

pub mod cli;
pub mod config;
pub mod daemon;
pub mod gnome;
pub mod ipc;
pub mod logging;
pub mod menu;
pub mod model;
pub mod tiling;
