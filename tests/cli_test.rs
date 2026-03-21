use clap::Parser;
use tiler::cli::{Cli, Commands};

#[test]
fn should_parse_daemon_subcommand() {
    // Arrange
    let args = ["tiler", "daemon"];

    // Act
    let cli = Cli::try_parse_from(args).expect("should parse daemon subcommand");

    // Assert
    assert!(
        matches!(cli.command, Commands::Daemon),
        "expected Commands::Daemon, got {:?}",
        cli.command
    );
}

#[test]
fn should_parse_menu_subcommand() {
    // Arrange
    let args = ["tiler", "menu"];

    // Act
    let cli = Cli::try_parse_from(args).expect("should parse menu subcommand");

    // Assert
    assert!(
        matches!(cli.command, Commands::Menu),
        "expected Commands::Menu, got {:?}",
        cli.command
    );
}

#[test]
fn should_fail_with_no_subcommand() {
    // Arrange
    let args = ["tiler"];

    // Act
    let result = Cli::try_parse_from(args);

    // Assert
    assert!(
        result.is_err(),
        "expected error when no subcommand is provided"
    );
}

#[test]
fn should_fail_with_unknown_subcommand() {
    // Arrange
    let args = ["tiler", "unknown-cmd"];

    // Act
    let result = Cli::try_parse_from(args);

    // Assert
    assert!(
        result.is_err(),
        "expected error for unknown subcommand"
    );
}
