use crate::resource::helpers::misc::get_download_api;
use crate::resource::models::{OptiFineResourceInfo, ResourceError, ResourceType, SourceType};
use serde::{Deserialize, Serialize};
use tauri::AppHandle;


#[derive(Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
struct OptiFineMetaItem {
    pub filename: String,
    pub patch: String,
    pub r#type: String,
}

pub async fn get_optifine_meta_by_game_version(
    _app: &AppHandle,
    client: &reqwest::Client,
    priority_list: &[SourceType],
    game_version: &str,
) -> Result<Vec<OptiFineResourceInfo>, ResourceError> {
    for source_type in priority_list.iter() {
        let url = get_download_api(*source_type, ResourceType::OptiFine)
            .map_err(|_| ResourceError::NoDownloadApi)?;
        let full_url = url.join(game_version).map_err(|_| ResourceError::ParseError)?;

        match client.get(full_url).send().await {
            Ok(response) => {
                if response.status().is_success() {
                    if let Ok(manifest) = response.json::<Vec<OptiFineMetaItem>>().await {
                        return Ok(
                            manifest
                                .into_iter()
                                .map(|info| OptiFineResourceInfo {
                                    filename: info.filename,
                                    patch: info.patch,
                                    r#type: info.r#type,
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
