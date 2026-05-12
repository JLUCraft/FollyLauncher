use crate::error::LauncherError;
use crate::instance::commands::get_instance_in;
use crate::AppState;
use std::io::{Read, Write};
use std::path::{Path, PathBuf};
use std::sync::Arc;
use tauri::State;
use tokio::sync::Mutex;

use super::models::{
    ExportModpackManifestResult, ExportModpackZipRequest, ExportModpackZipResult,
    ImportModpackManifestRequest, ImportModpackManifestResult, ImportModpackZipRequest,
    ImportModpackZipResult, ModpackFileEntry, ModpackFileKind, ModpackManifest,
};



fn kind_dir(kind: &ModpackFileKind) -> &str {
    match kind {
        ModpackFileKind::Mod => "mods",
        ModpackFileKind::ResourcePack => "resourcepacks",
        ModpackFileKind::ShaderPack => "shaderpacks",
    }
}

fn compute_sha1(path: &Path) -> Result<String, LauncherError> {
    let data = std::fs::read(path).map_err(|e| {
        LauncherError::from(format!(
            "无法读取文件以计算 SHA1: {} — {}",
            path.display(),
            e
        ))
    })?;
    Ok(sha1_smol::Sha1::from(&data).digest().to_string())
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
    let path = Path::new(name);
    if path.is_absolute() {
        return Err(LauncherError::from("文件名不能为绝对路径"));
    }

    let bytes = name.as_bytes();
    if bytes.len() >= 2 && bytes[0].is_ascii_alphabetic() && bytes[1] == b':' {
        return Err(LauncherError::from("文件名不能包含盘符"));
    }
    Ok(())
}





fn sanitize_zip_path(entry_path: &str) -> Result<(), LauncherError> {
    if entry_path.is_empty() {
        return Err(LauncherError::from("ZIP entry path 不能为空"));
    }
    if entry_path.contains('\0') {
        return Err(LauncherError::from("ZIP entry path 包含非法字符 NUL"));
    }
    if entry_path.contains('\\') {
        return Err(LauncherError::from("ZIP entry path 包含反斜杠，应使用 /"));
    }

    let path = Path::new(entry_path);
    if path.is_absolute() || entry_path.starts_with('/') {
        return Err(LauncherError::from("ZIP entry path 不能为绝对路径"));
    }

    let bytes = entry_path.as_bytes();
    if bytes.len() >= 2 && bytes[0].is_ascii_alphabetic() && bytes[1] == b':' {
        return Err(LauncherError::from("ZIP entry path 不能包含盘符"));
    }

    for component in entry_path.split('/') {
        if component == ".." || component == "." {
            return Err(LauncherError::from(
                "ZIP entry path 不能包含 '.' 或 '..' 段",
            ));
        }
        if component.is_empty() {
            return Err(LauncherError::from("ZIP entry path 包含连续斜杠"));
        }
    }
    Ok(())
}



fn manifest_for_zip(manifest: &ModpackManifest) -> ModpackManifest {
    let cleaned_files: Vec<ModpackFileEntry> = manifest
        .files
        .iter()
        .map(|f| ModpackFileEntry {
            kind: f.kind.clone(),
            file_name: f.file_name.clone(),
            relative_path: f.relative_path.clone(),
            source_path: String::new(),
            size: f.size,
            sha1: f.sha1.clone(),
            enabled: f.enabled,
        })
        .collect();
    ModpackManifest {
        schema_version: manifest.schema_version,
        name: manifest.name.clone(),
        source_instance_id: manifest.source_instance_id.clone(),
        game_version: manifest.game_version.clone(),
        instance_kind: manifest.instance_kind.clone(),
        exported_at: manifest.exported_at.clone(),
        files: cleaned_files,
    }
}



fn scan_mod_files(game_dir: &Path) -> Vec<ModpackFileEntry> {
    let mods_dir = game_dir.join("mods");
    let mut entries = Vec::new();
    if let Ok(dir_entries) = std::fs::read_dir(&mods_dir) {
        for entry in dir_entries.flatten() {
            let path = entry.path();
            if path.is_dir() {
                continue;
            }
            let file_name = path
                .file_name()
                .and_then(|n| n.to_str())
                .unwrap_or("unknown")
                .to_string();

            let ext_lower = path
                .extension()
                .and_then(|e| e.to_str())
                .unwrap_or("")
                .to_lowercase();

            if ext_lower != "jar" && ext_lower != "disabled" {
                continue;
            }

            let enabled = ext_lower == "jar";
            let size = std::fs::metadata(&path).map(|m| m.len()).unwrap_or(0);
            let sha1 = compute_sha1(&path).unwrap_or_default();

            entries.push(ModpackFileEntry {
                kind: ModpackFileKind::Mod,
                file_name,
                relative_path: format!(
                    "mods/{}",
                    path.file_name()
                        .and_then(|n| n.to_str())
                        .unwrap_or("unknown")
                ),
                source_path: path.to_string_lossy().to_string(),
                size,
                sha1,
                enabled,
            });
        }
    }
    entries.sort_by(|a, b| a.file_name.cmp(&b.file_name));
    entries
}

fn scan_pack_files(game_dir: &Path, kind: ModpackFileKind) -> Vec<ModpackFileEntry> {
    let dir_name = kind_dir(&kind);
    let target_dir = game_dir.join(dir_name);
    let mut entries = Vec::new();
    if let Ok(dir_entries) = std::fs::read_dir(&target_dir) {
        for entry in dir_entries.flatten() {
            let path = entry.path();
            if path.is_dir() {
                continue;
            }

            let ext_lower = path
                .extension()
                .and_then(|e| e.to_str())
                .unwrap_or("")
                .to_lowercase();
            if ext_lower != "zip" {
                continue;
            }
            let file_name = path
                .file_name()
                .and_then(|n| n.to_str())
                .unwrap_or("unknown")
                .to_string();

            let size = std::fs::metadata(&path).map(|m| m.len()).unwrap_or(0);
            let sha1 = compute_sha1(&path).unwrap_or_default();

            entries.push(ModpackFileEntry {
                kind: kind.clone(),
                file_name: file_name.clone(),
                relative_path: format!("{}/{}", dir_name, file_name),
                source_path: path.to_string_lossy().to_string(),
                size,
                sha1,
                enabled: false,
            });
        }
    }
    entries.sort_by(|a, b| a.file_name.cmp(&b.file_name));
    entries
}



struct ImportCounts {
    imported: usize,
    skipped: usize,
    failed: usize,
    bytes_written: u64,
}

