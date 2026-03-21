use serde::Deserialize;
use std::path::Path;

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
    pub fn load(path: &Path) -> Result<Self, Box<dyn std::error::Error>> {
        let contents = match std::fs::read_to_string(path) {
            Ok(s) => s,
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => {
                return Ok(Self::default());
            }
            Err(e) => return Err(e.into()),
        };
        let config: TilerConfig = toml::from_str(&contents)?;
        Ok(config)
    }
}
