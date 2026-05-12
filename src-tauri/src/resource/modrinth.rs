use crate::resource::misc::{apply_other_resource_enhancements, version_pack_sort};
use crate::resource::mod_db::handle_search_query;
use crate::resource::models::{
    OtherResourceApiEndpoint, OtherResourceDependency, OtherResourceFileInfo, OtherResourceInfo,
    OtherResourceSearchQuery, OtherResourceSearchRes, OtherResourceSource,
    OtherResourceVersionPack, OtherResourceVersionPackQuery, ResourceError,
};
use serde::Deserialize;
use std::collections::HashMap;
use tauri::{AppHandle, Manager};

const ALL_FILTER: &str = "All";



#[derive(Deserialize, Debug)]
pub struct ModrinthProject {
    #[serde(alias = "id")]
    pub project_id: String,
    pub project_type: String,
    pub slug: String,
    pub title: String,
    pub description: String,
    pub categories: Vec<String>,
    pub downloads: u64,
    pub icon_url: Option<String>,
    #[serde(alias = "updated")]
    pub date_modified: String,
    pub author: Option<String>,
}

#[derive(Deserialize, Debug)]
pub struct ModrinthSearchRes {
    pub hits: Vec<ModrinthProject>,
    pub total_hits: u64,
    pub offset: u32,
    pub limit: u32,
}

#[derive(Deserialize, Debug)]
pub struct ModrinthFileHashes {
    pub sha1: String,
}

#[derive(Deserialize, Debug)]
pub struct ModrinthFileInfo {
    pub url: String,
    pub filename: String,
    pub hashes: ModrinthFileHashes,
}

#[derive(Deserialize, Debug)]
pub struct ModrinthDependency {
    pub project_id: Option<String>,
    pub dependency_type: String,
}

#[derive(Deserialize, Debug)]
pub struct ModrinthVersionPack {
    pub project_id: String,
    pub dependencies: Vec<ModrinthDependency>,
    pub game_versions: Vec<String>,
    pub loaders: Vec<String>,
    pub name: String,
    pub date_published: String,
    pub downloads: u64,
    pub version_type: String,
    pub files: Vec<ModrinthFileInfo>,
}

#[derive(Deserialize, Debug)]
pub struct ModrinthTranslationRes {
    pub translated: String,
}



fn append_query_params(
    base_url: &str,
    params: &HashMap<String, String>,
) -> Result<String, ResourceError> {
    let mut url = url::Url::parse(base_url).map_err(|_| ResourceError::ParseError)?;
    {
        let mut pairs = url.query_pairs_mut();
        for (k, v) in params {
            pairs.append_pair(k, v);
        }
    }
    Ok(url.to_string())
}

fn normalize_modrinth_loader(loader: &str) -> Option<String> {
    if loader.is_empty() || loader == "minecraft" {
        None
    } else {
        match loader.to_lowercase().as_str() {
            "forge" => Some("Forge".to_string()),
            "fabric" => Some("Fabric".to_string()),
            "quilt" => Some("Quilt".to_string()),
            "neoforge" => Some("NeoForge".to_string()),
            "vanilla" => Some("Vanilla".to_string()),
            "iris" => Some("Iris".to_string()),
            "canvas" => Some("Canvas".to_string()),
            "optifine" => Some("OptiFine".to_string()),
            _ => Some(loader.to_string()),
        }
    }
}



pub async fn fetch_resource_list_by_name_modrinth(
    app: &AppHandle,
    query: &OtherResourceSearchQuery,
) -> Result<OtherResourceSearchRes, ResourceError> {
    let url = get_modrinth_api(OtherResourceApiEndpoint::Search, None)?;
    let OtherResourceSearchQuery {
        resource_type,
        search_query,
        game_version,
        selected_tag,
        sort_by,
        page,
        page_size,
    } = query;
    let handled_search_query = handle_search_query(app, search_query)
        .await
        .unwrap_or(search_query.clone());
    let mut facets = vec![vec![format!("project_type:{}", resource_type)]];
    if !game_version.is_empty() && game_version != ALL_FILTER {
        facets.push(vec![format!("versions:{}", game_version)]);
    }
    if !selected_tag.is_empty() && selected_tag != ALL_FILTER {
        facets.push(vec![format!("categories:{}", selected_tag)]);
    }
    let mut params = HashMap::new();
    params.insert("query".to_string(), handled_search_query);
    params.insert(
        "facets".to_string(),
        serde_json::to_string(&facets).unwrap_or_default(),
    );
    params.insert("offset".to_string(), (page * page_size).to_string());
    params.insert("limit".to_string(), page_size.to_string());
    params.insert("index".to_string(), sort_by.to_string());
    let final_url = append_query_params(&url, &params)?;
    let client = app.state::<reqwest::Client>();
    let response = client
        .get(&final_url)
        .send()
        .await
        .map_err(|_| ResourceError::NetworkError)?;
    if !response.status().is_success() {
        return Err(ResourceError::NetworkError);
    }
    let results: ModrinthSearchRes = response
        .json()
        .await
        .map_err(|_| ResourceError::ParseError)?;
    let mut search_result: OtherResourceSearchRes = results.into();
    for resource_info in &mut search_result.list {
        let _ = apply_other_resource_enhancements(app, resource_info).await;
    }
    Ok(search_result)
}