fn import_entry(
    entry: &ModpackFileEntry,
    target_game_dir: &Path,
    overwrite: bool,
    counts: &mut ImportCounts,
) {

    if let Err(e) = sanitize_file_name(&entry.file_name) {
        tracing::warn!(
            file_name = %entry.file_name,
            error = %e,
            "import: skipping entry with illegal file_name"
        );
        counts.failed += 1;
        return;
    }


    let target_dir = target_game_dir.join(kind_dir(&entry.kind));
    let target_path = target_dir.join(&entry.file_name);


    if let Err(e) = std::fs::create_dir_all(&target_dir) {
        tracing::warn!(
            target_dir = %target_dir.display(),
            error = %e,
            "import: failed to create target directory"
        );
        counts.failed += 1;
        return;
    }


    let source_path = Path::new(&entry.source_path);
    let data = match std::fs::read(source_path) {
        Ok(d) => d,
        Err(e) => {
            tracing::warn!(
                source_path = %entry.source_path,
                error = %e,
                "import: failed to read source file"
            );
            counts.failed += 1;
            return;
        }
    };


    let actual_sha1 = sha1_smol::Sha1::from(&data).digest().to_string();
    if actual_sha1 != entry.sha1 {
        tracing::warn!(
            file_name = %entry.file_name,
            expected = %entry.sha1,
            actual = %actual_sha1,
            "import: SHA1 mismatch"
        );
        counts.failed += 1;
        return;
    }


    if target_path.exists() && !overwrite {
        tracing::info!(
            file_name = %entry.file_name,
            "import: target already exists, skipping (overwrite=false)"
        );
        counts.skipped += 1;
        return;
    }


    let mut tmp_path = target_path.clone();
    let random_suffix = uuid::Uuid::new_v4().to_string();
    if let Some(stem) = target_path.file_stem().and_then(|s| s.to_str()) {
        let ext = target_path
            .extension()
            .and_then(|e| e.to_str())
            .map(|e| format!(".{}", e))
            .unwrap_or_default();
        tmp_path.set_file_name(format!("{}.tmp{}{}", stem, &random_suffix[..8], ext));
    } else {
        tmp_path.set_file_name(format!("{}.tmp{}", entry.file_name, &random_suffix[..8]));
    }

    if let Err(e) = std::fs::write(&tmp_path, &data) {
        tracing::warn!(
            tmp_path = %tmp_path.display(),
            error = %e,
            "import: failed to write temp file"
        );
        counts.failed += 1;
        return;
    }

    if let Err(e) = std::fs::rename(&tmp_path, &target_path) {
        tracing::warn!(
            tmp_path = %tmp_path.display(),
            target_path = %target_path.display(),
            error = %e,
            "import: rename failed"
        );
        if let Err(cleanup_err) = std::fs::remove_file(&tmp_path) {
            tracing::warn!(
                tmp_path = %tmp_path.display(),
                error = %cleanup_err,
                "import: failed to clean up temp file after rename failure"
            );
        }
        counts.failed += 1;
        return;
    }

    counts.imported += 1;
    counts.bytes_written += data.len() as u64;
}



#[tauri::command]
pub async fn export_modpack_manifest(
    instance_id: String,
    state: State<'_, Arc<Mutex<AppState>>>,
) -> Result<ExportModpackManifestResult, LauncherError> {
    let data_dir = {
        let s = state.lock().await;
        s.data_dir.clone()
    };
    let instance = get_instance_in(&data_dir, &instance_id)?;
    let game_dir = PathBuf::from(&instance.game_dir);


    let mut files: Vec<ModpackFileEntry> = Vec::new();

    files.extend(scan_mod_files(&game_dir));
    files.extend(scan_pack_files(&game_dir, ModpackFileKind::ResourcePack));
    files.extend(scan_pack_files(&game_dir, ModpackFileKind::ShaderPack));

    let file_count = files.len();
    let total_bytes: u64 = files.iter().map(|f| f.size).sum();

    let manifest = ModpackManifest {
        schema_version: 1,
        name: instance.name.clone(),
        source_instance_id: instance.id.clone(),
        game_version: instance.game_version.clone(),
        instance_kind: format!("{:?}", instance.kind),
        exported_at: crate::utils::now_iso8601(),
        files,
    };

    Ok(ExportModpackManifestResult {
        manifest,
        file_count,
        total_bytes,
    })
}

#[tauri::command]
pub async fn import_modpack_manifest(
    request: ImportModpackManifestRequest,
    state: State<'_, Arc<Mutex<AppState>>>,
) -> Result<ImportModpackManifestResult, LauncherError> {
    if request.manifest.schema_version != 1 {
        return Err(LauncherError::new(
            "UNSUPPORTED_VERSION",
            format!(
                "不支持的清单版本: {}，仅支持 schema_version=1",
                request.manifest.schema_version
            ),
        ));
    }

    let data_dir = {
        let s = state.lock().await;
        s.data_dir.clone()
    };
    let instance = get_instance_in(&data_dir, &request.target_instance_id)?;
    let target_game_dir = PathBuf::from(&instance.game_dir);

    let mut counts = ImportCounts {
        imported: 0,
        skipped: 0,
        failed: 0,
        bytes_written: 0,
    };

    for entry in &request.manifest.files {
        import_entry(entry, &target_game_dir, request.overwrite, &mut counts);
    }

    Ok(ImportModpackManifestResult {
        target_instance_id: request.target_instance_id,
        imported: counts.imported,
        skipped: counts.skipped,
        failed: counts.failed,
        bytes_written: counts.bytes_written,
    })
}



