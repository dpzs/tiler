use clap::Parser;
use std::path::PathBuf;
use tiler::cli::{Cli, Commands};
use tiler::ipc::client::send_command;
use tiler::ipc::protocol::Command;
use tiler::ipc::server::run_server;

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
        Commands::Daemon => run_server(&sock).await.map(|_| None),
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