pub async fn fetch_resource_version_packs_modrinth(
    app: &AppHandle,
    query: &OtherResourceVersionPackQuery,
) -> Result<Vec<OtherResourceVersionPack>, ResourceError> {
    let OtherResourceVersionPackQuery {
        resource_id,
        mod_loader,
        game_versions,
    } = query;
    let url = get_modrinth_api(OtherResourceApiEndpoint::VersionPack, Some(resource_id))?;
    let mut params = HashMap::new();
    if mod_loader != ALL_FILTER {
        params.insert(
            "loaders".to_string(),
            format!("[\"{}\"]", mod_loader.to_lowercase()),
        );
    }
    if let Some(first_version) = game_versions.first() {
        if first_version != ALL_FILTER {
            let versions_json = format!(
                "[{}]",
                game_versions
                    .iter()
                    .map(|v| format!("\"{}\"", v))
                    .collect::<Vec<_>>()
                    .join(",")
            );
            params.insert("game_versions".to_string(), versions_json);
        }
    }
    let final_url = append_query_params(&url, &params)?;
    let client = app.state::<reqwest::Client>();
    let response = client
        .get(&final_url)
        .send()
        .await
        .map_err(|_| ResourceError::NetworkError)?;
    if !response.status().is_success() {
        return Err(ResourceError::NetworkError);
    }
    let results: Vec<ModrinthVersionPack> = response
        .json()
        .await
        .map_err(|_| ResourceError::ParseError)?;
    Ok(map_modrinth_file_to_version_pack(results))
}

pub async fn fetch_remote_resource_by_local_modrinth(
    app: &AppHandle,
    file_path: &str,
) -> Result<OtherResourceFileInfo, ResourceError> {
    let file_content = tokio::fs::read(file_path)
        .await
        .map_err(|_| ResourceError::ParseError)?;
    let hash_string = hex::encode(sha1_smol::Sha1::from(&file_content).digest().bytes());
    let url = get_modrinth_api(OtherResourceApiEndpoint::FromLocal, Some(&hash_string))?;
    let mut params = HashMap::new();
    params.insert("algorithm".to_string(), "sha1".to_string());
    let final_url = append_query_params(&url, &params)?;
    let client = app.state::<reqwest::Client>();
    let response = client
        .get(&final_url)
        .send()
        .await
        .map_err(|_| ResourceError::NetworkError)?;
    if !response.status().is_success() {
        return Err(ResourceError::NetworkError);
    }
    let version_pack: ModrinthVersionPack = response
        .json()
        .await
        .map_err(|_| ResourceError::ParseError)?;
    let file_info = version_pack
        .files
        .iter()
        .find(|file| file.hashes.sha1 == hash_string)
        .ok_or(ResourceError::ParseError)?;
    Ok((
        &version_pack,
        file_info,
        if version_pack.loaders.is_empty() {
            None
        } else {
            Some(version_pack.loaders[0].clone())
        },
    )
        .into())
}

pub async fn fetch_remote_resource_by_id_modrinth(
    app: &AppHandle,
    resource_id: &str,
) -> Result<OtherResourceInfo, ResourceError> {
    let url = get_modrinth_api(OtherResourceApiEndpoint::ById, Some(resource_id))?;
    let client = app.state::<reqwest::Client>();
    let response = client
        .get(&url)
        .send()
        .await
        .map_err(|_| ResourceError::NetworkError)?;
    if !response.status().is_success() {
        return Err(ResourceError::NetworkError);
    }
    let results: ModrinthProject = response
        .json()
        .await
        .map_err(|_| ResourceError::ParseError)?;
    let mut resource_info: OtherResourceInfo = results.into();
    let _ = apply_other_resource_enhancements(app, &mut resource_info).await;
    Ok(resource_info)
}



