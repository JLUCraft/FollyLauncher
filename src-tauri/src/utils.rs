use std::collections::HashMap;
use std::path::Path;
use tokio::io::AsyncReadExt;

// ── Time ────────────────────────────────────────────────────────────────

/// Return the current UTC time as an RFC 3339 string.
pub fn now_iso8601() -> String {
    chrono::Utc::now().to_rfc3339()
}

// ── Varint ───────────────────────────────────────────────────────────────

/// Read a varint-prefixed integer from an async reader.
///
/// Used by both the control client (protobuf framing) and the proxy
/// (Minecraft protocol framing).
pub async fn read_varint_u64<R: tokio::io::AsyncRead + Unpin>(
    reader: &mut R,
) -> std::io::Result<u64> {
    let mut value: u64 = 0;
    let mut shift = 0u32;
    loop {
        let mut buf = [0u8; 1];
        reader.read_exact(&mut buf).await?;
        let byte = buf[0];
        value |= ((byte & 0x7F) as u64) << shift;
        if byte & 0x80 == 0 {
            break;
        }
        shift += 7;
        if shift >= 64 {
            return Err(std::io::Error::new(
                std::io::ErrorKind::InvalidData,
                "varint too long",
            ));
        }
    }
    Ok(value)
}

// ── JSON map persistence ────────────────────────────────────────────────

/// Load a `HashMap<String, String>` from a JSON file path.
/// Returns an empty map if the file does not exist or is empty.
pub fn load_json_map(path: &Path) -> Result<HashMap<String, String>, crate::error::LauncherError> {
    if !path.exists() {
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)
                .map_err(|e| crate::error::LauncherError::from(format!("cannot create data dir: {e}")))?;
        }
        return Ok(HashMap::new());
    }
    let content = std::fs::read_to_string(path)
        .map_err(|e| crate::error::LauncherError::from(format!("cannot read file {}: {e}", path.display())))?;
    if content.trim().is_empty() {
        return Ok(HashMap::new());
    }
    serde_json::from_str(&content)
        .map_err(|e| crate::error::LauncherError::from(format!("file {} is corrupted, cannot parse: {e}", path.display())))
}

/// Save a `HashMap<String, String>` as a pretty-printed JSON file.
pub fn save_json_map(path: &Path, map: &HashMap<String, String>) -> Result<(), crate::error::LauncherError> {
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)
            .map_err(|e| crate::error::LauncherError::from(format!("cannot create data dir: {e}")))?;
    }
    let json = serde_json::to_string_pretty(map)
        .map_err(|e| crate::error::LauncherError::from(format!("cannot serialize: {e}")))?;
    std::fs::write(path, json)
        .map_err(|e| crate::error::LauncherError::from(format!("cannot write file {}: {e}", path.display())))
}

// ── Tests ───────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn now_iso8601_is_rfc3339() {
        let ts = now_iso8601();
        assert!(ts.contains('T'));
        assert!(ts.contains(':'));
        assert!(!ts.is_empty());
    }
}
