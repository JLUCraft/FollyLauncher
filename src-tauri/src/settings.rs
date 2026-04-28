use serde::{Deserialize, Serialize};
use std::path::Path;
use tracing::info;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GameSettings {
    pub java_path: String,
    pub max_memory_mb: u32,
    pub jvm_args: String,
    pub game_directory: String,
    pub resolution_width: u32,
    pub resolution_height: u32,
    pub fullscreen: bool,
    pub show_game_log: bool,
}

impl GameSettings {
    pub fn load(data_dir: &Path) -> anyhow::Result<Self> {
        let path = data_dir.join("game_settings.toml");
        let contents = std::fs::read_to_string(&path)
            .map_err(|e| anyhow::anyhow!("failed to read {}: {}", path.display(), e))?;
        let settings: Self = toml::from_str(&contents)
            .map_err(|e| anyhow::anyhow!("failed to parse {}: {}", path.display(), e))?;
        info!("loaded game settings from {}", path.display());
        Ok(settings)
    }

    pub fn save(&self, data_dir: &Path) -> anyhow::Result<()> {
        let path = data_dir.join("game_settings.toml");
        let contents = toml::to_string_pretty(self)
            .map_err(|e| anyhow::anyhow!("failed to serialize game settings: {}", e))?;
        std::fs::write(&path, contents)?;
        info!("saved game settings to {}", path.display());
        Ok(())
    }

    pub fn build_jvm_args(&self) -> Vec<String> {
        let mut args = Vec::new();

        args.push(format!("-Xmx{}M", self.max_memory_mb));
        if !self.jvm_args.trim().is_empty() {
            for arg in self.jvm_args.split_whitespace() {
                args.push(arg.to_string());
            }
        }

        args
    }
}
