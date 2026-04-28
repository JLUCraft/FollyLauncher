use crate::resource::helpers::misc::get_download_api;
use crate::resource::models::{ModLoaderResourceInfo, ModLoaderType, ResourceError, ResourceType, SourceType};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use tauri::AppHandle;

#[derive(Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
struct FabricMetaItem {
    pub loader: FabricLoaderInfo,
    pub intermediary: Value,
    pub launcher_meta: Value,
}

#[derive(Serialize, Deserialize, Default)]
struct FabricLoaderInfo {
    pub separator: String,
    pub build: i64,
    pub maven: String,
    pub version: String,
    pub stable: bool,
}

pub async fn get_fabric_meta_by_game_version(
    _app: &AppHandle,
    client: &reqwest::Client,
    priority_list: &[SourceType],
    game_version: &str,
) -> Result<Vec<ModLoaderResourceInfo>, ResourceError> {
    for source_type in priority_list.iter() {
        let url = get_download_api(*source_type, ResourceType::FabricMeta)?
            .join("v2/versions/loader/")
            .map_err(|_| ResourceError::ParseError)?
            .join(game_version)
            .map_err(|_| ResourceError::ParseError)?;
        match client.get(url).send().await {
            Ok(response) => {
                if response.status().is_success() {
                    if let Ok(manifest) = response.json::<Vec<FabricMetaItem>>().await {
                        return Ok(
                            manifest
                                .into_iter()
                                .map(|info| ModLoaderResourceInfo {
                                    loader_type: ModLoaderType::Fabric,
                                    version: info.loader.version,
                                    description: String::new(),
                                    stable: info.loader.stable,
                                    branch: None,
                                })
                                .collect(),
                        );
                    } else {
                        return Err(ResourceError::ParseError);
                    }
                } else {
                    continue;
                }
            }
            Err(_) => continue,
        }
    }
    Err(ResourceError::NetworkError)
}
