use crate::resource::helpers::misc::get_download_api;
use crate::resource::models::{ModLoaderResourceInfo, ModLoaderType, ResourceError, ResourceType, SourceType};
use serde::{Deserialize, Serialize};
use tauri::AppHandle;

#[derive(Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
struct NeoforgeMetaItem {
    pub raw_version: String,
    pub version: String,
    pub mcversion: String,
}

async fn get_neoforge_meta_by_game_version_official(
    _app: &AppHandle,
    client: &reqwest::Client,
    game_version: &str,
) -> Result<Vec<ModLoaderResourceInfo>, ResourceError> {
    if game_version == "1.20.1" {
        let url = get_download_api(SourceType::Official, ResourceType::NeoforgeMetaForge)?;
        let response = client
            .get(url)
            .send()
            .await
            .map_err(|_| ResourceError::NetworkError)?;
        if !response.status().is_success() {
            return Err(ResourceError::NetworkError);
        }

        let versions: serde_json::Value = response
            .json()
            .await
            .map_err(|_| ResourceError::ParseError)?;
        let Some(version_list) = versions.get("versions").and_then(|v| v.as_array()) else {
            return Err(ResourceError::ParseError);
        };

        let mut results = Vec::new();
        for version_value in version_list {
            if let Some(version) = version_value.as_str() {
                if let Some(ver) = version.strip_prefix("1.20.1-") {
                    let parts: Vec<&str> = ver.split('.').collect();
                    if parts.len() >= 3 {
                        results.push(ModLoaderResourceInfo {
                            loader_type: ModLoaderType::NeoForge,
                            version: version.to_string(),
                            description: String::new(),
                            stable: versions
                                .get("is_snapshot")
                                .is_none_or(|v| !v.as_bool().unwrap_or(false)),
                            branch: None,
                        });
                    }
                }
            }
        }
        return Ok(results);
    }

    let url = get_download_api(SourceType::Official, ResourceType::NeoforgeMetaNeoforge)?;
    let response = client
        .get(url)
        .send()
        .await
        .map_err(|_| ResourceError::NetworkError)?;
    if !response.status().is_success() {
        return Err(ResourceError::NetworkError);
    }

    let versions: serde_json::Value = response
        .json()
        .await
        .map_err(|_| ResourceError::ParseError)?;
    let Some(version_list) = versions.get("versions").and_then(|v| v.as_array()) else {
        return Err(ResourceError::ParseError);
    };

    let mut results: Vec<ModLoaderResourceInfo> = Vec::new();
    for version_value in version_list {
        if let Some(version) = version_value.as_str() {
            results.push(ModLoaderResourceInfo {
                loader_type: ModLoaderType::NeoForge,
                version: version.to_string(),
                description: String::new(),
                stable: !version.contains("beta") && !version.contains("alpha"),
                branch: None,
            });
        }
    }

    Ok(results)
}

async fn get_neoforge_meta_by_game_version_bmcl(
    _app: &AppHandle,
    client: &reqwest::Client,
    game_version: &str,
) -> Result<Vec<ModLoaderResourceInfo>, ResourceError> {
    let url = get_download_api(
        SourceType::BMCLAPIMirror,
        ResourceType::NeoforgeMetaNeoforge,
    )?
    .join("list/")
    .map_err(|_| ResourceError::ParseError)?
    .join(game_version)
    .map_err(|_| ResourceError::ParseError)?;

    match client.get(url).send().await {
        Ok(response) => {
            if response.status().is_success() {
                if let Ok(mut manifest) = response.json::<Vec<NeoforgeMetaItem>>().await {
                    manifest.sort_by(|a, b| {
                        let parse_version = |v: &str| {
                            let stripped = if game_version == "1.20.1" {
                                v.strip_prefix("1.20.1-").unwrap_or(v)
                            } else {
                                v
                            };
                            stripped
                                .split('.')
                                .flat_map(|part| part.split('-'))
                                .flat_map(|part| part.split('+'))
                                .map(|s| s.parse::<i32>().unwrap_or(0))
                                .collect::<Vec<_>>()
                        };
                        parse_version(&b.version).cmp(&parse_version(&a.version))
                    });
                    Ok(
                        manifest
                            .into_iter()
                            .map(|info| {
                                let version = info.version;
                                let stable = !version.contains("beta") && !version.contains("alpha");
                                ModLoaderResourceInfo {
                                    loader_type: ModLoaderType::NeoForge,
                                    version,
                                    description: String::new(),
                                    stable,
                                    branch: None,
                                }
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

pub async fn get_neoforge_meta_by_game_version(
    app: &AppHandle,
    client: &reqwest::Client,
    priority_list: &[SourceType],
    game_version: &str,
) -> Result<Vec<ModLoaderResourceInfo>, ResourceError> {
    for source_type in priority_list.iter() {
        match *source_type {
            SourceType::Official => {
                if let Ok(meta) = get_neoforge_meta_by_game_version_official(app, client, game_version).await {
                    return Ok(meta);
                }
            }
            SourceType::BMCLAPIMirror => {
                if let Ok(meta) = get_neoforge_meta_by_game_version_bmcl(app, client, game_version).await {
                    return Ok(meta);
                }
            }
        }
    }
    Err(ResourceError::NetworkError)
}
