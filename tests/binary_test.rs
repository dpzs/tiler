use clap::CommandFactory;
use tiler::cli::Cli;

#[test]
fn should_have_binary_name_tiler() {
    // Arrange
    let cmd = Cli::command();

    // Act
    let name = cmd.get_name();

    // Assert
    assert_eq!(name, "tiler", "binary name should be 'tiler'");
}

#[test]
fn should_have_version_set() {
    // Arrange
    let cmd = Cli::command();

    // Act
    let version = cmd.get_version();

    // Assert
    assert_eq!(
        version,
        Some("0.1.0"),
        "binary version should be '0.1.0' (requires #[command(version)] on Cli)"
    );
}
