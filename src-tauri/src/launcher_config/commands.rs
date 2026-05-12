use super::models::LauncherConfig;
use crate::error::LauncherError;
use crate::settings::GameSettings;
use crate::AppState;
use std::path::Path;
use std::sync::Arc;
use tauri::State;
use tokio::sync::Mutex;
use tracing::info;

fn config_path(data_dir: &Path) -> std::path::PathBuf {
    data_dir.join("launcher_config.toml")
}

fn read_config_toml(data_dir: &Path) -> Result<LauncherConfig, LauncherError> {
    let path = config_path(data_dir);
    let contents = std::fs::read_to_string(&path)
        .map_err(|e| LauncherError::from(format!("读取 launcher_config.toml 失败: {e}")))?;
    toml::from_str(&contents)
        .map_err(|e| LauncherError::from(format!("解析 launcher_config.toml 失败: {e}")))
}

fn write_config_toml(data_dir: &Path, config: &LauncherConfig) -> Result<(), LauncherError> {
    let path = config_path(data_dir);
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)
            .map_err(|e| LauncherError::from(format!("创建配置目录失败: {e}")))?;
    }
    let contents = toml::to_string_pretty(config)
        .map_err(|e| LauncherError::from(format!("序列化 launcher_config 失败: {e}")))?;
    std::fs::write(&path, contents)
        .map_err(|e| LauncherError::from(format!("写入 launcher_config.toml 失败: {e}")))?;
    Ok(())
}

fn init_default_config(data_dir: &Path) -> Result<LauncherConfig, LauncherError> {
    let game = GameSettings::load_or_default(data_dir)
        .map_err(|e| LauncherError::from(format!("加载/创建默认游戏设置失败: {e}")))?;
    let config = LauncherConfig::with_game_settings(game);
    write_config_toml(data_dir, &config)?;
    info!(
        path = %config_path(data_dir).display(),
        "created default launcher config"
    );
    Ok(config)
}

#[tauri::command]
pub async fn retrieve_launcher_config(
    state: State<'_, Arc<Mutex<AppState>>>,
) -> Result<LauncherConfig, LauncherError> {
    let data_dir = {
        let s = state.lock().await;
        s.data_dir.clone()
    };

    let path = config_path(&data_dir);
    if path.exists() {
        read_config_toml(&data_dir)
    } else {
        init_default_config(&data_dir)
    }
}

