use serde::Deserialize;
use std::path::Path;

use crate::gnome::dbus_proxy::MonitorInfo;

/// Valid positions for the stack screen relative to other monitors.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum StackScreenPosition {
    Left,
    Right,
}

impl StackScreenPosition {
    /// Parse a position string, returning an error for unrecognized values.
    pub fn parse(s: &str) -> Result<Self, ConfigError> {
        match s.to_lowercase().as_str() {
            "left" => Ok(Self::Left),
            "right" => Ok(Self::Right),
            _ => Err(ConfigError::InvalidStackScreenPosition(s.to_string())),
        }
    }

    /// Resolve the stack screen index from a list of monitors.
    ///
    /// For "left", picks the monitor with the smallest x coordinate.
    /// For "right", picks the monitor with the largest x coordinate.
    /// Returns `None` if the monitor list is empty.
    #[must_use]
    pub fn resolve_index(&self, monitors: &[MonitorInfo]) -> Option<usize> {
        if monitors.is_empty() {
            return None;
        }
        match self {
            Self::Left => monitors
                .iter()
                .enumerate()
                .min_by_key(|(_, m)| m.x)
                .map(|(i, _)| i),
            Self::Right => monitors
                .iter()
                .enumerate()
                .max_by_key(|(_, m)| m.x)
                .map(|(i, _)| i),
        }
    }
}

/// Errors specific to tiler configuration.
#[derive(Debug)]
pub enum ConfigError {
    /// The `stack_screen_position` value is not "left" or "right".
    InvalidStackScreenPosition(String),
    /// An I/O error occurred while reading the config file.
    Io(std::io::Error),
    /// The config file contains invalid TOML.
    Parse(toml::de::Error),
}

impl std::fmt::Display for ConfigError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::InvalidStackScreenPosition(v) => {
                write!(
                    f,
                    "invalid stack_screen_position '{v}': expected 'left' or 'right'"
                )
            }
            Self::Io(e) => write!(f, "config I/O error: {e}"),
            Self::Parse(e) => write!(f, "config parse error: {e}"),
        }
    }
}

impl std::error::Error for ConfigError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::Io(e) => Some(e),
            Self::Parse(e) => Some(e),
            Self::InvalidStackScreenPosition(_) => None,
        }
    }
}

impl From<std::io::Error> for ConfigError {
    fn from(e: std::io::Error) -> Self {
        Self::Io(e)
    }
}

impl From<toml::de::Error> for ConfigError {
    fn from(e: toml::de::Error) -> Self {
        Self::Parse(e)
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
#[serde(default)]
pub struct TilerConfig {
    pub stack_screen_position: String,
}

impl Default for TilerConfig {
    fn default() -> Self {
        Self {
            stack_screen_position: "left".to_string(),
        }
    }
}

impl TilerConfig {
    /// Load config from a TOML file. Returns defaults if the file does not exist.
    pub fn load(path: &Path) -> Result<Self, ConfigError> {
        let contents = match std::fs::read_to_string(path) {
            Ok(s) => s,
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => {
                return Ok(Self::default());
            }
            Err(e) => return Err(ConfigError::Io(e)),
        };
        let config: TilerConfig = toml::from_str(&contents)?;
        Ok(config)
    }

    /// Validate the configuration, returning a typed error for invalid values.
    pub fn validate(&self) -> Result<(), ConfigError> {
        StackScreenPosition::parse(&self.stack_screen_position)?;
        Ok(())
    }

    /// Parse and return the stack screen position.
    pub fn stack_position(&self) -> Result<StackScreenPosition, ConfigError> {
        StackScreenPosition::parse(&self.stack_screen_position)
    }

    /// Return the config file path, checking in order:
    ///
    /// 1. `$TILER_CONFIG` (explicit override, useful for NixOS module or testing)
    /// 2. `$XDG_CONFIG_HOME/tiler/config.toml`
    /// 3. `~/.config/tiler/config.toml`
    /// 4. `/etc/tiler/config.toml` (system-wide, e.g. NixOS `environment.etc`)
    ///
    /// Returns the first path that exists on disk. If none exist, returns the
    /// XDG path (so that `load()` falls through to defaults).
    #[must_use]
    pub fn default_path() -> std::path::PathBuf {
        // Explicit override via environment variable
        if let Ok(path) = std::env::var("TILER_CONFIG") {
            return std::path::PathBuf::from(path);
        }

        let user_path = if let Ok(dir) = std::env::var("XDG_CONFIG_HOME") {
            std::path::PathBuf::from(dir).join("tiler").join("config.toml")
        } else {
            let home = std::env::var("HOME").unwrap_or_else(|_| "/tmp".to_string());
            std::path::PathBuf::from(home)
                .join(".config")
                .join("tiler")
                .join("config.toml")
        };

        if user_path.exists() {
            return user_path;
        }

        let system_path = std::path::PathBuf::from("/etc/tiler/config.toml");
        if system_path.exists() {
            return system_path;
        }

        // Neither exists; return user path so load() uses defaults
        user_path
    }
}
