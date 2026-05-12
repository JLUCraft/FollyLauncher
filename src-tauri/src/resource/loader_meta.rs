use crate::resource::misc::get_download_api;
use crate::resource::models::{
    ModLoaderResourceInfo, ModLoaderType, OptiFineResourceInfo, ResourceError, ResourceType,
    SourceType,
};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use tauri::AppHandle;



#[derive(Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct FabricMetaItem {
    pub loader: FabricLoaderInfo,
    pub intermediary: Value,
    pub launcher_meta: Value,
}

#[derive(Serialize, Deserialize, Default)]
pub struct FabricLoaderInfo {
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
            Ok(response) if response.status().is_success() => {
                let manifest = response
                    .json::<Vec<FabricMetaItem>>()
                    .await
                    .map_err(|_| ResourceError::ParseError)?;
                return Ok(manifest
                    .into_iter()
                    .map(|info| ModLoaderResourceInfo {
                        loader_type: ModLoaderType::Fabric,
                        version: info.loader.version,
                        description: String::new(),
                        stable: info.loader.stable,
                        branch: None,
                    })
                    .collect());
            }
            _ => continue,
        }
    }
    Err(ResourceError::NetworkError)
}



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
        Ok(response) if response.status().is_success() => {
            let mut manifest = response
                .json::<Vec<ForgeMetaItem>>()
                .await
                .map_err(|_| ResourceError::ParseError)?;
            manifest.sort_by_key(|b| std::cmp::Reverse(b.build));
            Ok(manifest
                .into_iter()
                .map(|info| ModLoaderResourceInfo {
                    loader_type: ModLoaderType::Forge,
                    version: info.version,
                    description: info.modified,
                    stable: true,
                    branch: info.branch.and_then(|v| v.as_str().map(String::from)),
                })
                .collect())
        }
        _ => Err(ResourceError::NetworkError),
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
                if let Ok(meta) =
                    get_forge_meta_by_game_version_bmcl(app, client, game_version).await
                {
                    return Ok(meta);
                }
            }
            _ => continue,
        }
    }
    Err(ResourceError::NoDownloadApi)
}



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
        let versions: Value = response
            .json()
            .await
            .map_err(|_| ResourceError::ParseError)?;
        let Some(version_list) = versions.get("versions").and_then(|v| v.as_array()) else {
            return Err(ResourceError::ParseError);
        };
        let mut results = Vec::new();
        for v in version_list {
            if let Some(version) = v.as_str() {
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
    let versions: Value = response
        .json()
        .await
        .map_err(|_| ResourceError::ParseError)?;
    let Some(version_list) = versions.get("versions").and_then(|v| v.as_array()) else {
        return Err(ResourceError::ParseError);
    };
    let mut results = Vec::new();
    for v in version_list {
        if let Some(version) = v.as_str() {
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
        Ok(response) if response.status().is_success() => {
            let mut manifest = response
                .json::<Vec<NeoforgeMetaItem>>()
                .await
                .map_err(|_| ResourceError::ParseError)?;
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
            Ok(manifest
                .into_iter()
                .map(|info| {
                    let stable = !info.version.contains("beta") && !info.version.contains("alpha");
                    ModLoaderResourceInfo {
                        loader_type: ModLoaderType::NeoForge,
                        version: info.version,
                        description: String::new(),
                        stable,
                        branch: None,
                    }
                })
                .collect())
        }
        _ => Err(ResourceError::NetworkError),
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
                if let Ok(meta) =
                    get_neoforge_meta_by_game_version_official(app, client, game_version).await
                {
                    return Ok(meta);
                }
            }
            SourceType::BMCLAPIMirror => {
                if let Ok(meta) =
                    get_neoforge_meta_by_game_version_bmcl(app, client, game_version).await
                {
                    return Ok(meta);
                }
            }
        }
    }
    Err(ResourceError::NetworkError)
}



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
        let url = get_download_api(*source_type, ResourceType::OptiFine)?;
        let full_url = url
            .join(game_version)
            .map_err(|_| ResourceError::ParseError)?;
        match client.get(full_url).send().await {
            Ok(response) if response.status().is_success() => {
                let manifest = response
                    .json::<Vec<OptiFineMetaItem>>()
                    .await
                    .map_err(|_| ResourceError::ParseError)?;
                return Ok(manifest
                    .into_iter()
                    .map(|info| OptiFineResourceInfo {
                        filename: info.filename,
                        patch: info.patch,
                        r#type: info.r#type,
                    })
                    .collect());
            }
            _ => continue,
        }
    }
    Err(ResourceError::NetworkError)
}



#[derive(Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct QuiltMetaItem {
    pub loader: QuiltLoaderInfo,
    pub intermediary: Value,
}

#[derive(Serialize, Deserialize, Default)]
pub struct QuiltLoaderInfo {
    pub version: String,
}

pub async fn get_quilt_meta_by_game_version(
    _app: &AppHandle,
    client: &reqwest::Client,
    priority_list: &[SourceType],
    game_version: &str,
) -> Result<Vec<ModLoaderResourceInfo>, ResourceError> {
    for source_type in priority_list.iter() {
        let url = get_download_api(*source_type, ResourceType::QuiltMeta)?
            .join("v3/versions/loader/")
            .map_err(|_| ResourceError::ParseError)?
            .join(game_version)
            .map_err(|_| ResourceError::ParseError)?;
        match client.get(url).send().await {
            Ok(response) if response.status().is_success() => {
                let manifest = response
                    .json::<Vec<QuiltMetaItem>>()
                    .await
                    .map_err(|_| ResourceError::ParseError)?;
                return Ok(manifest
                    .into_iter()
                    .map(|info| ModLoaderResourceInfo {
                        loader_type: ModLoaderType::Quilt,
                        version: info.loader.version,
                        description: String::new(),
                        stable: true,
                        branch: None,
                    })
                    .collect());
            }
            _ => continue,
        }
    }
    Err(ResourceError::NetworkError)
}