#[tauri::command]
pub async fn update_launcher_config(
    state: State<'_, Arc<Mutex<AppState>>>,
    config: LauncherConfig,
) -> Result<(), LauncherError> {
    config.validate()?;

    let data_dir = {
        let s = state.lock().await;
        s.data_dir.clone()
    };


    write_config_toml(&data_dir, &config)?;
    info!(
        path = %config_path(&data_dir).display(),
        "saved launcher config"
    );


    config
        .game
        .save(&data_dir)
        .map_err(|e| format!("同步 game_settings.toml 失败: {e}"))?;

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::launcher_config::models::CloseBehavior;
    use std::path::PathBuf;

    fn temp_data_dir() -> (tempfile::TempDir, PathBuf) {
        let dir = tempfile::tempdir().expect("tempdir");
        let data_dir = dir.path().join("data");
        (dir, data_dir)
    }

    fn default_config() -> LauncherConfig {
        let game = GameSettings {
            java_path: "/usr/bin/java".to_string(),
            game_directory: "/tmp/mc".to_string(),
            ..GameSettings::default()
        };
        LauncherConfig::with_game_settings(game)
    }

    #[test]
    fn default_values_match_contract() {
        let c = LauncherConfig::with_game_settings(GameSettings::default());
        assert_eq!(c.basic.language, "zh-CN");
        assert_eq!(c.basic.theme, "system");
        assert_eq!(c.basic.close_behavior, CloseBehavior::Ask);
        assert_eq!(c.basic.download_threads, 4);
        assert!(c.java.auto_scan);
        assert!(c.java.auto_select);
        assert!(c.java.preferred_java_path.is_none());
        assert!(c.advanced.enable_process_monitor);
        assert!(c.advanced.enable_crash_report);
        assert!(c.advanced.keep_launcher_open);
    }

    #[test]
    fn retrieve_creates_files_when_missing() {
        let (_tmp, data_dir) = temp_data_dir();
        let path = config_path(&data_dir);
        let gs_path = data_dir.join("game_settings.toml");

        assert!(!path.exists());
        assert!(!gs_path.exists());

        let config = init_default_config(&data_dir).unwrap();

        assert!(path.exists(), "launcher_config.toml should be created");
        assert!(
            gs_path.exists(),
            "game_settings.toml should be created by load_or_default"
        );

        assert_eq!(config.basic.language, "zh-CN");
        assert_eq!(config.basic.theme, "system");
    }

    #[test]
    fn retrieve_reads_existing_file() {
        let (_tmp, data_dir) = temp_data_dir();
        let mut config = default_config();
        config.basic.language = "en-US".to_string();
        config.basic.theme = "dark".to_string();
        config.basic.download_threads = 8;
        write_config_toml(&data_dir, &config).unwrap();

        let loaded = read_config_toml(&data_dir).unwrap();
        assert_eq!(loaded.basic.language, "en-US");
        assert_eq!(loaded.basic.theme, "dark");
        assert_eq!(loaded.basic.download_threads, 8);
    }

    #[test]
    fn broken_toml_returns_error() {
        let (_tmp, data_dir) = temp_data_dir();
        let path = config_path(&data_dir);
        std::fs::create_dir_all(&data_dir).unwrap();
        std::fs::write(&path, "this is not valid {{{ toml").unwrap();

        let result = read_config_toml(&data_dir);
        assert!(result.is_err(), "broken toml should error");
    }

    #[test]
    fn update_saves_both_config_and_game_settings() {
        let (_tmp, data_dir) = temp_data_dir();


        let gs_path = data_dir.join("game_settings.toml");
        let initial_gs = GameSettings {
            game_directory: "/tmp/mc".to_string(),
            ..GameSettings::default()
        };
        initial_gs.save(&data_dir).unwrap();
        assert!(gs_path.exists());


        let mut config = LauncherConfig::with_game_settings(GameSettings {
            java_path: "/usr/bin/java".to_string(),
            game_directory: "/games/mc".to_string(),
            max_memory_mb: 8192,
            resolution_width: 1920,
            resolution_height: 1080,
            ..GameSettings::default()
        });
        config.basic.language = "en-US".to_string();
        config.validate().unwrap();


        write_config_toml(&data_dir, &config).unwrap();
        config
            .game
            .save(&data_dir)
            .map_err(|e| format!("{e}"))
            .unwrap();


        let gs = GameSettings::load(&data_dir).unwrap();
        assert_eq!(gs.max_memory_mb, 8192);


        let lc = read_config_toml(&data_dir).unwrap();
        assert_eq!(lc.basic.language, "en-US");
    }

    #[test]
    fn validate_download_threads_zero_rejected() {
        let mut config = default_config();
        config.basic.download_threads = 0;
        let err = config.validate().unwrap_err();
        assert!(
            err.contains("下载线程数"),
            "expected download threads error, got: {err}"
        );
    }

    #[test]
    fn validate_download_threads_exceeds_max_rejected() {
        let mut config = default_config();
        config.basic.download_threads = 17;
        let err = config.validate().unwrap_err();
        assert!(
            err.contains("下载线程数"),
            "expected download threads error, got: {err}"
        );
    }

    #[test]
    fn validate_illegal_theme_rejected() {
        let mut config = default_config();
        config.basic.theme = "blue".to_string();
        let err = config.validate().unwrap_err();
        assert!(err.contains("主题"), "expected theme error, got: {err}");
    }

    #[test]
    fn validate_empty_language_rejected() {
        let mut config = default_config();
        config.basic.language = String::new();
        let err = config.validate().unwrap_err();
        assert!(err.contains("语言"), "expected language error, got: {err}");
    }

    #[test]
    fn validate_game_settings_failure_propagates() {
        let mut config = default_config();

        config.game.java_path = String::new();
        config.game.game_directory = String::new();
        let err = config.validate().unwrap_err();
        assert!(
            err.contains("游戏设置校验失败"),
            "expected game settings validation error, got: {err}"
        );
    }

    #[test]
    fn validate_valid_config_passes() {
        let config = LauncherConfig::with_game_settings(GameSettings {
            java_path: "/usr/bin/java".to_string(),
            game_directory: "/games/mc".to_string(),
            max_memory_mb: 4096,
            resolution_width: 1280,
            resolution_height: 720,
            ..GameSettings::default()
        });
        assert!(config.validate().is_ok());
    }

    #[test]
    fn theme_variants_accepted() {
        for theme in &["system", "light", "dark"] {
            let mut config = default_config();
            config.basic.theme = theme.to_string();
            assert!(config.validate().is_ok(), "theme {theme} should be valid");
        }
    }

    #[test]
    fn download_threads_boundary_values() {
        let mut config = default_config();
        config.basic.download_threads = 1;
        assert!(config.validate().is_ok());
        config.basic.download_threads = 16;
        assert!(config.validate().is_ok());
    }
}
