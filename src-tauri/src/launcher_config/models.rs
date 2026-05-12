use crate::settings::GameSettings;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LauncherConfig {
    pub basic: LauncherBasicConfig,
    pub game: GameSettings,
    pub java: JavaConfig,
    pub advanced: AdvancedConfig,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LauncherBasicConfig {
    pub language: String,
    pub theme: String,
    pub close_behavior: CloseBehavior,
    pub download_threads: u8,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum CloseBehavior {
    Ask,
    MinimizeToTray,
    Exit,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct JavaConfig {
    pub auto_scan: bool,
    pub auto_select: bool,
    pub preferred_java_path: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AdvancedConfig {
    pub enable_process_monitor: bool,
    pub enable_crash_report: bool,
    pub keep_launcher_open: bool,
}

impl Default for LauncherBasicConfig {
    fn default() -> Self {
        Self {
            language: "zh-CN".to_string(),
            theme: "system".to_string(),
            close_behavior: CloseBehavior::Ask,
            download_threads: 4,
        }
    }
}

impl Default for JavaConfig {
    fn default() -> Self {
        Self {
            auto_scan: true,
            auto_select: true,
            preferred_java_path: None,
        }
    }
}

impl Default for AdvancedConfig {
    fn default() -> Self {
        Self {
            enable_process_monitor: true,
            enable_crash_report: true,
            keep_launcher_open: true,
        }
    }
}

impl LauncherConfig {
    pub fn with_game_settings(game: GameSettings) -> Self {
        Self {
            basic: LauncherBasicConfig::default(),
            game,
            java: JavaConfig::default(),
            advanced: AdvancedConfig::default(),
        }
    }

    pub fn validate(&self) -> Result<(), crate::error::LauncherError> {
        if self.basic.language.trim().is_empty() {
            return Err(crate::error::LauncherError::new(
                "VALIDATION_ERROR",
                "语言设置不能为空",
            ));
        }

        let theme = self.basic.theme.as_str();
        if theme != "system" && theme != "light" && theme != "dark" {
            return Err(crate::error::LauncherError::new(
                "VALIDATION_ERROR",
                format!("不支持的界面主题: {theme}（允许: system, light, dark）"),
            ));
        }

        if self.basic.download_threads == 0 || self.basic.download_threads > 16 {
            return Err(crate::error::LauncherError::new(
                "VALIDATION_ERROR",
                format!(
                    "下载线程数必须在 1–16 之间（当前: {}）",
                    self.basic.download_threads
                ),
            ));
        }

        self.game.validate().map_err(|e| {
            crate::error::LauncherError::new("VALIDATION_ERROR", format!("游戏设置校验失败: {e}"))
        })?;

        Ok(())
    }
}
