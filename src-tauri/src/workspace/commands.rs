use serde::{Deserialize, Serialize};
use std::path::PathBuf;

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

fn get_instance_dir(instance_id: &str) -> PathBuf {
    let data_dir = crate::project_data_dir().unwrap_or_else(|_| PathBuf::from("."));
    data_dir
        .join("instances")
        .join(instance_id)
        .join(".minecraft")
}

fn read_dir_names(dir: &PathBuf) -> Vec<(String, PathBuf)> {
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

#[tauri::command]
pub async fn retrieve_world_list(instance_id: String) -> Result<Vec<WorldInfo>, String> {
    let game_dir = get_instance_dir(&instance_id);
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
    Ok(worlds)
}

#[tauri::command]
pub async fn retrieve_screenshot_list(instance_id: String) -> Result<Vec<ScreenshotInfo>, String> {
    let game_dir = get_instance_dir(&instance_id);
    let screenshots_dir = game_dir.join("screenshots");

    let mut screenshots = Vec::new();
    for (name, path) in read_dir_names(&screenshots_dir) {
        if path.extension() == Some(std::ffi::OsStr::new("png")) {
            let timestamp = std::fs::metadata(&path)
                .ok()
                .and_then(|m| m.modified().ok())
                .and_then(|t| t.duration_since(std::time::UNIX_EPOCH).ok())
                .map(|d| d.as_secs())
                .unwrap_or(0);

            screenshots.push(ScreenshotInfo {
                name,
                path: path.to_string_lossy().to_string(),
                timestamp,
            });
        }
    }
    screenshots.sort_by(|a, b| b.timestamp.cmp(&a.timestamp));
    Ok(screenshots)
}

#[tauri::command]
pub async fn retrieve_resource_pack_list(
    instance_id: String,
) -> Result<Vec<ResourcePackInfo>, String> {
    let game_dir = get_instance_dir(&instance_id);
    let resourcepacks_dir = game_dir.join("resourcepacks");

    let packs = read_dir_names(&resourcepacks_dir)
        .into_iter()
        .map(|(name, path)| ResourcePackInfo {
            name,
            path: path.to_string_lossy().to_string(),
            enabled: false,
        })
        .collect();
    Ok(packs)
}

#[tauri::command]
pub async fn retrieve_shader_pack_list(
    instance_id: String,
) -> Result<Vec<ResourcePackInfo>, String> {
    let game_dir = get_instance_dir(&instance_id);
    let shaderpacks_dir = game_dir.join("shaderpacks");

    let packs = read_dir_names(&shaderpacks_dir)
        .into_iter()
        .map(|(name, path)| ResourcePackInfo {
            name,
            path: path.to_string_lossy().to_string(),
            enabled: false,
        })
        .collect();
    Ok(packs)
}

#[tauri::command]
pub async fn retrieve_game_server_list(instance_id: String) -> Result<Vec<ServerInfo>, String> {
    let game_dir = get_instance_dir(&instance_id);
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
    Ok(servers)
}

fn parse_servers_dat(path: &PathBuf) -> Result<Vec<ServerInfo>, String> {
    let data = std::fs::read(path).map_err(|e| format!("无法读取 servers.dat: {}", e))?;
    let bytes: &[u8] = data.as_ref();
    let mut cursor = std::io::Cursor::new(bytes);

    // Read root compound tag
    let root_type = read_u8(&mut cursor)?;
    if root_type != 0x0a {
        return Err(format!("servers.dat 格式错误: 期望 Compound (0x0a), 得到 {:#x}", root_type));
    }
    read_nbt_string(&mut cursor)?; // root name length + name bytes (empty string typically)

    // Read "servers" list
    let tag_type = read_u8(&mut cursor)?;
    if tag_type != 0x09 {
        return Err(format!("期望 List tag, 得到 {:#x}", tag_type));
    }
    read_nbt_string(&mut cursor)?; // "servers"
    let list_type = read_u8(&mut cursor)?; // tag type of list elements
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
            let ft = field_type.map_err(|e| format!("读取标签类型失败: {}", e))?;
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

fn read_u8(cursor: &mut std::io::Cursor<&[u8]>) -> Result<u8, String> {
    use std::io::Read;
    let mut buf = [0u8; 1];
    cursor.read_exact(&mut buf).map_err(|e| format!("读取字节失败: {}", e))?;
    Ok(buf[0])
}

fn read_i32_be(cursor: &mut std::io::Cursor<&[u8]>) -> Result<i32, String> {
    use std::io::Read;
    let mut buf = [0u8; 4];
    cursor.read_exact(&mut buf).map_err(|e| format!("读取 i32 失败: {}", e))?;
    Ok(i32::from_be_bytes(buf))
}

fn read_u16_be(src: &mut std::io::Cursor<&[u8]>) -> Result<u16, String> {
    use std::io::Read;
    let mut buf = [0u8; 2];
    src.read_exact(&mut buf).map_err(|e| format!("读取 u16 失败: {}", e))?;
    Ok(u16::from_be_bytes(buf))
}

fn read_nbt_string(cursor: &mut std::io::Cursor<&[u8]>) -> Result<String, String> {
    let len = read_u16_be(cursor)? as usize;
    use std::io::Read;
    let mut buf = vec![0u8; len];
    cursor.read_exact(&mut buf).map_err(|e| format!("读取 NBT 字符串失败: {}", e))?;
    String::from_utf8(buf).map_err(|e| format!("NBT 字符串 UTF-8 无效: {}", e))
}

fn skip_nbt_value(cursor: &mut std::io::Cursor<&[u8]>, tag_type: u8) -> Result<(), String> {
    use std::io::{Read, Seek};
    match tag_type {
        0x01 => { cursor.read_exact(&mut [0u8; 1]).map_err(|e| e.to_string())?; } // Byte
        0x02 => { cursor.read_exact(&mut [0u8; 2]).map_err(|e| e.to_string())?; } // Short
        0x03 => { cursor.read_exact(&mut [0u8; 4]).map_err(|e| e.to_string())?; } // Int
        0x04 => { cursor.read_exact(&mut [0u8; 8]).map_err(|e| e.to_string())?; } // Long
        0x05 => { cursor.read_exact(&mut [0u8; 4]).map_err(|e| e.to_string())?; } // Float
        0x06 => { cursor.read_exact(&mut [0u8; 8]).map_err(|e| e.to_string())?; } // Double
        0x07 => { let len = read_i32_be(cursor)?; cursor.seek_relative(len as i64 * 4).map_err(|e| e.to_string())?; } // Int Array
        0x08 => { read_nbt_string(cursor)?; } // String
        0x09 => {
            let el_type = read_u8(cursor)?;
            let count = read_i32_be(cursor)?;
            for _ in 0..count { skip_nbt_value(cursor, el_type)?; }
        } // List
        0x0a => {
            loop {
                let ft = read_u8(cursor)?;
                if ft == 0x00 { break; }
                read_nbt_string(cursor)?;
                skip_nbt_value(cursor, ft)?;
            }
        } // Compound
        0x0b => { let len = read_i32_be(cursor)?; cursor.seek_relative(len as i64 * 4).map_err(|e| e.to_string())?; } // Int Array (legacy)
        0x0c => { let len = read_i32_be(cursor)?; cursor.seek_relative(len as i64 * 8).map_err(|e| e.to_string())?; } // Long Array
        _ => return Err(format!("unknown NBT tag type: {:#x}", tag_type)),
    }
    Ok(())
}
