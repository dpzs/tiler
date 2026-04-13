use clap::Parser;
use std::path::PathBuf;
use tiler::cli::{Cli, Commands};
use tiler::config::TilerConfig;
use tiler::daemon::run_daemon;
use tiler::gnome::zbus_proxy::ZbusGnomeProxy;
use tiler::ipc::client::send_command;
use tiler::ipc::protocol::{Command, Response};

fn socket_path() -> PathBuf {
    if let Ok(dir) = std::env::var("XDG_RUNTIME_DIR") {
        PathBuf::from(dir).join("tiler.sock")
    } else {
        std::env::temp_dir().join("tiler.sock")
    }
}

#[tokio::main]
async fn main() {
    let cli = Cli::parse();
    let sock = socket_path();

    let result = match cli.command {
        Commands::Daemon => {
            // Only the daemon process needs file logging
            let _log_guard = match tiler::logging::init_logging() {
                Ok(guard) => guard,
                Err(e) => {
                    eprintln!("error: {e}");
                    std::process::exit(1);
                }
            };

            // Load and validate configuration
            let config_path = TilerConfig::default_path();
            let config = match TilerConfig::load(&config_path) {
                Ok(c) => c,
                Err(e) => {
                    tracing::error!(path = %config_path.display(), "failed to load config: {e}");
                    eprintln!("error: failed to load config from {}: {e}", config_path.display());
                    std::process::exit(1);
                }
            };
            if let Err(e) = config.validate() {
                tracing::error!("invalid configuration: {e}");
                eprintln!("error: invalid configuration: {e}");
                std::process::exit(1);
            }
            tracing::info!(
                config_path = %config_path.display(),
                stack_screen_position = %config.stack_screen_position,
                "configuration loaded"
            );

            let proxy = match ZbusGnomeProxy::connect().await {
                Ok(p) => p,
                Err(e) => {
                    tracing::error!("failed to connect to GNOME Shell extension: {e}");
                    eprintln!("error: {e}");
                    std::process::exit(1);
                }
            };

            // Set up signal handling for graceful shutdown
            let (shutdown_tx, shutdown_rx) = tokio::sync::oneshot::channel::<()>();
            tokio::spawn(async move {
                let mut sigterm = tokio::signal::unix::signal(
                    tokio::signal::unix::SignalKind::terminate(),
                )
                .expect("failed to register SIGTERM handler");
                let mut sigint = tokio::signal::unix::signal(
                    tokio::signal::unix::SignalKind::interrupt(),
                )
                .expect("failed to register SIGINT handler");
                tokio::select! {
                    _ = sigterm.recv() => {
                        tracing::info!("received SIGTERM");
                    }
                    _ = sigint.recv() => {
                        tracing::info!("received SIGINT");
                    }
                }
                let _ = shutdown_tx.send(());
            });

            let stack_position = config
                .stack_position()
                .expect("config was validated above");

            let (tx, rx) = tokio::sync::mpsc::unbounded_channel();
            proxy.spawn_signal_listener(tx);
            run_daemon(proxy, &sock, stack_position, Some(shutdown_rx), Some(rx))
                .await
                .map(|()| None)
        }
        Commands::Menu => send_command(&sock, Command::Menu).await.map(Some),
        Commands::Status => send_command(&sock, Command::Status).await.map(Some),
        Commands::Apply { monitor, layout } => {
            send_command(&sock, Command::ApplyLayout { monitor: monitor - 1, layout }).await.map(Some)
        }
        Commands::Windows => send_command(&sock, Command::Windows).await.map(Some),
    };

    match result {
        Ok(Some(Response::Windows(json))) => println!("{json}"),
        Ok(Some(resp)) => println!("{resp:?}"),
        Ok(None) => {}
        Err(e) => {
            eprintln!("error: {e}");
            std::process::exit(1);
        }
    }
}
