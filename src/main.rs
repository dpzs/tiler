use clap::Parser;
use std::path::PathBuf;
use tiler::cli::{Cli, Commands};
use tiler::daemon::run_daemon;
use tiler::gnome::dbus_proxy::MockGnomeProxy;
use tiler::ipc::client::send_command;
use tiler::ipc::protocol::Command;

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
            let proxy = MockGnomeProxy::new();
            run_daemon(proxy, &sock, 0, None, None).await.map(|_| None)
        }
        Commands::Menu => send_command(&sock, Command::Menu).await.map(Some),
        Commands::Status => send_command(&sock, Command::Status).await.map(Some),
    };

    match result {
        Ok(Some(resp)) => println!("{:?}", resp),
        Ok(None) => {}
        Err(e) => {
            eprintln!("error: {e}");
            std::process::exit(1);
        }
    }
}
