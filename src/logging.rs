use std::path::PathBuf;
use tracing_appender::non_blocking::WorkerGuard;
use tracing_subscriber::{fmt, EnvFilter};

/// Initialize file-based logging to ~/tiler.log.
///
/// Returns a guard that must be held for the lifetime of the daemon —
/// dropping it flushes and closes the log file.
pub fn init_logging() -> WorkerGuard {
    let log_path = dirs_log_path();

    let file = std::fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open(&log_path)
        .expect("failed to open log file");

    let (non_blocking, guard) = tracing_appender::non_blocking(file);

    let filter = EnvFilter::try_from_default_env()
        .unwrap_or_else(|_| EnvFilter::new("info"));

    fmt()
        .with_env_filter(filter)
        .with_writer(non_blocking)
        .with_ansi(false)
        .with_target(false)
        .with_thread_ids(false)
        .init();

    guard
}

fn dirs_log_path() -> PathBuf {
    let home = std::env::var("HOME").unwrap_or_else(|_| "/tmp".to_string());
    PathBuf::from(home).join("tiler.log")
}
