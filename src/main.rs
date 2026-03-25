use clap::Parser;
use std::path::PathBuf;
use tiler::cli::{Cli, Commands};
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
            let _log_guard = tiler::logging::init_logging();

            let proxy = match ZbusGnomeProxy::connect().await {
                Ok(p) => p,
                Err(e) => {
                    tracing::error!("failed to connect to GNOME Shell extension: {e}");
                    eprintln!("error: {e}");
                    std::process::exit(1);
                }
            };
            let (tx, rx) = tokio::sync::mpsc::unbounded_channel();
            proxy.spawn_signal_listener(tx);
            run_daemon(proxy, &sock, 0, None, Some(rx)).await.map(|_| None)
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
        Ok(Some(resp)) => println!("{:?}", resp),
        Ok(None) => {}
        Err(e) => {
            eprintln!("error: {e}");
            std::process::exit(1);
        }
    }
}