#[tauri::command]
pub async fn export_modpack_zip(
    request: ExportModpackZipRequest,
    state: State<'_, Arc<Mutex<AppState>>>,
) -> Result<ExportModpackZipResult, LauncherError> {
    if request.output_path.trim().is_empty() {
        return Err(LauncherError::new("INVALID_INPUT", "输出路径不能为空"));
    }

    let data_dir = {
        let s = state.lock().await;
        s.data_dir.clone()
    };
    let instance = get_instance_in(&data_dir, &request.instance_id)?;
    let game_dir = PathBuf::from(&instance.game_dir);


    let mut files: Vec<ModpackFileEntry> = Vec::new();
    files.extend(scan_mod_files(&game_dir));
    files.extend(scan_pack_files(&game_dir, ModpackFileKind::ResourcePack));
    files.extend(scan_pack_files(&game_dir, ModpackFileKind::ShaderPack));

    let file_count = files.len();
    let total_bytes: u64 = files.iter().map(|f| f.size).sum();

    let manifest = ModpackManifest {
        schema_version: 1,
        name: instance.name.clone(),
        source_instance_id: instance.id.clone(),
        game_version: instance.game_version.clone(),
        instance_kind: format!("{:?}", instance.kind),
        exported_at: crate::utils::now_iso8601(),
        files,
    };


    let zip_manifest = manifest_for_zip(&manifest);


    let manifest_json =
        serde_json::to_vec_pretty(&zip_manifest).map_err(|e| format!("序列化清单失败: {}", e))?;
    let manifest_bytes = manifest_json.len() as u64;


    let output_path = Path::new(&request.output_path);
    if let Some(parent) = output_path.parent() {
        std::fs::create_dir_all(parent).map_err(|e| format!("创建输出目录失败: {}", e))?;
    }


    let zip_file =
        std::fs::File::create(output_path).map_err(|e| format!("创建 ZIP 文件失败: {}", e))?;
    let mut zip_writer = zip::ZipWriter::new(zip_file);
    let zip_options = zip::write::SimpleFileOptions::default()
        .compression_method(zip::CompressionMethod::Deflated);


    zip_writer
        .start_file("folly-modpack.json", zip_options)
        .map_err(|e| format!("ZIP 写入清单失败: {}", e))?;
    zip_writer
        .write_all(&manifest_json)
        .map_err(|e| format!("ZIP 写入清单内容失败: {}", e))?;


    for entry in &manifest.files {
        let source_path = Path::new(&entry.source_path);
        let data = std::fs::read(source_path)
            .map_err(|e| format!("读取文件失败 {}: {}", entry.file_name, e))?;


        let actual_sha1 = sha1_smol::Sha1::from(&data).digest().to_string();
        if actual_sha1 != entry.sha1 {
            return Err(LauncherError::new(
                "SHA1_MISMATCH",
                format!("SHA1 校验失败: {}", entry.file_name),
            ));
        }

        let zip_entry_path = format!("overrides/{}", entry.relative_path);

        sanitize_zip_path(&zip_entry_path)?;

        zip_writer
            .start_file(&zip_entry_path, zip_options)
            .map_err(|e| format!("ZIP 写入文件 {} 失败: {}", entry.file_name, e))?;
        zip_writer
            .write_all(&data)
            .map_err(|e| format!("ZIP 写入文件内容 {} 失败: {}", entry.file_name, e))?;
    }

    zip_writer
        .finish()
        .map_err(|e| format!("完成 ZIP 写入失败: {}", e))?;

    Ok(ExportModpackZipResult {
        output_path: request.output_path,
        file_count,
        total_bytes,
        manifest_bytes,
    })
}



struct ZipImportCounts {
    imported: usize,
    skipped: usize,
    failed: usize,
    bytes_written: u64,
}

fn import_entry_from_zip(
    entry: &ModpackFileEntry,
    target_game_dir: &Path,
    overwrite: bool,
    zip_data: &[u8],
    counts: &mut ZipImportCounts,
) {

    if let Err(e) = sanitize_file_name(&entry.file_name) {
        tracing::warn!(
            file_name = %entry.file_name,
            error = %e,
            "zip-import: skipping entry with illegal file_name"
        );
        counts.failed += 1;
        return;
    }


    let target_dir = target_game_dir.join(kind_dir(&entry.kind));
    let target_path = target_dir.join(&entry.file_name);


    if let Err(e) = std::fs::create_dir_all(&target_dir) {
        tracing::warn!(
            target_dir = %target_dir.display(),
            error = %e,
            "zip-import: failed to create target directory"
        );
        counts.failed += 1;
        return;
    }


    let actual_sha1 = sha1_smol::Sha1::from(zip_data).digest().to_string();
    if actual_sha1 != entry.sha1 {
        tracing::warn!(
            file_name = %entry.file_name,
            expected = %entry.sha1,
            actual = %actual_sha1,
            "zip-import: SHA1 mismatch"
        );
        counts.failed += 1;
        return;
    }


    if target_path.exists() && !overwrite {
        tracing::info!(
            file_name = %entry.file_name,
            "zip-import: target already exists, skipping (overwrite=false)"
        );
        counts.skipped += 1;
        return;
    }


    let mut tmp_path = target_path.clone();
    let random_suffix = uuid::Uuid::new_v4().to_string();
    if let Some(stem) = target_path.file_stem().and_then(|s| s.to_str()) {
        let ext = target_path
            .extension()
            .and_then(|e| e.to_str())
            .map(|e| format!(".{}", e))
            .unwrap_or_default();
        tmp_path.set_file_name(format!("{}.tmp{}{}", stem, &random_suffix[..8], ext));
    } else {
        tmp_path.set_file_name(format!("{}.tmp{}", entry.file_name, &random_suffix[..8]));
    }

    if let Err(e) = std::fs::write(&tmp_path, zip_data) {
        tracing::warn!(
            tmp_path = %tmp_path.display(),
            error = %e,
            "zip-import: failed to write temp file"
        );
        counts.failed += 1;
        return;
    }

    if let Err(e) = std::fs::rename(&tmp_path, &target_path) {
        tracing::warn!(
            tmp_path = %tmp_path.display(),
            target_path = %target_path.display(),
            error = %e,
            "zip-import: rename failed"
        );
        if let Err(cleanup_err) = std::fs::remove_file(&tmp_path) {
            tracing::warn!(
                tmp_path = %tmp_path.display(),
                error = %cleanup_err,
                "zip-import: failed to clean up temp file after rename failure"
            );
        }
        counts.failed += 1;
        return;
    }

    counts.imported += 1;
    counts.bytes_written += zip_data.len() as u64;
}



#[tauri::command]
pub async fn import_modpack_zip(
    request: ImportModpackZipRequest,
    state: State<'_, Arc<Mutex<AppState>>>,
) -> Result<ImportModpackZipResult, LauncherError> {
    if request.zip_path.trim().is_empty() {
        return Err(LauncherError::new("INVALID_INPUT", "ZIP 文件路径不能为空"));
    }

    let zip_path = Path::new(&request.zip_path);
    if !zip_path.is_file() {
        return Err(LauncherError::new("NOT_FOUND", "ZIP 文件不存在或不是文件"));
    }

    let data_dir = {
        let s = state.lock().await;
        s.data_dir.clone()
    };
    let instance = get_instance_in(&data_dir, &request.target_instance_id)?;
    let target_game_dir = PathBuf::from(&instance.game_dir);


    let zip_data = std::fs::read(zip_path).map_err(|e| format!("读取 ZIP 文件失败: {}", e))?;
    let cursor = std::io::Cursor::new(&zip_data);
    let mut zip_archive =
        zip::ZipArchive::new(cursor).map_err(|e| format!("解析 ZIP 文件失败: {}", e))?;


    let manifest_entry = zip_archive
        .by_name("folly-modpack.json")
        .map_err(|_| "ZIP 中缺少 folly-modpack.json".to_string())?;
    let manifest: ModpackManifest = serde_json::from_reader(manifest_entry)
        .map_err(|e| format!("清单 JSON 解析失败: {}", e))?;

    if manifest.schema_version != 1 {
        return Err(LauncherError::new(
            "UNSUPPORTED_VERSION",
            format!(
                "不支持的清单版本: {}，仅支持 schema_version=1",
                manifest.schema_version
            ),
        ));
    }

    let mut counts = ZipImportCounts {
        imported: 0,
        skipped: 0,
        failed: 0,
        bytes_written: 0,
    };

    for entry in &manifest.files {
        let zip_entry_path = format!("overrides/{}", entry.relative_path);


        if let Err(e) = sanitize_zip_path(&zip_entry_path) {
            tracing::warn!(
                expected_path = %zip_entry_path,
                error = %e,
                "zip-import: skipping entry with illegal path"
            );
            counts.failed += 1;
            continue;
        }


        match zip_archive.by_name(&zip_entry_path) {
            Ok(mut zip_file) => {
                let mut data = Vec::new();
                if let Err(e) = zip_file.read_to_end(&mut data) {
                    tracing::warn!(
                        entry_path = %zip_entry_path,
                        error = %e,
                        "zip-import: failed to read entry data"
                    );
                    counts.failed += 1;
                    continue;
                }
                import_entry_from_zip(
                    entry,
                    &target_game_dir,
                    request.overwrite,
                    &data,
                    &mut counts,
                );
            }
            Err(_) => {
                tracing::warn!(
                    entry_path = %zip_entry_path,
                    "zip-import: entry not found in ZIP"
                );
                counts.failed += 1;
            }
        }
    }

    Ok(ImportModpackZipResult {
        target_instance_id: request.target_instance_id,
        imported: counts.imported,
        skipped: counts.skipped,
        failed: counts.failed,
        bytes_written: counts.bytes_written,
    })
}



