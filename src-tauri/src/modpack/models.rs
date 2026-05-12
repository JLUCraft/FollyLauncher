use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum ModpackFileKind {
    Mod,
    ResourcePack,
    ShaderPack,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModpackFileEntry {
    pub kind: ModpackFileKind,
    pub file_name: String,
    pub relative_path: String,
    pub source_path: String,
    pub size: u64,
    pub sha1: String,
    pub enabled: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModpackManifest {
    pub schema_version: u32,
    pub name: String,
    pub source_instance_id: String,
    pub game_version: String,
    pub instance_kind: String,
    pub exported_at: String,
    pub files: Vec<ModpackFileEntry>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExportModpackManifestResult {
    pub manifest: ModpackManifest,
    pub file_count: usize,
    pub total_bytes: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ImportModpackManifestRequest {
    pub target_instance_id: String,
    pub manifest: ModpackManifest,
    pub overwrite: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ImportModpackManifestResult {
    pub target_instance_id: String,
    pub imported: usize,
    pub skipped: usize,
    pub failed: usize,
    pub bytes_written: u64,
}



#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ExportModpackZipRequest {
    pub instance_id: String,
    pub output_path: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ExportModpackZipResult {
    pub output_path: String,
    pub file_count: usize,
    pub total_bytes: u64,
    pub manifest_bytes: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ImportModpackZipRequest {
    pub target_instance_id: String,
    pub zip_path: String,
    pub overwrite: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ImportModpackZipResult {
    pub target_instance_id: String,
    pub imported: usize,
    pub skipped: usize,
    pub failed: usize,
    pub bytes_written: u64,
}
