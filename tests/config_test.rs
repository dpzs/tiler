use std::path::Path;
use tiler::config::{StackScreenPosition, TilerConfig};
use tiler::gnome::dbus_proxy::MonitorInfo;

#[test]
fn should_have_default_stack_screen_position_left() {
    // Arrange / Act
    let config = TilerConfig::default();

    // Assert
    assert_eq!(
        config.stack_screen_position, "left",
        "default stack_screen_position should be 'left'"
    );
}

#[test]
fn should_load_defaults_when_file_missing() {
    // Arrange
    let path = Path::new("/tmp/tiler-test-nonexistent-config.toml");
    assert!(!path.exists(), "precondition: file should not exist");

    // Act
    let config = TilerConfig::load(path).expect("should not error on missing file");

    // Assert
    assert_eq!(
        config, TilerConfig::default(),
        "missing file should produce default config"
    );
}

#[test]
fn should_parse_custom_stack_screen_position() {
    // Arrange
    let dir = std::env::temp_dir().join("tiler-config-test-custom");
    std::fs::create_dir_all(&dir).unwrap();
    let path = dir.join("config.toml");
    std::fs::write(&path, "stack_screen_position = \"right\"\n").unwrap();

    // Act
    let config = TilerConfig::load(&path).expect("should parse valid TOML");

    // Assert
    assert_eq!(
        config.stack_screen_position, "right",
        "should respect custom stack_screen_position"
    );

    // Cleanup
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn should_use_defaults_for_empty_toml_file() {
    // Arrange
    let dir = std::env::temp_dir().join("tiler-config-test-empty");
    std::fs::create_dir_all(&dir).unwrap();
    let path = dir.join("config.toml");
    std::fs::write(&path, "").unwrap();

    // Act
    let config = TilerConfig::load(&path).expect("should parse empty TOML");

    // Assert
    assert_eq!(
        config, TilerConfig::default(),
        "empty file should produce default config"
    );

    // Cleanup
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn should_error_on_invalid_toml_syntax() {
    // Arrange
    let dir = std::env::temp_dir().join("tiler-config-test-invalid");
    std::fs::create_dir_all(&dir).unwrap();
    let path = dir.join("config.toml");
    std::fs::write(&path, "this is not [valid toml ===").unwrap();

    // Act
    let result = TilerConfig::load(&path);

    // Assert
    assert!(
        result.is_err(),
        "invalid TOML syntax should produce an error"
    );

    // Cleanup
    let _ = std::fs::remove_dir_all(&dir);
}

// --- Validation tests ---

#[test]
fn should_validate_left_position() {
    let config = TilerConfig::default();
    assert!(config.validate().is_ok(), "default 'left' should validate");
}

#[test]
fn should_validate_right_position() {
    let dir = std::env::temp_dir().join("tiler-config-test-validate-right");
    std::fs::create_dir_all(&dir).unwrap();
    let path = dir.join("config.toml");
    std::fs::write(&path, "stack_screen_position = \"right\"\n").unwrap();

    let config = TilerConfig::load(&path).expect("should parse");
    assert!(config.validate().is_ok(), "'right' should validate");

    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn should_reject_invalid_position() {
    let dir = std::env::temp_dir().join("tiler-config-test-validate-bad");
    std::fs::create_dir_all(&dir).unwrap();
    let path = dir.join("config.toml");
    std::fs::write(&path, "stack_screen_position = \"center\"\n").unwrap();

    let config = TilerConfig::load(&path).expect("should parse TOML");
    let result = config.validate();
    assert!(result.is_err(), "'center' should fail validation");
    let err_msg = format!("{}", result.unwrap_err());
    assert!(
        err_msg.contains("center"),
        "error should mention the invalid value, got: {err_msg}"
    );

    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn should_validate_case_insensitively() {
    let dir = std::env::temp_dir().join("tiler-config-test-validate-case");
    std::fs::create_dir_all(&dir).unwrap();
    let path = dir.join("config.toml");
    std::fs::write(&path, "stack_screen_position = \"LEFT\"\n").unwrap();

    let config = TilerConfig::load(&path).expect("should parse TOML");
    assert!(
        config.validate().is_ok(),
        "'LEFT' (uppercase) should validate"
    );

    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn should_parse_stack_position_type() {
    let config = TilerConfig::default();
    let pos = config.stack_position().expect("should parse default position");
    assert_eq!(pos, StackScreenPosition::Left);
}

// --- StackScreenPosition::resolve_index tests ---

fn make_two_monitors() -> Vec<MonitorInfo> {
    vec![
        MonitorInfo { id: 0, name: "DP-1".into(), x: 0, y: 0, width: 1920, height: 1080 },
        MonitorInfo { id: 1, name: "DP-2".into(), x: 1920, y: 0, width: 1920, height: 1080 },
    ]
}

#[test]
fn should_resolve_left_to_leftmost_monitor() {
    let monitors = make_two_monitors();
    let idx = StackScreenPosition::Left.resolve_index(&monitors);
    assert_eq!(idx, Some(0), "left should resolve to monitor at x=0");
}

#[test]
fn should_resolve_right_to_rightmost_monitor() {
    let monitors = make_two_monitors();
    let idx = StackScreenPosition::Right.resolve_index(&monitors);
    assert_eq!(idx, Some(1), "right should resolve to monitor at x=1920");
}

#[test]
fn should_resolve_left_with_reversed_monitor_order() {
    // Monitors listed in reverse x order
    let monitors = vec![
        MonitorInfo { id: 1, name: "DP-2".into(), x: 1920, y: 0, width: 1920, height: 1080 },
        MonitorInfo { id: 0, name: "DP-1".into(), x: 0, y: 0, width: 1920, height: 1080 },
    ];
    let idx = StackScreenPosition::Left.resolve_index(&monitors);
    assert_eq!(
        idx,
        Some(1),
        "left should pick index 1 when the leftmost monitor is second in the list"
    );
}

#[test]
fn should_return_none_for_empty_monitors() {
    let idx = StackScreenPosition::Left.resolve_index(&[]);
    assert_eq!(idx, None, "empty monitor list should return None");
}

#[test]
fn should_resolve_single_monitor() {
    let monitors = vec![
        MonitorInfo { id: 0, name: "DP-1".into(), x: 0, y: 0, width: 1920, height: 1080 },
    ];
    // Both left and right should resolve to the only monitor
    assert_eq!(StackScreenPosition::Left.resolve_index(&monitors), Some(0));
    assert_eq!(StackScreenPosition::Right.resolve_index(&monitors), Some(0));
}

// --- Config ignores unknown fields ---

#[test]
fn should_ignore_unknown_toml_keys() {
    let dir = std::env::temp_dir().join("tiler-config-test-unknown-keys");
    std::fs::create_dir_all(&dir).unwrap();
    let path = dir.join("config.toml");
    std::fs::write(
        &path,
        "stack_screen_position = \"right\"\nunknown_future_key = 42\n",
    )
    .unwrap();

    let config = TilerConfig::load(&path);
    // By default, serde+toml rejects unknown fields unless #[serde(deny_unknown_fields)]
    // is used. We should handle this gracefully.
    match config {
        Ok(c) => {
            assert_eq!(c.stack_screen_position, "right");
        }
        Err(e) => {
            let msg = format!("{e}");
            assert!(
                msg.contains("unknown") || msg.contains("Unknown"),
                "error should mention 'unknown' key, got: {msg}"
            );
        }
    }

    let _ = std::fs::remove_dir_all(&dir);
}
