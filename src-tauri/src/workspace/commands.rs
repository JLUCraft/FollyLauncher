use crate::error::LauncherError;
use crate::instance::commands::get_instance_in;
use crate::AppState;
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};
use std::sync::Arc;
use tauri::State;
use tokio::sync::Mutex;



#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorldInfo {
    pub name: String,
    pub icon_path: Option<String>,
    pub last_played: Option<u64>,
    pub game_mode: String,
    pub cheats: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ResourcePackInfo {
    pub name: String,
    pub path: String,
    pub enabled: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ScreenshotInfo {
    pub name: String,
    pub path: String,
    pub timestamp: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ServerInfo {
    pub name: String,
    pub address: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModFileInfo {
    pub name: String,
    pub path: String,
    pub file_name: String,
    pub enabled: bool,
    pub size: u64,
    pub modified_at: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InstanceWorkspaceInfo {
    pub instance_id: String,
    pub game_dir: String,
    pub mods: Vec<ModFileInfo>,
    pub resource_packs: Vec<ResourcePackInfo>,
    pub shader_packs: Vec<ResourcePackInfo>,
    pub worlds: Vec<WorldInfo>,
    pub screenshots: Vec<ScreenshotInfo>,
    pub servers: Vec<ServerInfo>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModOperationResult {
    pub instance_id: String,
    pub old_path: String,
    pub new_path: Option<String>,
    pub file_name: String,
    pub enabled: Option<bool>,
    pub deleted: bool,
}



fn get_instance_dir(instance_id: &str) -> Result<PathBuf, LauncherError> {
    let data_dir = crate::project_data_dir()
        .map_err(|e| LauncherError::from(format!("无法确定项目数据目录: {e}")))?;
    Ok(data_dir
        .join("instances")
        .join(instance_id)
        .join(".minecraft"))
}



fn read_dir_names(dir: &Path) -> Vec<(String, PathBuf)> {
    let mut items = Vec::new();
    if let Ok(entries) = std::fs::read_dir(dir) {
        for entry in entries.flatten() {
            let path = entry.path();
            let name = path
                .file_name()
                .and_then(|n| n.to_str())
                .unwrap_or("unknown")
                .to_string();
            items.push((name, path));
        }
    }
    items
}

fn file_modified_unix(path: &Path) -> u64 {
    std::fs::metadata(path)
        .ok()
        .and_then(|m| m.modified().ok())
        .and_then(|t| t.duration_since(std::time::UNIX_EPOCH).ok())
        .map(|d| d.as_secs())
        .unwrap_or(0)
}

fn file_size(path: &Path) -> u64 {
    std::fs::metadata(path).map(|m| m.len()).unwrap_or(0)
}















pub fn parse_mod_name(raw_name: &str) -> String {
    let lower = raw_name.to_lowercase();
    let mut result = raw_name.to_string();

    if lower.ends_with(".disabled") {
        let trim_len = ".disabled".len();
        result.truncate(result.len() - trim_len);
    }

    let lower_result = result.to_lowercase();
    if lower_result.ends_with(".jar") {
        let trim_len = ".jar".len();
        result.truncate(result.len() - trim_len);
    }

    result
}



pub fn scan_worlds(game_dir: &Path) -> Vec<WorldInfo> {
    let saves_dir = game_dir.join("saves");
    let mut worlds = Vec::new();
    for (name, path) in read_dir_names(&saves_dir) {
        if path.is_dir() {
            let icon = path.join("icon.png");
            worlds.push(WorldInfo {
                name,
                icon_path: icon.exists().then(|| icon.to_string_lossy().to_string()),
                last_played: None,
                game_mode: "unknown".to_string(),
                cheats: false,
            });
        }
    }
    worlds
}

pub fn scan_screenshots(game_dir: &Path) -> Vec<ScreenshotInfo> {
    let screenshots_dir = game_dir.join("screenshots");
    let mut screenshots = Vec::new();
    for (name, path) in read_dir_names(&screenshots_dir) {
        if path.extension() == Some(std::ffi::OsStr::new("png")) {
            let timestamp = file_modified_unix(&path);
            screenshots.push(ScreenshotInfo {
                name,
                path: path.to_string_lossy().to_string(),
                timestamp,
            });
        }
    }
    screenshots.sort_by_key(|b| std::cmp::Reverse(b.timestamp));
    screenshots
}

pub fn scan_resource_packs(game_dir: &Path) -> Vec<ResourcePackInfo> {
    let resourcepacks_dir = game_dir.join("resourcepacks");
    read_dir_names(&resourcepacks_dir)
        .into_iter()
        .map(|(name, path)| ResourcePackInfo {
            name,
            path: path.to_string_lossy().to_string(),
            enabled: false,
        })
        .collect()
}

pub fn scan_shader_packs(game_dir: &Path) -> Vec<ResourcePackInfo> {
    let shaderpacks_dir = game_dir.join("shaderpacks");
    read_dir_names(&shaderpacks_dir)
        .into_iter()
        .map(|(name, path)| ResourcePackInfo {
            name,
            path: path.to_string_lossy().to_string(),
            enabled: false,
        })
        .collect()
}

pub fn scan_servers(game_dir: &Path) -> Vec<ServerInfo> {
    let servers_dat = game_dir.join("servers.dat");
    let mut servers = Vec::new();
    if servers_dat.exists() {
        match parse_servers_dat(&servers_dat) {
            Ok(list) => servers = list,
            Err(e) => {
                tracing::warn!(error = %e, "failed to parse servers.dat");
            }
        }
    }
    servers
}

pub fn scan_mods(game_dir: &Path) -> Vec<ModFileInfo> {
    let mods_dir = game_dir.join("mods");
    let mut mods = Vec::new();
    if let Ok(entries) = std::fs::read_dir(&mods_dir) {
        for entry in entries.flatten() {
            let path = entry.path();
            if path.is_dir() {
                continue;
            }
            let file_name = path
                .file_name()
                .and_then(|n| n.to_str())
                .unwrap_or("unknown")
                .to_string();

            let ext = path
                .extension()
                .and_then(|e| e.to_str())
                .unwrap_or("")
                .to_lowercase();


            if ext != "jar" && ext != "disabled" {
                continue;
            }

            let enabled = ext == "jar";
            let name = parse_mod_name(&file_name);
            let size = file_size(&path);
            let modified_at = i64::try_from(file_modified_unix(&path)).unwrap_or(0);

            mods.push(ModFileInfo {
                name,
                path: path.to_string_lossy().to_string(),
                file_name,
                enabled,
                size,
                modified_at,
            });
        }
    }
    mods.sort_by(|a, b| a.name.cmp(&b.name));
    mods
}



#[tauri::command]
pub async fn retrieve_world_list(instance_id: String) -> Result<Vec<WorldInfo>, LauncherError> {
    Ok(scan_worlds(&get_instance_dir(&instance_id)?))
}

#[tauri::command]
pub async fn retrieve_screenshot_list(
    instance_id: String,
) -> Result<Vec<ScreenshotInfo>, LauncherError> {
    Ok(scan_screenshots(&get_instance_dir(&instance_id)?))
}

#[tauri::command]
pub async fn retrieve_resource_pack_list(
    instance_id: String,
) -> Result<Vec<ResourcePackInfo>, LauncherError> {
    Ok(scan_resource_packs(&get_instance_dir(&instance_id)?))
}

#[tauri::command]
pub async fn retrieve_shader_pack_list(
    instance_id: String,
) -> Result<Vec<ResourcePackInfo>, LauncherError> {
    Ok(scan_shader_packs(&get_instance_dir(&instance_id)?))
}







#[tauri::command]
pub async fn retrieve_instance_workspace(
    instance_id: String,
    state: State<'_, Arc<Mutex<AppState>>>,
) -> Result<InstanceWorkspaceInfo, LauncherError> {
    let data_dir = {
        let s = state.lock().await;
        s.data_dir.clone()
    };
    let instance = get_instance_in(&data_dir, &instance_id)?;
    let game_dir = PathBuf::from(&instance.game_dir);

    let mods = scan_mods(&game_dir);
    let resource_packs = scan_resource_packs(&game_dir);
    let shader_packs = scan_shader_packs(&game_dir);
    let worlds = scan_worlds(&game_dir);
    let screenshots = scan_screenshots(&game_dir);
    let servers = scan_servers(&game_dir);

    Ok(InstanceWorkspaceInfo {
        instance_id,
        game_dir: instance.game_dir,
        mods,
        resource_packs,
        shader_packs,
        worlds,
        screenshots,
        servers,
    })
}



fn sanitize_file_name(name: &str) -> Result<(), LauncherError> {
    if name.is_empty() {
        return Err(LauncherError::from("文件名不能为空"));
    }
    if name.contains('\0') {
        return Err(LauncherError::from("文件名包含非法字符 NUL"));
    }
    if name.contains('/') || name.contains('\\') {
        return Err(LauncherError::from("文件名包含路径分隔符"));
    }

    if name == "." || name == ".." {
        return Err(LauncherError::from("文件名不能为 '.' 或 '..'"));
    }
    let path = std::path::Path::new(name);
    if path.is_absolute() {
        return Err(LauncherError::from("文件名不能为绝对路径"));
    }

    let bytes = name.as_bytes();
    if bytes.len() >= 2 && bytes[0].is_ascii_alphabetic() && bytes[1] == b':' {
        return Err(LauncherError::from("文件名不能包含盘符"));
    }
    Ok(())
}



fn target_for_enable(file_name: &str) -> Result<Option<String>, LauncherError> {
    let lower = file_name.to_lowercase();
    if lower.ends_with(".disabled") {
        let new_name = &file_name[..file_name.len() - ".disabled".len()];
        return Ok(Some(new_name.to_string()));
    }
    if lower.ends_with(".jar") {
        return Ok(None);
    }
    Err(LauncherError::new(
        "INVALID_INPUT",
        format!(
            "无法启用文件 '{}'：仅支持 .jar 或 .disabled 文件",
            file_name
        ),
    ))
}



fn target_for_disable(file_name: &str) -> Result<Option<String>, LauncherError> {
    let lower = file_name.to_lowercase();
    if lower.ends_with(".jar") {
        let new_name = format!("{}.disabled", file_name);
        return Ok(Some(new_name));
    }
    if lower.ends_with(".disabled") {
        return Ok(None);
    }
    Err(LauncherError::new(
        "INVALID_INPUT",
        format!(
            "无法禁用文件 '{}'：仅支持 .jar 或 .disabled 文件",
            file_name
        ),
    ))
}

#[tauri::command]
pub async fn set_mod_enabled(
    instance_id: String,
    file_name: String,
    enabled: bool,
    state: State<'_, Arc<Mutex<AppState>>>,
) -> Result<ModOperationResult, LauncherError> {
    sanitize_file_name(&file_name)?;

    let data_dir = {
        let s = state.lock().await;
        s.data_dir.clone()
    };
    let instance = get_instance_in(&data_dir, &instance_id)?;
    let game_dir = PathBuf::from(&instance.game_dir);
    let mods_dir = game_dir.join("mods");

    let src_path = mods_dir.join(&file_name);

    if !src_path.exists() {
        return Err(LauncherError::new(
            "NOT_FOUND",
            format!("Mod 文件不存在: {}", src_path.display()),
        ));
    }
    if !src_path.is_file() {
        return Err(LauncherError::new(
            "INVALID_INPUT",
            format!("路径不是文件: {}", src_path.display()),
        ));
    }

    let target_name_opt = if enabled {
        target_for_enable(&file_name)?
    } else {
        target_for_disable(&file_name)?
    };

    let (new_path, new_enabled) = match target_name_opt {
        Some(target_name) => {
            let dst_path = mods_dir.join(&target_name);
            if dst_path.exists() {
                return Err(LauncherError::new(
                    "CONFLICT",
                    format!("目标文件已存在: {}", dst_path.display()),
                ));
            }
            std::fs::rename(&src_path, &dst_path).map_err(|e| format!("重命名文件失败: {}", e))?;
            (Some(dst_path.to_string_lossy().to_string()), enabled)
        }
        None => {

            (None, enabled)
        }
    };

    Ok(ModOperationResult {
        instance_id,
        old_path: src_path.to_string_lossy().to_string(),
        new_path,
        file_name,
        enabled: Some(new_enabled),
        deleted: false,
    })
}

#[tauri::command]
pub async fn delete_mod_file(
    instance_id: String,
    file_name: String,
    state: State<'_, Arc<Mutex<AppState>>>,
) -> Result<ModOperationResult, LauncherError> {
    sanitize_file_name(&file_name)?;

    let data_dir = {
        let s = state.lock().await;
        s.data_dir.clone()
    };
    let instance = get_instance_in(&data_dir, &instance_id)?;
    let game_dir = PathBuf::from(&instance.game_dir);
    let mods_dir = game_dir.join("mods");

    let src_path = mods_dir.join(&file_name);

    if !src_path.exists() {
        return Err(LauncherError::new(
            "NOT_FOUND",
            format!("Mod 文件不存在: {}", src_path.display()),
        ));
    }
    if src_path.is_dir() {
        return Err(LauncherError::new(
            "INVALID_INPUT",
            format!("不能删除目录: {}", src_path.display()),
        ));
    }

    let ext = src_path
        .extension()
        .and_then(|e| e.to_str())
        .unwrap_or("")
        .to_lowercase();

    if ext != "jar" && ext != "disabled" {
        return Err(LauncherError::new(
            "INVALID_INPUT",
            format!(
                "仅支持删除 .jar 或 .disabled 文件，当前文件: {}",
                src_path.display()
            ),
        ));
    }

    std::fs::remove_file(&src_path).map_err(|e| format!("删除文件失败: {}", e))?;

    Ok(ModOperationResult {
        instance_id,
        old_path: src_path.to_string_lossy().to_string(),
        new_path: None,
        file_name,
        enabled: None,
        deleted: true,
    })
}



fn parse_servers_dat(path: &Path) -> Result<Vec<ServerInfo>, LauncherError> {
    let data = std::fs::read(path)
        .map_err(|e| LauncherError::from(format!("无法读取 servers.dat: {}", e)))?;
    let bytes: &[u8] = data.as_ref();
    let mut cursor = std::io::Cursor::new(bytes);

    let root_type = read_u8(&mut cursor)?;
    if root_type != 0x0a {
        return Err(LauncherError::new(
            "FORMAT_ERROR",
            format!(
                "servers.dat 格式错误: 期望 Compound (0x0a), 得到 {:#x}",
                root_type
            ),
        ));
    }
    read_nbt_string(&mut cursor)?;

    let tag_type = read_u8(&mut cursor)?;
    if tag_type != 0x09 {
        return Err(LauncherError::new(
            "FORMAT_ERROR",
            format!("期望 List tag, 得到 {:#x}", tag_type),
        ));
    }
    read_nbt_string(&mut cursor)?;
    let list_type = read_u8(&mut cursor)?;
    let list_len = read_i32_be(&mut cursor)?;
    if list_type != 0x0a {
        return Ok(Vec::new());
    }

    let mut servers = Vec::new();
    for _ in 0..list_len {
        let compound_type = read_u8(&mut cursor)?;
        if compound_type != 0x0a {
            break;
        }
        let mut name = String::new();
        let mut ip = String::new();

        loop {
            let field_type = read_u8(&mut cursor);
            if field_type.is_err() || field_type.as_ref() == Ok(&0x00) {
                break;
            }
            let field_name = read_nbt_string(&mut cursor)?;
            let ft =
                field_type.map_err(|e| LauncherError::from(format!("读取标签类型失败: {}", e)))?;
            match ft {
                0x08 => {
                    let value = read_nbt_string(&mut cursor)?;
                    match field_name.as_str() {
                        "name" => name = value,
                        "ip" => ip = value,
                        _ => {}
                    }
                }
                _ => skip_nbt_value(&mut cursor, ft)?,
            }
        }

        if !name.is_empty() && !ip.is_empty() {
            servers.push(ServerInfo { name, address: ip });
        }
    }

    Ok(servers)
}

fn read_u8(cursor: &mut std::io::Cursor<&[u8]>) -> Result<u8, LauncherError> {
    use std::io::Read;
    let mut buf = [0u8; 1];
    cursor
        .read_exact(&mut buf)
        .map_err(|e| LauncherError::from(format!("读取字节失败: {}", e)))?;
    Ok(buf[0])
}

fn read_i32_be(cursor: &mut std::io::Cursor<&[u8]>) -> Result<i32, LauncherError> {
    use std::io::Read;
    let mut buf = [0u8; 4];
    cursor
        .read_exact(&mut buf)
        .map_err(|e| LauncherError::from(format!("读取 i32 失败: {}", e)))?;
    Ok(i32::from_be_bytes(buf))
}

fn read_u16_be(src: &mut std::io::Cursor<&[u8]>) -> Result<u16, LauncherError> {
    use std::io::Read;
    let mut buf = [0u8; 2];
    src.read_exact(&mut buf)
        .map_err(|e| LauncherError::from(format!("读取 u16 失败: {}", e)))?;
    Ok(u16::from_be_bytes(buf))
}

fn read_nbt_string(cursor: &mut std::io::Cursor<&[u8]>) -> Result<String, LauncherError> {
    let len = read_u16_be(cursor)? as usize;
    use std::io::Read;
    let mut buf = vec![0u8; len];
    cursor
        .read_exact(&mut buf)
        .map_err(|e| LauncherError::from(format!("读取 NBT 字符串失败: {}", e)))?;
    String::from_utf8(buf).map_err(|e| LauncherError::from(format!("NBT 字符串 UTF-8 无效: {}", e)))
}

fn skip_nbt_value(cursor: &mut std::io::Cursor<&[u8]>, tag_type: u8) -> Result<(), LauncherError> {
    use std::io::{Read, Seek};
    match tag_type {
        0x01 => {
            cursor
                .read_exact(&mut [0u8; 1])
                .map_err(|e| LauncherError::from(e.to_string()))?;
        }
        0x02 => {
            cursor
                .read_exact(&mut [0u8; 2])
                .map_err(|e| LauncherError::from(e.to_string()))?;
        }
        0x03 => {
            cursor
                .read_exact(&mut [0u8; 4])
                .map_err(|e| LauncherError::from(e.to_string()))?;
        }
        0x04 => {
            cursor
                .read_exact(&mut [0u8; 8])
                .map_err(|e| LauncherError::from(e.to_string()))?;
        }
        0x05 => {
            cursor
                .read_exact(&mut [0u8; 4])
                .map_err(|e| LauncherError::from(e.to_string()))?;
        }
        0x06 => {
            cursor
                .read_exact(&mut [0u8; 8])
                .map_err(|e| LauncherError::from(e.to_string()))?;
        }
        0x07 => {
            let len = read_i32_be(cursor)?;
            cursor
                .seek_relative(len as i64 * 4)
                .map_err(|e| LauncherError::from(e.to_string()))?;
        }
        0x08 => {
            read_nbt_string(cursor)?;
        }
        0x09 => {
            let el_type = read_u8(cursor)?;
            let count = read_i32_be(cursor)?;
            for _ in 0..count {
                skip_nbt_value(cursor, el_type)?;
            }
        }
        0x0a => loop {
            let ft = read_u8(cursor)?;
            if ft == 0x00 {
                break;
            }
            read_nbt_string(cursor)?;
            skip_nbt_value(cursor, ft)?;
        },
        0x0b => {
            let len = read_i32_be(cursor)?;
            cursor
                .seek_relative(len as i64 * 4)
                .map_err(|e| LauncherError::from(e.to_string()))?;
        }
        0x0c => {
            let len = read_i32_be(cursor)?;
            cursor
                .seek_relative(len as i64 * 8)
                .map_err(|e| LauncherError::from(e.to_string()))?;
        }
        _ => {
            return Err(LauncherError::new(
                "FORMAT_ERROR",
                format!("unknown NBT tag type: {:#x}", tag_type),
            ))
        }
    }
    Ok(())
}



#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;



    #[test]
    fn parse_jar_name() {
        assert_eq!(parse_mod_name("foo.jar"), "foo");
    }

    #[test]
    fn parse_jar_disabled_name() {
        assert_eq!(parse_mod_name("foo.jar.disabled"), "foo");
    }

    #[test]
    fn parse_disabled_name() {
        assert_eq!(parse_mod_name("foo.disabled"), "foo");
    }

    #[test]
    fn parse_uppercase_jar() {
        assert_eq!(parse_mod_name("OptiFine.JAR"), "OptiFine");
    }

    #[test]
    fn parse_mixed_case_disabled() {
        assert_eq!(parse_mod_name("mod.Jar.Disabled"), "mod");
    }

    #[test]
    fn parse_no_extension_stays() {
        assert_eq!(parse_mod_name("readme"), "readme");
    }

    #[test]
    fn parse_double_jar_no_disabled() {

        assert_eq!(parse_mod_name("foo.jar.jar"), "foo.jar");
    }

    #[test]
    fn parse_name_with_dots() {
        assert_eq!(parse_mod_name("my.mod-1.0.jar"), "my.mod-1.0");
    }



    #[test]
    fn scan_mods_only_includes_jar_and_disabled() {
        let dir = TempDir::new().expect("tempdir");
        let game_dir = dir.path();
        let mods_dir = game_dir.join("mods");
        std::fs::create_dir_all(&mods_dir).expect("create mods dir");


        std::fs::write(mods_dir.join("alpha.jar"), b"aaa").expect("write");
        std::fs::write(mods_dir.join("beta.jar.disabled"), b"bbb").expect("write");
        std::fs::write(mods_dir.join("gamma.disabled"), b"ggg").expect("write");

        std::fs::write(mods_dir.join("readme.txt"), b"rrr").expect("write");
        std::fs::write(mods_dir.join("config.json"), b"ccc").expect("write");

        std::fs::create_dir_all(mods_dir.join("nested_mod")).expect("create nested dir");

        let mods = scan_mods(game_dir);
        assert_eq!(
            mods.len(),
            3,
            "should only include .jar and .disabled files"
        );

        let names: Vec<&str> = mods.iter().map(|m| m.name.as_str()).collect();
        assert_eq!(names, vec!["alpha", "beta", "gamma"]);


        assert!(mods[0].enabled);
        assert!(!mods[1].enabled);
        assert!(!mods[2].enabled);
    }

    #[test]
    fn scan_mods_sorted_by_name_ascending() {
        let dir = TempDir::new().expect("tempdir");
        let game_dir = dir.path();
        let mods_dir = game_dir.join("mods");
        std::fs::create_dir_all(&mods_dir).expect("create mods dir");

        std::fs::write(mods_dir.join("zebra.jar"), b"z").expect("write");
        std::fs::write(mods_dir.join("alpha.jar"), b"a").expect("write");
        std::fs::write(mods_dir.join("mike.jar"), b"m").expect("write");

        let mods = scan_mods(game_dir);
        let names: Vec<&str> = mods.iter().map(|m| m.name.as_str()).collect();
        assert_eq!(names, vec!["alpha", "mike", "zebra"]);
    }

    #[test]
    fn scan_mods_empty_dir_returns_empty() {
        let dir = TempDir::new().expect("tempdir");
        let game_dir = dir.path();
        let mods_dir = game_dir.join("mods");
        std::fs::create_dir_all(&mods_dir).expect("create mods dir");

        let mods = scan_mods(game_dir);
        assert!(mods.is_empty());
    }

    #[test]
    fn scan_mods_nonexistent_dir_returns_empty() {
        let dir = TempDir::new().expect("tempdir");
        let game_dir = dir.path();


        let mods = scan_mods(game_dir);
        assert!(mods.is_empty());
    }

    #[test]
    fn scan_mods_captures_size_and_modified() {
        let dir = TempDir::new().expect("tempdir");
        let game_dir = dir.path();
        let mods_dir = game_dir.join("mods");
        std::fs::create_dir_all(&mods_dir).expect("create mods dir");

        let content = b"hello mod content here";
        std::fs::write(mods_dir.join("testmod.jar"), content).expect("write");

        let mods = scan_mods(game_dir);
        assert_eq!(mods.len(), 1);
        assert_eq!(mods[0].size, content.len() as u64);
        assert!(mods[0].modified_at > 0);
    }



    #[test]
    fn scan_worlds_finds_directories() {
        let dir = TempDir::new().expect("tempdir");
        let game_dir = dir.path();
        let saves = game_dir.join("saves");
        std::fs::create_dir_all(saves.join("New World")).expect("create world dir");
        std::fs::create_dir_all(saves.join("Creative")).expect("create world dir");

        std::fs::write(saves.join("readme.txt"), b"ignored").expect("write");

        let worlds = scan_worlds(game_dir);
        assert_eq!(worlds.len(), 2);
        let names: Vec<&str> = worlds.iter().map(|w| w.name.as_str()).collect();
        assert!(names.contains(&"New World"));
        assert!(names.contains(&"Creative"));
    }



    #[test]
    fn scan_screenshots_filters_png_only() {
        let dir = TempDir::new().expect("tempdir");
        let game_dir = dir.path();
        let ss = game_dir.join("screenshots");
        std::fs::create_dir_all(&ss).expect("create screenshots dir");
        std::fs::write(ss.join("shot1.png"), b"fake png").expect("write");
        std::fs::write(ss.join("shot2.png"), b"fake png 2").expect("write");
        std::fs::write(ss.join("notes.txt"), b"not an image").expect("write");

        let shots = scan_screenshots(game_dir);
        assert_eq!(shots.len(), 2);
    }



    #[test]
    fn scan_resource_packs_lists_files() {
        let dir = TempDir::new().expect("tempdir");
        let game_dir = dir.path();
        let rp = game_dir.join("resourcepacks");
        std::fs::create_dir_all(&rp).expect("create dir");
        std::fs::write(rp.join("Faithful.zip"), b"pack").expect("write");

        let packs = scan_resource_packs(game_dir);
        assert_eq!(packs.len(), 1);
        assert_eq!(packs[0].name, "Faithful.zip");
    }

    #[test]
    fn scan_shader_packs_lists_files() {
        let dir = TempDir::new().expect("tempdir");
        let game_dir = dir.path();
        let sp = game_dir.join("shaderpacks");
        std::fs::create_dir_all(&sp).expect("create dir");
        std::fs::write(sp.join("SEUS.zip"), b"shader").expect("write");

        let packs = scan_shader_packs(game_dir);
        assert_eq!(packs.len(), 1);
        assert_eq!(packs[0].name, "SEUS.zip");
    }



    #[test]
    fn scan_servers_empty_when_no_dat() {
        let dir = TempDir::new().expect("tempdir");
        assert!(scan_servers(dir.path()).is_empty());
    }



    #[test]
    fn workspace_aggregation_uses_passed_game_dir() {
        let dir = TempDir::new().expect("tempdir");
        let game_dir = dir.path();


        let mods_dir = game_dir.join("mods");
        std::fs::create_dir_all(&mods_dir).expect("create mods dir");
        std::fs::write(mods_dir.join("example.jar"), b"mod data").expect("write");


        let saves_dir = game_dir.join("saves");
        std::fs::create_dir_all(saves_dir.join("World1")).expect("create world");


        let rp_dir = game_dir.join("resourcepacks");
        std::fs::create_dir_all(&rp_dir).expect("create rp dir");
        std::fs::write(rp_dir.join("pack.zip"), b"pack").expect("write");


        let sp_dir = game_dir.join("shaderpacks");
        std::fs::create_dir_all(&sp_dir).expect("create sp dir");
        std::fs::write(sp_dir.join("shader.zip"), b"shader").expect("write");


        let ss_dir = game_dir.join("screenshots");
        std::fs::create_dir_all(&ss_dir).expect("create ss dir");
        std::fs::write(ss_dir.join("screen.png"), b"png").expect("write");


        let mods = scan_mods(game_dir);
        let worlds = scan_worlds(game_dir);
        let rps = scan_resource_packs(game_dir);
        let sps = scan_shader_packs(game_dir);
        let sss = scan_screenshots(game_dir);
        let servers = scan_servers(game_dir);

        assert_eq!(mods.len(), 1);
        assert_eq!(mods[0].name, "example");
        assert!(mods[0].enabled);
        assert_eq!(worlds.len(), 1);
        assert_eq!(rps.len(), 1);
        assert_eq!(sps.len(), 1);
        assert_eq!(sss.len(), 1);
        assert!(servers.is_empty());
    }



    #[test]
    fn sanitize_legal_names() {
        assert!(sanitize_file_name("foo.jar").is_ok());
        assert!(sanitize_file_name("my-mod-1.0.jar").is_ok());
        assert!(sanitize_file_name("OptiFine_1.21.JAR").is_ok());
        assert!(sanitize_file_name("some mod.jar.disabled").is_ok());
        assert!(sanitize_file_name("fabric-api-0.92.0+1.21.jar").is_ok());
    }

    #[test]
    fn sanitize_rejects_empty() {
        assert!(sanitize_file_name("").is_err());
    }

    #[test]
    fn sanitize_rejects_slash() {
        assert!(sanitize_file_name("foo/bar.jar").is_err());
    }

    #[test]
    fn sanitize_rejects_backslash() {
        assert!(sanitize_file_name("foo\\bar.jar").is_err());
    }

    #[test]
    fn sanitize_rejects_dotdot() {
        assert!(sanitize_file_name("..").is_err());
        assert!(sanitize_file_name(".").is_err());

        assert!(sanitize_file_name("foo..bar.jar").is_ok());
        assert!(sanitize_file_name("...").is_ok());
    }

    #[test]
    fn sanitize_rejects_absolute_unix() {
        assert!(sanitize_file_name("/etc/passwd").is_err());
    }

    #[test]
    fn sanitize_rejects_windows_drive() {
        assert!(sanitize_file_name("C:foo.jar").is_err());
        assert!(sanitize_file_name("D:bar.jar").is_err());
    }

    #[test]
    fn sanitize_rejects_nul() {
        assert!(sanitize_file_name("foo\0bar.jar").is_err());
    }



    #[test]
    fn enable_disabled_file() {
        let result = target_for_enable("mod.jar.disabled").unwrap();
        assert_eq!(result, Some("mod.jar".to_string()));
    }

    #[test]
    fn enable_plain_disabled_file() {
        let result = target_for_enable("mod.disabled").unwrap();
        assert_eq!(result, Some("mod".to_string()));
    }

    #[test]
    fn enable_already_enabled_jar() {
        let result = target_for_enable("mod.jar").unwrap();
        assert_eq!(result, None);
    }

    #[test]
    fn enable_rejects_other_extension() {
        assert!(target_for_enable("readme.txt").is_err());
        assert!(target_for_enable("config").is_err());
    }



    #[test]
    fn disable_jar_file() {
        let result = target_for_disable("mod.jar").unwrap();
        assert_eq!(result, Some("mod.jar.disabled".to_string()));
    }

    #[test]
    fn disable_already_disabled_file() {
        let result = target_for_disable("mod.jar.disabled").unwrap();
        assert_eq!(result, None);
        let result2 = target_for_disable("mod.disabled").unwrap();
        assert_eq!(result2, None);
    }

    #[test]
    fn disable_rejects_other_extension() {
        assert!(target_for_disable("readme.txt").is_err());
        assert!(target_for_disable("config").is_err());
    }



    fn make_test_mods_dir() -> (TempDir, PathBuf) {
        let dir = TempDir::new().expect("tempdir");
        let mods_dir = dir.path().join("mods");
        std::fs::create_dir_all(&mods_dir).expect("create mods dir");
        (dir, mods_dir)
    }

    #[test]
    fn enable_mod_renames_disabled_to_jar() {
        let (_dir, mods_dir) = make_test_mods_dir();
        let src = mods_dir.join("mymod.jar.disabled");
        std::fs::write(&src, b"content").expect("write");
        assert!(src.exists());

        let result = target_for_enable("mymod.jar.disabled").unwrap();
        assert_eq!(result, Some("mymod.jar".to_string()));

        let dst = mods_dir.join("mymod.jar");
        std::fs::rename(&src, &dst).expect("rename");
        assert!(!src.exists());
        assert!(dst.exists());
    }

    #[test]
    fn disable_mod_renames_jar_to_disabled() {
        let (_dir, mods_dir) = make_test_mods_dir();
        let src = mods_dir.join("mymod.jar");
        std::fs::write(&src, b"content").expect("write");
        assert!(src.exists());

        let result = target_for_disable("mymod.jar").unwrap();
        assert_eq!(result, Some("mymod.jar.disabled".to_string()));

        let dst = mods_dir.join("mymod.jar.disabled");
        assert!(!dst.exists());
        std::fs::rename(&src, &dst).expect("rename");
        assert!(!src.exists());
        assert!(dst.exists());
    }

    #[test]
    fn enable_rejects_target_exists() {
        let (_dir, mods_dir) = make_test_mods_dir();
        let src = mods_dir.join("mymod.jar.disabled");
        let dst = mods_dir.join("mymod.jar");
        std::fs::write(&src, b"disabled content").expect("write");
        std::fs::write(&dst, b"existing jar").expect("write");

        let target = target_for_enable("mymod.jar.disabled").unwrap();
        assert!(target.is_some());
        let dst_check = mods_dir.join(target.unwrap());
        assert!(dst_check.exists());
    }

    #[test]
    fn delete_jar_file_succeeds() {
        let (_dir, mods_dir) = make_test_mods_dir();
        let path = mods_dir.join("oldmod.jar");
        std::fs::write(&path, b"junk").expect("write");
        assert!(path.exists());

        let ext = path.extension().and_then(|e| e.to_str()).unwrap_or("");
        assert_eq!(ext.to_lowercase(), "jar");

        std::fs::remove_file(&path).expect("remove");
        assert!(!path.exists());
    }

    #[test]
    fn delete_disabled_file_succeeds() {
        let (_dir, mods_dir) = make_test_mods_dir();
        let path = mods_dir.join("oldmod.jar.disabled");
        std::fs::write(&path, b"junk").expect("write");
        assert!(path.exists());

        let ext = path.extension().and_then(|e| e.to_str()).unwrap_or("");
        assert_eq!(ext.to_lowercase(), "disabled");

        std::fs::remove_file(&path).expect("remove");
        assert!(!path.exists());
    }

    #[test]
    fn delete_rejects_directory() {
        let (_dir, mods_dir) = make_test_mods_dir();
        let sub_dir = mods_dir.join("not_a_mod");
        std::fs::create_dir_all(&sub_dir).expect("create dir");
        assert!(sub_dir.is_dir());
    }

    #[test]
    fn delete_rejects_other_extension() {
        let (_dir, mods_dir) = make_test_mods_dir();
        let path = mods_dir.join("readme.txt");
        std::fs::write(&path, b"notes").expect("write");
        let ext = path.extension().and_then(|e| e.to_str()).unwrap_or("");
        assert!(!matches!(ext.to_lowercase().as_str(), "jar" | "disabled"));
    }

    #[test]
    fn mod_operation_result_delete_has_none_enabled() {
        let result = ModOperationResult {
            instance_id: "test-id".to_string(),
            old_path: "/tmp/old.jar".to_string(),
            new_path: None,
            file_name: "old.jar".to_string(),
            enabled: None,
            deleted: true,
        };
        assert!(result.deleted);
        assert!(result.enabled.is_none());
        assert!(result.new_path.is_none());
    }

    #[test]
    fn mod_operation_result_enable_has_some_enabled() {
        let result = ModOperationResult {
            instance_id: "test-id".to_string(),
            old_path: "/tmp/mod.jar.disabled".to_string(),
            new_path: Some("/tmp/mod.jar".to_string()),
            file_name: "mod.jar.disabled".to_string(),
            enabled: Some(true),
            deleted: false,
        };
        assert!(!result.deleted);
        assert_eq!(result.enabled, Some(true));
        assert!(result.new_path.is_some());
    }
}
