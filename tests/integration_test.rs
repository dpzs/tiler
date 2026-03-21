use std::process::Command;

fn tiler_bin() -> Command {
    Command::new(env!("CARGO_BIN_EXE_tiler"))
}

#[test]
fn should_print_version() {
    // Act
    let output = tiler_bin()
        .arg("--version")
        .output()
        .expect("failed to execute tiler binary");

    // Assert
    assert!(output.status.success(), "tiler --version should exit 0");
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert_eq!(
        stdout.trim(),
        "tiler 0.1.0",
        "expected 'tiler 0.1.0', got '{}'",
        stdout.trim()
    );
}

#[test]
fn should_list_subcommands_in_help() {
    // Act
    let output = tiler_bin()
        .arg("--help")
        .output()
        .expect("failed to execute tiler binary");

    // Assert
    assert!(output.status.success(), "tiler --help should exit 0");
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(
        stdout.contains("daemon"),
        "help output should mention 'daemon' subcommand, got:\n{}",
        stdout
    );
    assert!(
        stdout.contains("menu"),
        "help output should mention 'menu' subcommand, got:\n{}",
        stdout
    );
}