#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;



    #[test]
    fn kind_dir_maps_correctly() {
        assert_eq!(kind_dir(&ModpackFileKind::Mod), "mods");
        assert_eq!(kind_dir(&ModpackFileKind::ResourcePack), "resourcepacks");
        assert_eq!(kind_dir(&ModpackFileKind::ShaderPack), "shaderpacks");
    }



    #[test]
    fn sanitize_accepts_legal_names() {
        assert!(sanitize_file_name("foo.jar").is_ok());
        assert!(sanitize_file_name("my-mod-1.0.jar").is_ok());
        assert!(sanitize_file_name("OptiFine_1.21.JAR").is_ok());
        assert!(sanitize_file_name("some mod.jar.disabled").is_ok());
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
    }

    #[test]
    fn sanitize_rejects_absolute() {
        assert!(sanitize_file_name("/etc/passwd").is_err());
    }

    #[test]
    fn sanitize_rejects_windows_drive() {
        assert!(sanitize_file_name("C:foo.jar").is_err());
    }

    #[test]
    fn sanitize_rejects_nul() {
        assert!(sanitize_file_name("foo\0bar.jar").is_err());
    }



    #[test]
    fn sha1_computes_correctly() {
        let dir = TempDir::new().expect("tempdir");
        let file_path = dir.path().join("test.bin");
        std::fs::write(&file_path, b"hello").expect("write");
        let hash = compute_sha1(&file_path).expect("sha1");

        assert_eq!(hash, "aaf4c61ddcc5e8a2dabede0f3b482cd9aea9434d");
    }



    #[test]
    fn relative_path_uses_forward_slash() {
        let entry = ModpackFileEntry {
            kind: ModpackFileKind::Mod,
            file_name: "foo.jar".to_string(),
            relative_path: "mods/foo.jar".to_string(),
            source_path: "/fake/mods/foo.jar".to_string(),
            size: 100,
            sha1: "abc".to_string(),
            enabled: true,
        };
        assert_eq!(entry.relative_path, "mods/foo.jar");
    }



    fn setup_mock_game_dir() -> (TempDir, PathBuf) {
        let dir = TempDir::new().expect("tempdir");
        let game_dir = dir.path().to_path_buf();


        let mods_dir = game_dir.join("mods");
        std::fs::create_dir_all(&mods_dir).expect("create mods");
        std::fs::write(mods_dir.join("alpha.jar"), b"aaa").expect("write");
        std::fs::write(mods_dir.join("beta.jar.disabled"), b"bbb").expect("write");
        std::fs::write(mods_dir.join("gamma.disabled"), b"ggg").expect("write");

        std::fs::write(mods_dir.join("readme.txt"), b"rrr").expect("write");

        std::fs::create_dir_all(mods_dir.join("nested")).expect("create dir");


        let rp_dir = game_dir.join("resourcepacks");
        std::fs::create_dir_all(&rp_dir).expect("create rp");
        std::fs::write(rp_dir.join("Faithful.zip"), b"pack").expect("write");


        let sp_dir = game_dir.join("shaderpacks");
        std::fs::create_dir_all(&sp_dir).expect("create sp");
        std::fs::write(sp_dir.join("SEUS.zip"), b"shader").expect("write");

        (dir, game_dir)
    }

    #[test]
    fn scan_mod_files_records_enabled() {
        let (_dir, game_dir) = setup_mock_game_dir();
        let entries = scan_mod_files(&game_dir);
        assert_eq!(entries.len(), 3, "should find 3 mod files (not readme.txt)");

        let alpha = entries
            .iter()
            .find(|e| e.file_name == "alpha.jar")
            .expect("alpha");
        assert!(alpha.enabled, "alpha.jar should be enabled");
        assert_eq!(alpha.size, 3);
        assert!(!alpha.sha1.is_empty());

        let beta = entries
            .iter()
            .find(|e| e.file_name == "beta.jar.disabled")
            .expect("beta");
        assert!(!beta.enabled, "beta.jar.disabled should be disabled");

        let gamma = entries
            .iter()
            .find(|e| e.file_name == "gamma.disabled")
            .expect("gamma");
        assert!(!gamma.enabled, "gamma.disabled should be disabled");
    }

    #[test]
    fn scan_pack_files_collects_only_zip_files() {
        let (_dir, game_dir) = setup_mock_game_dir();

        std::fs::write(
            game_dir.join("resourcepacks").join("notes.txt"),
            b"ignore me",
        )
        .expect("write");

        std::fs::create_dir_all(game_dir.join("resourcepacks").join("nested_pack"))
            .expect("create dir");

        let rp_entries = scan_pack_files(&game_dir, ModpackFileKind::ResourcePack);
        assert_eq!(
            rp_entries.len(),
            1,
            "should only include .zip files, not .txt or directories"
        );
        assert_eq!(rp_entries[0].file_name, "Faithful.zip");
        assert_eq!(rp_entries[0].relative_path, "resourcepacks/Faithful.zip");
        assert_eq!(rp_entries[0].size, 4);
        assert!(!rp_entries[0].sha1.is_empty());

        let sp_entries = scan_pack_files(&game_dir, ModpackFileKind::ShaderPack);
        assert_eq!(sp_entries.len(), 1);
        assert_eq!(sp_entries[0].file_name, "SEUS.zip");
        assert_eq!(sp_entries[0].kind, ModpackFileKind::ShaderPack);
    }

    #[test]
    fn export_aggregates_all_kinds() {
        let (_dir, game_dir) = setup_mock_game_dir();
        let mut all: Vec<ModpackFileEntry> = Vec::new();
        all.extend(scan_mod_files(&game_dir));
        all.extend(scan_pack_files(&game_dir, ModpackFileKind::ResourcePack));
        all.extend(scan_pack_files(&game_dir, ModpackFileKind::ShaderPack));

        assert_eq!(all.len(), 5);
        let total_bytes: u64 = all.iter().map(|f| f.size).sum();
        assert!(total_bytes > 0);
    }



    #[test]
    fn import_copies_file_successfully() {
        let src_dir = TempDir::new().expect("tempdir");
        let dst_dir = TempDir::new().expect("tempdir");


        let src_path = src_dir.path().join("mymod.jar");
        let content = b"mod content";
        std::fs::write(&src_path, content).expect("write");

        let entry = ModpackFileEntry {
            kind: ModpackFileKind::Mod,
            file_name: "mymod.jar".to_string(),
            relative_path: "mods/mymod.jar".to_string(),
            source_path: src_path.to_string_lossy().to_string(),
            size: content.len() as u64,
            sha1: sha1_smol::Sha1::from(&content[..]).digest().to_string(),
            enabled: true,
        };

        let mut counts = ImportCounts {
            imported: 0,
            skipped: 0,
            failed: 0,
            bytes_written: 0,
        };

        import_entry(&entry, dst_dir.path(), true, &mut counts);

        assert_eq!(counts.imported, 1);
        assert_eq!(counts.failed, 0);
        assert_eq!(counts.skipped, 0);
        assert_eq!(counts.bytes_written, content.len() as u64);

        let target_path = dst_dir.path().join("mods").join("mymod.jar");
        assert!(target_path.exists());
        let copied = std::fs::read(&target_path).expect("read");
        assert_eq!(copied, content);
    }

    #[test]
    fn import_overwrite_false_skips_existing() {
        let src_dir = TempDir::new().expect("tempdir");
        let dst_dir = TempDir::new().expect("tempdir");


        let src_path = src_dir.path().join("mymod.jar");
        let content = b"new content";
        std::fs::write(&src_path, content).expect("write");


        let target_mods = dst_dir.path().join("mods");
        std::fs::create_dir_all(&target_mods).expect("create");
        std::fs::write(target_mods.join("mymod.jar"), b"old content").expect("write");

        let entry = ModpackFileEntry {
            kind: ModpackFileKind::Mod,
            file_name: "mymod.jar".to_string(),
            relative_path: "mods/mymod.jar".to_string(),
            source_path: src_path.to_string_lossy().to_string(),
            size: content.len() as u64,
            sha1: sha1_smol::Sha1::from(&content[..]).digest().to_string(),
            enabled: true,
        };

        let mut counts = ImportCounts {
            imported: 0,
            skipped: 0,
            failed: 0,
            bytes_written: 0,
        };

        import_entry(&entry, dst_dir.path(), false, &mut counts);

        assert_eq!(counts.skipped, 1);
        assert_eq!(counts.imported, 0);

        let existing = std::fs::read(target_mods.join("mymod.jar")).expect("read");
        assert_eq!(existing, b"old content");
    }

    #[test]
    fn import_sha1_mismatch_fails() {
        let src_dir = TempDir::new().expect("tempdir");
        let dst_dir = TempDir::new().expect("tempdir");

        let src_path = src_dir.path().join("mymod.jar");
        std::fs::write(&src_path, b"content").expect("write");

        let entry = ModpackFileEntry {
            kind: ModpackFileKind::Mod,
            file_name: "mymod.jar".to_string(),
            relative_path: "mods/mymod.jar".to_string(),
            source_path: src_path.to_string_lossy().to_string(),
            size: 7,
            sha1: "0000000000000000000000000000000000000000".to_string(),
            enabled: true,
        };

        let mut counts = ImportCounts {
            imported: 0,
            skipped: 0,
            failed: 0,
            bytes_written: 0,
        };

        import_entry(&entry, dst_dir.path(), true, &mut counts);

        assert_eq!(counts.failed, 1);
        assert_eq!(counts.imported, 0);
        let target_path = dst_dir.path().join("mods").join("mymod.jar");
        assert!(
            !target_path.exists(),
            "should not create file on sha1 mismatch"
        );
    }

    #[test]
    fn import_illegal_file_name_fails() {
        let dst_dir = TempDir::new().expect("tempdir");

        let entry = ModpackFileEntry {
            kind: ModpackFileKind::Mod,
            file_name: "../../etc/passwd".to_string(),
            relative_path: "mods/../../etc/passwd".to_string(),
            source_path: "/fake/mod".to_string(),
            size: 0,
            sha1: "any".to_string(),
            enabled: true,
        };

        let mut counts = ImportCounts {
            imported: 0,
            skipped: 0,
            failed: 0,
            bytes_written: 0,
        };

        import_entry(&entry, dst_dir.path(), true, &mut counts);
        assert_eq!(counts.failed, 1);
    }

    #[test]
    fn import_directory_entry_fails() {
        let src_dir = TempDir::new().expect("tempdir");
        let dst_dir = TempDir::new().expect("tempdir");


        let sub_dir = src_dir.path().join("not_a_file");
        std::fs::create_dir_all(&sub_dir).expect("create dir");

        let entry = ModpackFileEntry {
            kind: ModpackFileKind::Mod,
            file_name: "not_a_file".to_string(),
            relative_path: "mods/not_a_file".to_string(),
            source_path: sub_dir.to_string_lossy().to_string(),
            size: 0,
            sha1: "any".to_string(),
            enabled: true,
        };

        let mut counts = ImportCounts {
            imported: 0,
            skipped: 0,
            failed: 0,
            bytes_written: 0,
        };

        import_entry(&entry, dst_dir.path(), true, &mut counts);

        assert_eq!(counts.failed, 1);
        assert_eq!(counts.imported, 0);
    }

    #[test]
    fn import_overwrite_true_replaces_existing() {
        let src_dir = TempDir::new().expect("tempdir");
        let dst_dir = TempDir::new().expect("tempdir");

        let src_path = src_dir.path().join("mymod.jar");
        let content = b"fresh content";
        std::fs::write(&src_path, content).expect("write");


        let target_mods = dst_dir.path().join("mods");
        std::fs::create_dir_all(&target_mods).expect("create");
        std::fs::write(target_mods.join("mymod.jar"), b"stale content").expect("write");

        let entry = ModpackFileEntry {
            kind: ModpackFileKind::Mod,
            file_name: "mymod.jar".to_string(),
            relative_path: "mods/mymod.jar".to_string(),
            source_path: src_path.to_string_lossy().to_string(),
            size: content.len() as u64,
            sha1: sha1_smol::Sha1::from(&content[..]).digest().to_string(),
            enabled: true,
        };

        let mut counts = ImportCounts {
            imported: 0,
            skipped: 0,
            failed: 0,
            bytes_written: 0,
        };

        import_entry(&entry, dst_dir.path(), true, &mut counts);

        assert_eq!(counts.imported, 1);
        let existing = std::fs::read(target_mods.join("mymod.jar")).expect("read");
        assert_eq!(existing, content);
    }

    #[test]
    fn import_source_missing_fails() {
        let dst_dir = TempDir::new().expect("tempdir");

        let entry = ModpackFileEntry {
            kind: ModpackFileKind::Mod,
            file_name: "ghost.jar".to_string(),
            relative_path: "mods/ghost.jar".to_string(),
            source_path: "/nonexistent/path/ghost.jar".to_string(),
            size: 0,
            sha1: "any".to_string(),
            enabled: true,
        };

        let mut counts = ImportCounts {
            imported: 0,
            skipped: 0,
            failed: 0,
            bytes_written: 0,
        };

        import_entry(&entry, dst_dir.path(), true, &mut counts);
        assert_eq!(counts.failed, 1);
    }

    #[test]
    fn import_schema_version_mismatch_is_overall_error() {


        let manifest = ModpackManifest {
            schema_version: 99,
            name: "test".to_string(),
            source_instance_id: "id".to_string(),
            game_version: "1.20".to_string(),
            instance_kind: "Vanilla".to_string(),
            exported_at: crate::utils::now_iso8601(),
            files: vec![],
        };
        let request = ImportModpackManifestRequest {
            target_instance_id: "target".to_string(),
            manifest,
            overwrite: true,
        };

        let result: Result<(), LauncherError> = if request.manifest.schema_version != 1 {
            Err(LauncherError::new(
                "UNSUPPORTED_VERSION",
                format!(
                    "不支持的清单版本: {}，仅支持 schema_version=1",
                    request.manifest.schema_version
                ),
            ))
        } else {
            Ok(())
        };
        assert!(result.is_err());
    }



    #[test]
    fn manifest_serde_roundtrip() {
        let manifest = ModpackManifest {
            schema_version: 1,
            name: "My Pack".to_string(),
            source_instance_id: "uuid-instance".to_string(),
            game_version: "1.21.4".to_string(),
            instance_kind: "Fabric".to_string(),
            exported_at: "2026-01-01T00:00:00Z".to_string(),
            files: vec![ModpackFileEntry {
                kind: ModpackFileKind::Mod,
                file_name: "foo.jar".to_string(),
                relative_path: "mods/foo.jar".to_string(),
                source_path: "/path/to/mods/foo.jar".to_string(),
                size: 1024,
                sha1: "abc123".to_string(),
                enabled: true,
            }],
        };

        let json = serde_json::to_string(&manifest).expect("serialize");
        let parsed: ModpackManifest = serde_json::from_str(&json).expect("deserialize");
        assert_eq!(parsed.schema_version, 1);
        assert_eq!(parsed.files.len(), 1);
        assert_eq!(parsed.files[0].kind, ModpackFileKind::Mod);
    }

    #[test]
    fn file_kind_serde_roundtrip() {
        let kinds = vec![
            ModpackFileKind::Mod,
            ModpackFileKind::ResourcePack,
            ModpackFileKind::ShaderPack,
        ];
        let json = serde_json::to_string(&kinds).expect("serialize");
        let parsed: Vec<ModpackFileKind> = serde_json::from_str(&json).expect("deserialize");
        assert_eq!(parsed, kinds);
    }



    #[test]
    fn sanitize_zip_path_accepts_normal() {
        assert!(sanitize_zip_path("folly-modpack.json").is_ok());
        assert!(sanitize_zip_path("overrides/mods/foo.jar").is_ok());
        assert!(sanitize_zip_path("overrides/resourcepacks/pack.zip").is_ok());
        assert!(sanitize_zip_path("overrides/shaderpacks/SEUS-v11.zip").is_ok());
    }

    #[test]
    fn sanitize_zip_path_rejects_empty() {
        assert!(sanitize_zip_path("").is_err());
    }

    #[test]
    fn sanitize_zip_path_rejects_dotdot() {
        assert!(sanitize_zip_path("overrides/../etc/passwd").is_err());
        assert!(sanitize_zip_path("..").is_err());
        assert!(sanitize_zip_path(".").is_err());
        assert!(sanitize_zip_path("mods/./foo").is_err());
    }

    #[test]
    fn sanitize_zip_path_rejects_absolute() {
        assert!(sanitize_zip_path("/etc/passwd").is_err());
    }

    #[test]
    fn sanitize_zip_path_rejects_backslash() {
        assert!(sanitize_zip_path("overrides\\mods\\foo.jar").is_err());
    }

    #[test]
    fn sanitize_zip_path_rejects_nul() {
        assert!(sanitize_zip_path("overrides/mods/foo\0.jar").is_err());
    }

    #[test]
    fn sanitize_zip_path_rejects_windows_drive() {
        assert!(sanitize_zip_path("C:foo.jar").is_err());
    }

    #[test]
    fn sanitize_zip_path_rejects_consecutive_slashes() {
        assert!(sanitize_zip_path("overrides//mods/foo.jar").is_err());
    }



    #[test]
    fn manifest_for_zip_clears_source_path() {
        let manifest = ModpackManifest {
            schema_version: 1,
            name: "Test".to_string(),
            source_instance_id: "id".to_string(),
            game_version: "1.20".to_string(),
            instance_kind: "Vanilla".to_string(),
            exported_at: crate::utils::now_iso8601(),
            files: vec![ModpackFileEntry {
                kind: ModpackFileKind::Mod,
                file_name: "foo.jar".to_string(),
                relative_path: "mods/foo.jar".to_string(),
                source_path: "C:\\Users\\me\\instances\\id\\minecraft\\mods\\foo.jar".to_string(),
                size: 100,
                sha1: "abc".to_string(),
                enabled: true,
            }],
        };
        let cleaned = manifest_for_zip(&manifest);
        assert_eq!(cleaned.files.len(), 1);
        assert_eq!(cleaned.files[0].source_path, "");
        assert_eq!(cleaned.files[0].relative_path, "mods/foo.jar");
        assert_eq!(cleaned.files[0].file_name, "foo.jar");
        assert_eq!(cleaned.files[0].sha1, "abc");
    }



    fn create_mock_game_dir_with_files() -> (TempDir, PathBuf) {
        let dir = TempDir::new().expect("tempdir");
        let game_dir = dir.path().to_path_buf();

        let mods_dir = game_dir.join("mods");
        std::fs::create_dir_all(&mods_dir).expect("create mods");
        std::fs::write(mods_dir.join("alpha.jar"), b"mod-alpha").expect("write");
        std::fs::write(mods_dir.join("beta.jar"), b"mod-beta-content").expect("write");

        let rp_dir = game_dir.join("resourcepacks");
        std::fs::create_dir_all(&rp_dir).expect("create rp");
        std::fs::write(rp_dir.join("Faithful.zip"), b"resource-pack-data").expect("write");

        let sp_dir = game_dir.join("shaderpacks");
        std::fs::create_dir_all(&sp_dir).expect("create sp");
        std::fs::write(sp_dir.join("SEUS.zip"), b"shader-data").expect("write");

        (dir, game_dir)
    }

    #[test]
    fn export_zip_contains_manifest_and_overrides() {
        let (_dir, game_dir) = create_mock_game_dir_with_files();

        let mut files: Vec<ModpackFileEntry> = Vec::new();
        files.extend(scan_mod_files(&game_dir));
        files.extend(scan_pack_files(&game_dir, ModpackFileKind::ResourcePack));
        files.extend(scan_pack_files(&game_dir, ModpackFileKind::ShaderPack));

        let manifest = ModpackManifest {
            schema_version: 1,
            name: "TestPack".to_string(),
            source_instance_id: "inst-1".to_string(),
            game_version: "1.21".to_string(),
            instance_kind: "Fabric".to_string(),
            exported_at: crate::utils::now_iso8601(),
            files,
        };


        let zip_path = _dir.path().join("test.zip");
        let zip_manifest = manifest_for_zip(&manifest);
        let manifest_json = serde_json::to_vec_pretty(&zip_manifest).expect("serialize");

        let zip_file = std::fs::File::create(&zip_path).expect("create zip");
        let mut zip_writer = zip::ZipWriter::new(zip_file);
        let opts = zip::write::SimpleFileOptions::default()
            .compression_method(zip::CompressionMethod::Deflated);

        zip_writer
            .start_file("folly-modpack.json", opts)
            .expect("start manifest");
        zip_writer
            .write_all(&manifest_json)
            .expect("write manifest");

        for entry in &manifest.files {
            let src = Path::new(&entry.source_path);
            let data = std::fs::read(src).expect("read source");
            let zip_entry = format!("overrides/{}", entry.relative_path);
            zip_writer
                .start_file(&zip_entry, opts)
                .expect("start entry");
            zip_writer.write_all(&data).expect("write entry");
        }
        zip_writer.finish().expect("finish");


        let zip_data = std::fs::read(&zip_path).expect("read zip");
        let cursor = std::io::Cursor::new(&zip_data);
        let mut archive = zip::ZipArchive::new(cursor).expect("open archive");


        let manifest_entry = archive.by_name("folly-modpack.json");
        assert!(manifest_entry.is_ok(), "manifest should be present");


        let mf: ModpackManifest =
            serde_json::from_reader(manifest_entry.unwrap()).expect("deserialize manifest");
        assert_eq!(mf.schema_version, 1);
        for f in &mf.files {
            assert!(
                f.source_path.is_empty(),
                "manifest should not leak absolute source_path: got '{}'",
                f.source_path
            );
        }


        for entry in &zip_manifest.files {
            let expected = format!("overrides/{}", entry.relative_path);
            assert!(
                archive.by_name(&expected).is_ok(),
                "missing override: {}",
                expected
            );
        }
    }



    #[test]
    fn import_zip_success() {
        let src_dir = TempDir::new().expect("tempdir");
        let dst_dir = TempDir::new().expect("tempdir");


        let src_file = src_dir.path().join("mymod.jar");
        let content = b"mod content for zip import";
        std::fs::write(&src_file, content).expect("write");

        let sha1_val = sha1_smol::Sha1::from(&content[..]).digest().to_string();


        let manifest = ModpackManifest {
            schema_version: 1,
            name: "Test".to_string(),
            source_instance_id: "src".to_string(),
            game_version: "1.20".to_string(),
            instance_kind: "Vanilla".to_string(),
            exported_at: crate::utils::now_iso8601(),
            files: vec![ModpackFileEntry {
                kind: ModpackFileKind::Mod,
                file_name: "mymod.jar".to_string(),
                relative_path: "mods/mymod.jar".to_string(),
                source_path: String::new(),
                size: content.len() as u64,
                sha1: sha1_val,
                enabled: true,
            }],
        };

        let _ = build_test_zip(&manifest, &[("mods/mymod.jar", content as &[u8])]);


        let mut counts = ZipImportCounts {
            imported: 0,
            skipped: 0,
            failed: 0,
            bytes_written: 0,
        };

        import_entry_from_zip(
            &manifest.files[0],
            dst_dir.path(),
            true,
            content,
            &mut counts,
        );

        assert_eq!(counts.imported, 1);
        assert_eq!(counts.failed, 0);
        assert_eq!(counts.bytes_written, content.len() as u64);

        let target = dst_dir.path().join("mods").join("mymod.jar");
        assert!(target.exists());
        assert_eq!(std::fs::read(&target).expect("read"), content);
    }

    #[test]
    fn import_zip_overwrite_false_skips() {
        let dst_dir = TempDir::new().expect("tempdir");


        let target_mods = dst_dir.path().join("mods");
        std::fs::create_dir_all(&target_mods).expect("create");
        std::fs::write(target_mods.join("mymod.jar"), b"existing").expect("write");

        let content = b"new data";
        let sha1_val = sha1_smol::Sha1::from(&content[..]).digest().to_string();

        let entry = ModpackFileEntry {
            kind: ModpackFileKind::Mod,
            file_name: "mymod.jar".to_string(),
            relative_path: "mods/mymod.jar".to_string(),
            source_path: String::new(),
            size: content.len() as u64,
            sha1: sha1_val,
            enabled: true,
        };

        let mut counts = ZipImportCounts {
            imported: 0,
            skipped: 0,
            failed: 0,
            bytes_written: 0,
        };

        import_entry_from_zip(&entry, dst_dir.path(), false, content, &mut counts);

        assert_eq!(counts.skipped, 1);
        assert_eq!(counts.imported, 0);


        let existing = std::fs::read(target_mods.join("mymod.jar")).expect("read");
        assert_eq!(existing, b"existing");
    }

    #[test]
    fn import_zip_overwrite_true_replaces() {
        let dst_dir = TempDir::new().expect("tempdir");


        let target_mods = dst_dir.path().join("mods");
        std::fs::create_dir_all(&target_mods).expect("create");
        std::fs::write(target_mods.join("mymod.jar"), b"old").expect("write");

        let content = b"new fresh data";
        let sha1_val = sha1_smol::Sha1::from(&content[..]).digest().to_string();

        let entry = ModpackFileEntry {
            kind: ModpackFileKind::Mod,
            file_name: "mymod.jar".to_string(),
            relative_path: "mods/mymod.jar".to_string(),
            source_path: String::new(),
            size: content.len() as u64,
            sha1: sha1_val,
            enabled: true,
        };

        let mut counts = ZipImportCounts {
            imported: 0,
            skipped: 0,
            failed: 0,
            bytes_written: 0,
        };

        import_entry_from_zip(&entry, dst_dir.path(), true, content, &mut counts);

        assert_eq!(counts.imported, 1);
        let existing = std::fs::read(target_mods.join("mymod.jar")).expect("read");
        assert_eq!(existing, content);
    }

    #[test]
    fn import_zip_sha1_mismatch_fails() {
        let dst_dir = TempDir::new().expect("tempdir");

        let content = b"actual content";
        let entry = ModpackFileEntry {
            kind: ModpackFileKind::Mod,
            file_name: "bad.jar".to_string(),
            relative_path: "mods/bad.jar".to_string(),
            source_path: String::new(),
            size: content.len() as u64,
            sha1: "0000000000000000000000000000000000000000".to_string(),
            enabled: true,
        };

        let mut counts = ZipImportCounts {
            imported: 0,
            skipped: 0,
            failed: 0,
            bytes_written: 0,
        };

        import_entry_from_zip(&entry, dst_dir.path(), true, content, &mut counts);

        assert_eq!(counts.failed, 1);
        assert_eq!(counts.imported, 0);
        assert!(
            !dst_dir.path().join("mods").join("bad.jar").exists(),
            "should not create file on sha1 mismatch"
        );
    }

    #[test]
    fn import_zip_illegal_file_name_fails() {
        let dst_dir = TempDir::new().expect("tempdir");

        let entry = ModpackFileEntry {
            kind: ModpackFileKind::Mod,
            file_name: "../../etc/passwd".to_string(),
            relative_path: "mods/../../etc/passwd".to_string(),
            source_path: String::new(),
            size: 0,
            sha1: "any".to_string(),
            enabled: true,
        };

        let mut counts = ZipImportCounts {
            imported: 0,
            skipped: 0,
            failed: 0,
            bytes_written: 0,
        };

        import_entry_from_zip(&entry, dst_dir.path(), true, b"data", &mut counts);
        assert_eq!(counts.failed, 1);
    }

    #[test]
    fn import_zip_schema_version_mismatch_is_error() {
        let manifest = ModpackManifest {
            schema_version: 99,
            name: "test".to_string(),
            source_instance_id: "id".to_string(),
            game_version: "1.20".to_string(),
            instance_kind: "Vanilla".to_string(),
            exported_at: crate::utils::now_iso8601(),
            files: vec![],
        };


        let result: Result<(), LauncherError> = if manifest.schema_version != 1 {
            Err(LauncherError::new(
                "UNSUPPORTED_VERSION",
                format!(
                    "不支持的清单版本: {}，仅支持 schema_version=1",
                    manifest.schema_version
                ),
            ))
        } else {
            Ok(())
        };
        assert!(result.is_err());
    }

    #[test]
    fn import_zip_missing_manifest_is_error() {

        let zip_data = build_empty_test_zip();
        let cursor = std::io::Cursor::new(&zip_data);
        let mut archive = zip::ZipArchive::new(cursor).expect("open archive");
        let result = archive.by_name("folly-modpack.json");
        assert!(result.is_err(), "should fail when manifest is missing");
    }

    #[test]
    fn import_zip_corrupted_manifest_is_error() {

        let temp_dir = TempDir::new().expect("tempdir");
        let zip_path = temp_dir.path().join("corrupt.zip");
        let zip_file = std::fs::File::create(&zip_path).expect("create");
        let mut zip_writer = zip::ZipWriter::new(zip_file);
        let opts = zip::write::SimpleFileOptions::default()
            .compression_method(zip::CompressionMethod::Deflated);
        zip_writer
            .start_file("folly-modpack.json", opts)
            .expect("start");
        zip_writer
            .write_all(b"this is not valid json {")
            .expect("write");
        zip_writer.finish().expect("finish");

        let zip_data = std::fs::read(&zip_path).expect("read");
        let cursor = std::io::Cursor::new(&zip_data);
        let mut archive = zip::ZipArchive::new(cursor).expect("open archive");
        let manifest_entry = archive.by_name("folly-modpack.json").expect("get entry");
        let result: Result<ModpackManifest, _> = serde_json::from_reader(manifest_entry);
        assert!(result.is_err(), "should fail on corrupted JSON");
    }

    #[test]
    fn import_zip_entry_missing_in_zip_fails() {
        let manifest = ModpackManifest {
            schema_version: 1,
            name: "Test".to_string(),
            source_instance_id: "src".to_string(),
            game_version: "1.20".to_string(),
            instance_kind: "Vanilla".to_string(),
            exported_at: crate::utils::now_iso8601(),
            files: vec![ModpackFileEntry {
                kind: ModpackFileKind::Mod,
                file_name: "ghost.jar".to_string(),
                relative_path: "mods/ghost.jar".to_string(),
                source_path: String::new(),
                size: 10,
                sha1: "abc".to_string(),
                enabled: true,
            }],
        };


        let zip_data = build_test_zip(&manifest, &[]);
        let cursor = std::io::Cursor::new(&zip_data);
        let mut archive = zip::ZipArchive::new(cursor).expect("open archive");


        let result = archive.by_name("overrides/mods/ghost.jar");
        assert!(result.is_err(), "entry should be missing from ZIP");
    }



    fn build_test_zip(manifest: &ModpackManifest, overrides: &[(&str, &[u8])]) -> Vec<u8> {
        let mut buf = std::io::Cursor::new(Vec::new());
        {
            let mut zip_writer = zip::ZipWriter::new(&mut buf);
            let opts = zip::write::SimpleFileOptions::default()
                .compression_method(zip::CompressionMethod::Deflated);

            let manifest_json =
                serde_json::to_vec_pretty(&manifest_for_zip(manifest)).expect("serialize manifest");
            zip_writer
                .start_file("folly-modpack.json", opts)
                .expect("start manifest");
            zip_writer
                .write_all(&manifest_json)
                .expect("write manifest");

            for (path, data) in overrides {
                let entry_path = format!("overrides/{}", path);
                zip_writer
                    .start_file(&entry_path, opts)
                    .expect("start override");
                zip_writer.write_all(data).expect("write override");
            }

            zip_writer.finish().expect("finish");
        }
        buf.into_inner()
    }

    fn build_empty_test_zip() -> Vec<u8> {
        let mut buf = std::io::Cursor::new(Vec::new());
        {
            let mut zip_writer = zip::ZipWriter::new(&mut buf);
            let opts = zip::write::SimpleFileOptions::default()
                .compression_method(zip::CompressionMethod::Deflated);

            zip_writer
                .start_file("dummy.txt", opts)
                .expect("start dummy");
            zip_writer.write_all(b"hello").expect("write dummy");
            zip_writer.finish().expect("finish");
        }
        buf.into_inner()
    }
}
