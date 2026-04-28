use crate::resource::helpers::misc::get_download_api;
use crate::resource::models::{ModLoaderResourceInfo, ModLoaderType, ResourceError, ResourceType, SourceType};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use tauri::AppHandle;

#[derive(Serialize, Deserialize, Default)]
struct ForgeMetaItem {
    pub branch: Option<Value>,
    pub build: i64,
    pub files: Vec<Value>,
    pub mcversion: String,
    pub modified: String,
    pub version: String,
}

async fn get_forge_meta_by_game_version_bmcl(
    _app: &AppHandle,
    client: &reqwest::Client,
    game_version: &str,
) -> Result<Vec<ModLoaderResourceInfo>, ResourceError> {
    let url = get_download_api(SourceType::BMCLAPIMirror, ResourceType::ForgeMeta)?
        .join("minecraft/")
        .map_err(|_| ResourceError::ParseError)?
        .join(game_version)
        .map_err(|_| ResourceError::ParseError)?;
    match client.get(url).send().await {
        Ok(response) => {
            if response.status().is_success() {
                if let Ok(mut manifest) = response.json::<Vec<ForgeMetaItem>>().await {
                    manifest.sort_by(|a, b| b.build.cmp(&a.build));
                    Ok(
                        manifest
                            .into_iter()
                            .map(|info| ModLoaderResourceInfo {
                                loader_type: ModLoaderType::Forge,
                                version: info.version,
                                description: info.modified,
                                stable: true,
                                branch: info.branch.and_then(|v| v.as_str().map(String::from)),
                            })
                            .collect(),
                    )
                } else {
                    Err(ResourceError::ParseError)
                }
            } else {
                Err(ResourceError::NetworkError)
            }
        }
        Err(_) => Err(ResourceError::NetworkError),
    }
}

pub async fn get_forge_meta_by_game_version(
    app: &AppHandle,
    client: &reqwest::Client,
    priority_list: &[SourceType],
    game_version: &str,
) -> Result<Vec<ModLoaderResourceInfo>, ResourceError> {
    for source_type in priority_list.iter() {
        match *source_type {
            SourceType::BMCLAPIMirror => {
                if let Ok(meta) = get_forge_meta_by_game_version_bmcl(app, client, game_version).await {
                    return Ok(meta);
                }
            }
            _ => continue,
        }
    }
    Err(ResourceError::NoDownloadApi)
}
