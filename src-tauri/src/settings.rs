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
    /// P2-4 phase A: persisted last-launch tracking for resume.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub last_instance_id: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub last_peer_id: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub last_connected_at: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub last_version: Option<String>,
}

impl Default for GameSettings {
    fn default() -> Self {
        Self {
            java_path: String::new(),
            max_memory_mb: 4096,
            jvm_args: String::new(),
            game_directory: String::new(),
            resolution_width: 1280,
            resolution_height: 720,
            fullscreen: false,
            show_game_log: false,
            last_instance_id: None,
            last_peer_id: None,
            last_connected_at: None,
            last_version: None,
        }
    }
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

    /// Load settings from `game_settings.toml`. If the file does not exist, create
    /// the data directory, write default settings, and return the default.
    /// If the file exists but fails to parse, return an error without overwriting.
    pub fn load_or_default(data_dir: &Path) -> anyhow::Result<Self> {
        let path = data_dir.join("game_settings.toml");
        if path.exists() {
            return Self::load(data_dir);
        }

        // Create data_dir and write defaults.
        std::fs::create_dir_all(data_dir).map_err(|e| {
            anyhow::anyhow!(
                "failed to create data directory {}: {}",
                data_dir.display(),
                e
            )
        })?;

        let default = Self {
            game_directory: data_dir.join("minecraft").to_string_lossy().to_string(),
            ..Self::default()
        };
        default.save(data_dir)?;
        info!("created default game settings in {}", path.display());
        Ok(default)
    }

    pub fn save(&self, data_dir: &Path) -> anyhow::Result<()> {
        // Ensure parent directory exists before writing.
        std::fs::create_dir_all(data_dir).map_err(|e| {
            anyhow::anyhow!(
                "failed to create data directory {}: {}",
                data_dir.display(),
                e
            )
        })?;

        let path = data_dir.join("game_settings.toml");
        let contents = toml::to_string_pretty(self)
            .map_err(|e| anyhow::anyhow!("failed to serialize game settings: {}", e))?;
        std::fs::write(&path, contents)?;
        info!("saved game settings to {}", path.display());
        Ok(())
    }

    /// Validate settings for launching Minecraft. Returns an error with a
    /// user-actionable message if any required field is missing or invalid.
    pub fn validate(&self) -> anyhow::Result<()> {
        if self.java_path.trim().is_empty() {
            anyhow::bail!("请先在「设置」页面选择 Java 路径");
        }
        if self.game_directory.trim().is_empty() {
            anyhow::bail!("请先在「设置」页面选择游戏目录");
        }
        if self.max_memory_mb < 512 {
            anyhow::bail!("内存分配不能小于 512MB（当前: {}MB）", self.max_memory_mb);
        }
        if self.resolution_width == 0 || self.resolution_height == 0 {
            anyhow::bail!(
                "分辨率不能为 0（当前: {}x{}）",
                self.resolution_width,
                self.resolution_height
            );
        }
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

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    fn temp_dir() -> (tempfile::TempDir, PathBuf) {
        let dir = tempfile::tempdir().expect("tempdir");
        let data_dir = dir.path().join("data");
        (dir, data_dir)
    }

    #[test]
    fn load_or_default_creates_default_when_missing() {
        let (_tmp, data_dir) = temp_dir();
        let settings = GameSettings::load_or_default(&data_dir).unwrap();

        assert_eq!(settings.max_memory_mb, 4096);
        assert_eq!(settings.resolution_width, 1280);
        assert_eq!(settings.resolution_height, 720);
        assert!(!settings.fullscreen);
        assert!(!settings.show_game_log);
        assert!(settings.java_path.is_empty());
        // game_directory should default to <data_dir>/minecraft
        let expected_dir = data_dir.join("minecraft");
        assert_eq!(
            settings.game_directory,
            expected_dir.to_string_lossy().to_string()
        );

        // The file should now exist.
        let path = data_dir.join("game_settings.toml");
        assert!(path.exists(), "game_settings.toml should have been created");
    }

    #[test]
    fn load_or_default_reads_existing_file() {
        let (_tmp, data_dir) = temp_dir();
        std::fs::create_dir_all(&data_dir).unwrap();
        let path = data_dir.join("game_settings.toml");
        let content = r#"
java_path = "/usr/bin/java"
max_memory_mb = 8192
jvm_args = "-XX:+UseG1GC"
game_directory = "/games/mc"
resolution_width = 1920
resolution_height = 1080
fullscreen = true
show_game_log = true
"#;
        std::fs::write(&path, content).unwrap();

        let settings = GameSettings::load_or_default(&data_dir).unwrap();
        assert_eq!(settings.java_path, "/usr/bin/java");
        assert_eq!(settings.max_memory_mb, 8192);
        assert_eq!(settings.jvm_args, "-XX:+UseG1GC");
        assert_eq!(settings.game_directory, "/games/mc");
        assert_eq!(settings.resolution_width, 1920);
        assert_eq!(settings.resolution_height, 1080);
        assert!(settings.fullscreen);
        assert!(settings.show_game_log);
    }

    #[test]
    fn load_or_default_errors_on_broken_file() {
        let (_tmp, data_dir) = temp_dir();
        std::fs::create_dir_all(&data_dir).unwrap();
        let path = data_dir.join("game_settings.toml");
        std::fs::write(&path, "this is not valid toml {{{").unwrap();

        let result = GameSettings::load_or_default(&data_dir);
        assert!(result.is_err(), "broken toml should error");
    }

    #[test]
    fn validate_rejects_empty_java_path() {
        let s = GameSettings {
            java_path: String::new(),
            game_directory: "/games/mc".to_string(),
            max_memory_mb: 2048,
            resolution_width: 1280,
            resolution_height: 720,
            ..GameSettings::default()
        };
        assert!(s.validate().is_err());
    }

    #[test]
    fn validate_rejects_empty_game_directory() {
        let s = GameSettings {
            java_path: "/usr/bin/java".to_string(),
            game_directory: String::new(),
            max_memory_mb: 2048,
            resolution_width: 1280,
            resolution_height: 720,
            ..GameSettings::default()
        };
        assert!(s.validate().is_err());
    }

    #[test]
    fn validate_rejects_low_memory() {
        let s = GameSettings {
            java_path: "/usr/bin/java".to_string(),
            game_directory: "/games/mc".to_string(),
            max_memory_mb: 256,
            resolution_width: 1280,
            resolution_height: 720,
            ..GameSettings::default()
        };
        assert!(s.validate().is_err());
    }

    #[test]
    fn validate_rejects_zero_resolution() {
        let mut s = GameSettings {
            java_path: "/usr/bin/java".to_string(),
            game_directory: "/games/mc".to_string(),
            max_memory_mb: 2048,
            resolution_width: 0,
            resolution_height: 720,
            ..GameSettings::default()
        };
        assert!(s.validate().is_err());

        s.resolution_width = 1280;
        s.resolution_height = 0;
        assert!(s.validate().is_err());
    }

    #[test]
    fn validate_passes_for_valid_settings() {
        let s = GameSettings {
            java_path: "/usr/bin/java".to_string(),
            game_directory: "/games/mc".to_string(),
            max_memory_mb: 2048,
            resolution_width: 1280,
            resolution_height: 720,
            ..GameSettings::default()
        };
        assert!(s.validate().is_ok());
    }

    #[test]
    fn save_creates_parent_directory() {
        let (_tmp, data_dir) = temp_dir();
        let nested = data_dir.join("nested").join("deeper");
        let settings = GameSettings::default();
        settings.save(&nested).unwrap();
        assert!(nested.join("game_settings.toml").exists());
    }
}