pub fn map_modrinth_file_to_version_pack(
    res: Vec<ModrinthVersionPack>,
) -> Vec<OtherResourceVersionPack> {
    let mut version_packs: HashMap<String, OtherResourceVersionPack> = HashMap::new();

    for version in res {
        let game_versions = if version.game_versions.is_empty() {
            vec!["".to_string()]
        } else {
            version.game_versions.clone()
        };

        const ALLOWED_LOADERS: &[&str] = &[
            "forge",
            "fabric",
            "quilt",
            "neoforge",
            "vanilla",
            "iris",
            "canvas",
            "optifine",
            "minecraft",
        ];

        let loaders = if version.loaders.is_empty() {
            vec!["".to_string()]
        } else {
            version
                .loaders
                .iter()
                .filter(|loader| ALLOWED_LOADERS.contains(&loader.as_str()))
                .cloned()
                .collect::<Vec<_>>()
        };

        for game_version in &game_versions {
            for loader in &loaders {
                let file_infos = version
                    .files
                    .iter()
                    .map(|file| (&version, file, normalize_modrinth_loader(loader)).into())
                    .collect::<Vec<_>>();

                version_packs
                    .entry(game_version.clone())
                    .or_insert_with(|| OtherResourceVersionPack {
                        name: game_version.clone(),
                        items: Vec::new(),
                    })
                    .items
                    .extend(file_infos);
            }
        }
    }

    let mut list: Vec<OtherResourceVersionPack> = version_packs.into_values().collect();
    list.sort_by(version_pack_sort);

    list
}



impl From<ModrinthProject> for OtherResourceInfo {
    fn from(project: ModrinthProject) -> Self {
        Self {
            id: project.project_id,
            mcmod_id: 0,
            _type: project.project_type,
            name: project.title,
            slug: project.slug.to_string(),
            description: project.description,
            icon_src: project.icon_url.unwrap_or_default(),
            website_url: format!("https://modrinth.com/mod/{}", project.slug),
            tags: project.categories,
            last_updated: project.date_modified,
            downloads: project.downloads,
            source: OtherResourceSource::Modrinth,
            translated_name: None,
            translated_description: None,
            author: project.author,
        }
    }
}

impl From<(&ModrinthVersionPack, &ModrinthFileInfo, Option<String>)> for OtherResourceFileInfo {
    fn from(
        (version, file, loader): (&ModrinthVersionPack, &ModrinthFileInfo, Option<String>),
    ) -> Self {
        Self {
            resource_id: version.project_id.clone(),
            name: version.name.clone(),
            release_type: version.version_type.clone(),
            downloads: version.downloads,
            file_date: version.date_published.clone(),
            download_url: file.url.clone(),
            sha1: file.hashes.sha1.clone(),
            file_name: file.filename.clone(),
            dependencies: version
                .dependencies
                .iter()
                .map(|d| OtherResourceDependency {
                    resource_id: d.project_id.clone().unwrap_or_default(),
                    relation: d.dependency_type.clone(),
                })
                .collect(),
            loader,
        }
    }
}

impl From<ModrinthSearchRes> for OtherResourceSearchRes {
    fn from(res: ModrinthSearchRes) -> Self {
        let list = res.hits.into_iter().map(OtherResourceInfo::from).collect();

        Self {
            list,
            total: res.total_hits,
            page: res.offset / res.limit,
            page_size: res.limit,
        }
    }
}



pub fn get_modrinth_api(
    endpoint: OtherResourceApiEndpoint,
    param: Option<&str>,
) -> Result<String, ResourceError> {
    let base_url = "https://api.modrinth.com/v2";

    let url_str = match endpoint {
        OtherResourceApiEndpoint::Search => format!("{}/search", base_url),
        OtherResourceApiEndpoint::VersionPack => {
            let project_id = param.ok_or(ResourceError::ParseError)?;
            format!("{}/project/{}/version", base_url, project_id)
        }
        OtherResourceApiEndpoint::FromLocal => {
            let hash = param.ok_or(ResourceError::ParseError)?;
            format!("{}/version_file/{}", base_url, hash)
        }
        OtherResourceApiEndpoint::ById => {
            let project_id = param.ok_or(ResourceError::ParseError)?;
            format!("{}/project/{}", base_url, project_id)
        }
        OtherResourceApiEndpoint::TranslateDesc => {
            let project_id = param.ok_or(ResourceError::ParseError)?;
            format!(
                "https://mod.mcimirror.top/translate/modrinth/{}",
                project_id
            )
        }
    };

    Ok(url_str)
}



pub async fn translate_description_modrinth(
    app: &AppHandle,
    resource_id: &str,
) -> Result<Option<String>, ResourceError> {
    let result = async {
        let url = get_modrinth_api(OtherResourceApiEndpoint::TranslateDesc, Some(resource_id))?;
        let client = app.state::<reqwest::Client>();

        let translation_res = client
            .get(&url)
            .send()
            .await
            .map_err(|_| ResourceError::NetworkError)?
            .json::<ModrinthTranslationRes>()
            .await
            .map_err(|_| ResourceError::ParseError)?;

        Ok::<_, ResourceError>(translation_res.translated)
    }
    .await;

    Ok(result.ok())
}
