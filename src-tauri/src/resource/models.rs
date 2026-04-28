use serde::{Deserialize, Serialize};
use std::str::FromStr;

#[derive(Debug, PartialEq, Eq, Clone, Copy, Serialize, Deserialize, Default)]
pub enum ModLoaderType {
    #[default]
    Unknown,
    Forge,
    Fabric,
    NeoForge,
    Quilt,
    LiteLoader,
    LegacyForge,
    OptiFine,
}

impl std::fmt::Display for ModLoaderType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ModLoaderType::Unknown => write!(f, "Unknown"),
            ModLoaderType::Forge => write!(f, "Forge"),
            ModLoaderType::Fabric => write!(f, "Fabric"),
            ModLoaderType::NeoForge => write!(f, "NeoForge"),
            ModLoaderType::Quilt => write!(f, "Quilt"),
            ModLoaderType::LiteLoader => write!(f, "LiteLoader"),
            ModLoaderType::LegacyForge => write!(f, "LegacyForge"),
            ModLoaderType::OptiFine => write!(f, "OptiFine"),
        }
    }
}

#[derive(Eq, Hash, PartialEq, Clone, Copy, Debug)]
pub enum ResourceType {
    VersionManifest,
    ForgeMeta,
    OptiFine,
    FabricMeta,
    NeoforgeMetaForge,
    NeoforgeMetaNeoforge,
    QuiltMeta,
}

#[derive(Eq, Hash, PartialEq, Clone, Copy, Debug)]
pub enum SourceType {
    Official,
    BMCLAPIMirror,
}

#[derive(Debug, PartialEq, Eq, Clone, Deserialize, Serialize, Default)]
pub enum OtherResourceSource {
    #[default]
    Unknown,
    CurseForge,
    Modrinth,
    MultiMc,
}

impl FromStr for OtherResourceSource {
    type Err = String;

    fn from_str(input: &str) -> Result<Self, Self::Err> {
        match input.to_lowercase().as_str() {
            "curseforge" => Ok(OtherResourceSource::CurseForge),
            "modrinth" => Ok(OtherResourceSource::Modrinth),
            "multimc" => Ok(OtherResourceSource::MultiMc),
            _ => Err(format!("Unknown resource download type: {}", input)),
        }
    }
}

#[derive(Debug, Clone, Copy)]
pub enum OtherResourceApiEndpoint {
    Search,
    VersionPack,
    FromLocal,
    ById,
    TranslateDesc,
}

#[derive(Debug, PartialEq, Eq, Clone, Deserialize, Serialize, Default)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct OtherResourceInfo {
    pub id: String,
    pub mcmod_id: u32,
    pub _type: String,
    pub name: String,
    pub slug: String,
    pub translated_name: Option<String>,
    pub description: String,
    pub translated_description: Option<String>,
    pub icon_src: String,
    pub tags: Vec<String>,
    pub last_updated: String,
    pub downloads: u64,
    pub source: OtherResourceSource,
    pub website_url: String,
    pub author: Option<String>,
}

#[derive(Debug, PartialEq, Eq, Clone, Deserialize, Serialize, Default)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct OtherResourceSearchRes {
    pub list: Vec<OtherResourceInfo>,
    pub total: u64,
    pub page: u32,
    pub page_size: u32,
}

#[derive(Debug, PartialEq, Eq, Clone, Deserialize, Serialize, Default)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct OtherResourceSearchQuery {
    pub resource_type: String,
    pub search_query: String,
    pub game_version: String,
    pub selected_tag: String,
    pub sort_by: String,
    pub page: u32,
    pub page_size: u32,
}

#[derive(Debug, PartialEq, Eq, Clone, Deserialize, Serialize, Default)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct OtherResourceVersionPackQuery {
    pub resource_id: String,
    pub mod_loader: String,
    pub game_versions: Vec<String>,
}

#[derive(Debug, PartialEq, Eq, Clone, Deserialize, Serialize, Default)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct OtherResourceFileInfo {
    pub resource_id: String,
    pub name: String,
    pub release_type: String,
    pub downloads: u64,
    pub file_date: String,
    pub download_url: String,
    pub sha1: String,
    pub file_name: String,
    pub dependencies: Vec<OtherResourceDependency>,
    pub loader: Option<String>,
}

#[derive(Debug, PartialEq, Eq, Clone, Deserialize, Serialize, Default)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct OtherResourceDependency {
    pub resource_id: String,
    pub relation: String,
}

#[derive(Debug, PartialEq, Eq, Clone, Deserialize, Serialize, Default)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct OtherResourceVersionPack {
    pub name: String,
    pub items: Vec<OtherResourceFileInfo>,
}

#[derive(Debug, PartialEq, Eq, Clone, Deserialize, Serialize, Default)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ModUpdateQuery {
    pub url: String,
    pub sha1: String,
    pub file_name: String,
    pub old_file_path: String,
}

#[derive(Debug, PartialEq, Eq, Clone, Deserialize, Serialize, Default)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct GameClientResourceInfo {
    pub id: String,
    pub game_type: String,
    pub release_time: String,
    pub url: String,
}

#[derive(Debug, PartialEq, Eq, Clone, Deserialize, Serialize, Default)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ModLoaderResourceInfo {
    pub loader_type: ModLoaderType,
    pub version: String,
    pub description: String,
    pub stable: bool,
    pub branch: Option<String>,
}

#[derive(Debug, PartialEq, Eq, Clone, Deserialize, Serialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct OptiFineResourceInfo {
    pub filename: String,
    pub patch: String,
    pub r#type: String,
}

#[derive(Debug)]
pub enum ResourceError {
    ParseError,
    NoDownloadApi,
    NetworkError,
}

impl std::fmt::Display for ResourceError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ResourceError::ParseError => write!(f, "Parse error"),
            ResourceError::NoDownloadApi => write!(f, "No download API available"),
            ResourceError::NetworkError => write!(f, "Network error"),
        }
    }
}

impl std::error::Error for ResourceError {}