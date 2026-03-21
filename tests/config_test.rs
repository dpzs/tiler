use std::path::Path;
use tiler::config::TilerConfig;

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
