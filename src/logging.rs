use std::path::PathBuf;
use tracing_appender::non_blocking::WorkerGuard;
use tracing_subscriber::{fmt, EnvFilter};

/// Initialize file-based logging to ~/tiler.log.
///
/// Returns a guard that must be held for the lifetime of the daemon --
/// dropping it flushes and closes the log file.
///
/// # Errors
///
/// Returns an error if the log file cannot be opened or created.
pub fn init_logging() -> Result<WorkerGuard, std::io::Error> {
    let log_path = dirs_log_path();

    let file = std::fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open(&log_path)
        .map_err(|e| {
            std::io::Error::new(
                e.kind(),
                format!("failed to open log file {}: {e}", log_path.display()),
            )
        })?;

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

    Ok(guard)
}

fn dirs_log_path() -> PathBuf {
    let home = std::env::var("HOME").unwrap_or_else(|_| "/tmp".to_string());
    PathBuf::from(home).join("tiler.log")
}
