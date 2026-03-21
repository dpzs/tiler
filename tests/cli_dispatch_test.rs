use std::process::Command;

fn tiler_bin() -> Command {
    Command::new(env!("CARGO_BIN_EXE_tiler"))
}

#[test]
fn should_exit_nonzero_when_menu_and_no_daemon() {
    // Act
    let output = tiler_bin()
        .arg("menu")
        .output()
        .expect("failed to execute tiler binary");

    // Assert
    assert!(
        !output.status.success(),
        "tiler menu should exit non-zero when daemon is not running"
    );
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        stderr.contains("daemon") || stderr.contains("connect") || stderr.contains("error"),
        "stderr should indicate daemon is not running, got: {}",
        stderr
    );
}

#[test]
fn should_exit_nonzero_when_status_and_no_daemon() {
    // Act
    let output = tiler_bin()
        .arg("status")
        .output()
        .expect("failed to execute tiler binary");

    // Assert
    assert!(
        !output.status.success(),
        "tiler status should exit non-zero when daemon is not running"
    );
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        stderr.contains("daemon") || stderr.contains("connect") || stderr.contains("error"),
        "stderr should indicate daemon is not running, got: {}",
        stderr
    );
}

#[test]
fn should_list_status_subcommand_in_help() {
    // Act
    let output = tiler_bin()
        .arg("--help")
        .output()
        .expect("failed to execute tiler binary");

    // Assert
    assert!(output.status.success(), "tiler --help should exit 0");
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(
        stdout.contains("status"),
        "help output should mention 'status' subcommand, got:\n{}",
        stdout
    );
}
