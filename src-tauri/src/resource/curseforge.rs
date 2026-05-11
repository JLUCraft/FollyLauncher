use crate::resource::misc::{apply_other_resource_enhancements, version_pack_sort};
use crate::resource::mod_db::handle_search_query;
use crate::resource::models::{
    OtherResourceApiEndpoint, OtherResourceDependency, OtherResourceFileInfo, OtherResourceInfo,
    OtherResourceSearchQuery, OtherResourceSearchRes, OtherResourceSource,
    OtherResourceVersionPack, OtherResourceVersionPackQuery, ResourceError,
};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use tauri::{AppHandle, Manager};

// ── Constants ──────────────────────────────────────────────────────────────

const MINECRAFT_GAME_ID: &str = "432";
const ALL_FILTER: &str = "All";
const WORD_PERFECT_MATCH_WEIGHT: usize = 10;
const CURSEFORGE_API_KEY_ENV: &str = "FOLLY_CURSEFORGE_API_KEY";

fn get_cf_api_key() -> String {
    std::env::var(CURSEFORGE_API_KEY_ENV).unwrap_or_default()
}

// ── API types ──────────────────────────────────────────────────────────────

#[derive(Deserialize, Serialize, Debug, Clone)]
#[serde(rename_all = "camelCase")]
pub struct CurseForgeProjectLink {
    pub website_url: String,
}

#[derive(Deserialize, Serialize, Debug, Clone)]
#[serde(rename_all = "camelCase")]
pub struct CurseForgeCategory {
    pub name: String,
}

#[derive(Deserialize, Serialize, Debug, Clone)]
#[serde(rename_all = "camelCase")]
pub struct CurseForgeLogo {
    pub url: String,
}

#[derive(Deserialize, Serialize, Debug, Clone)]
#[serde(rename_all = "camelCase")]
pub struct CurseForgeAuthor {
    pub name: String,
}

#[derive(Deserialize, Serialize, Debug, Clone)]
#[serde(rename_all = "camelCase")]
pub struct CurseForgeProject {
    pub id: i32,
    pub class_id: Option<i32>,
    pub links: CurseForgeProjectLink,
    pub name: String,
    pub slug: String,
    pub summary: String,
    pub categories: Vec<CurseForgeCategory>,
    pub download_count: u64,
    pub logo: Option<CurseForgeLogo>,
    pub date_modified: String,
    pub authors: Vec<CurseForgeAuthor>,
}

#[derive(Deserialize, Serialize, Debug, Clone)]
#[serde(rename_all = "camelCase")]
pub struct CurseForgePagination {
    pub index: u32,
    pub page_size: u32,
    pub total_count: u64,
}

#[derive(Deserialize, Serialize, Debug, Clone)]
pub struct CurseForgeSearchRes {
    pub data: Vec<CurseForgeProject>,
    pub pagination: CurseForgePagination,
}

#[derive(Deserialize, Serialize, Debug, Clone)]
#[serde(rename_all = "camelCase")]
pub struct CurseForgeFileHash {
    pub value: String,
    pub algo: u32,
}

#[derive(Deserialize, Serialize, Debug, Clone)]
#[serde(rename_all = "camelCase")]
pub struct CurseForgeFileDependency {
    pub mod_id: i32,
    pub relation_type: u32,
}

#[derive(Deserialize, Serialize, Debug, Clone)]
#[serde(rename_all = "camelCase")]
pub struct CurseForgeFileInfo {
    pub id: i32,
    pub mod_id: i32,
    pub display_name: String,
    pub file_name: String,
    pub release_type: u32,
    pub hashes: Vec<CurseForgeFileHash>,
    pub file_date: String,
    pub download_url: Option<String>,
    pub download_count: u64,
    pub game_versions: Vec<String>,
    pub dependencies: Vec<CurseForgeFileDependency>,
}

#[derive(Deserialize, Serialize, Debug, Clone)]
pub struct CurseForgeVersionPackSearchRes {
    pub data: Vec<CurseForgeFileInfo>,
    pub pagination: CurseForgePagination,
}

#[derive(Deserialize, Serialize, Debug, Clone)]
#[serde(rename_all = "camelCase")]
pub struct CurseForgeExactMatch {
    pub file: CurseForgeFileInfo,
}

#[derive(Deserialize, Serialize, Debug, Clone)]
#[serde(rename_all = "camelCase")]
pub struct CurseForgeFingerprintData {
    pub exact_matches: Vec<CurseForgeExactMatch>,
}

#[derive(Deserialize, Serialize, Debug, Clone)]
#[serde(rename_all = "camelCase")]
pub struct CurseForgeFingerprintRes {
    pub data: CurseForgeFingerprintData,
}

#[derive(Deserialize, Serialize, Debug, Clone)]
#[serde(rename_all = "camelCase")]
pub struct CurseForgeGetProjectRes {
    pub data: CurseForgeProject,
}

#[derive(Deserialize, Debug)]
pub struct CurseForgeTranslationRes {
    pub translated: String,
}

// ── HTTP helpers ───────────────────────────────────────────────────────────

async fn cf_get<T: serde::de::DeserializeOwned>(
    client: &reqwest::Client,
    url: &str,
    params: Option<&HashMap<String, String>>,
) -> Result<T, ResourceError> {
    let key = get_cf_api_key();
    let full_url = if let Some(p) = params {
        let mut parsed = url::Url::parse(url).map_err(|_| ResourceError::NetworkError)?;
        parsed
            .query_pairs_mut()
            .extend_pairs(p.iter().map(|(k, v)| (k.as_str(), v.as_str())));
        parsed.to_string()
    } else {
        url.to_string()
    };
    let response = client
        .get(&full_url)
        .header("x-api-key", &key)
        .send()
        .await
        .map_err(|_| ResourceError::NetworkError)?;
    if !response.status().is_success() {
        return Err(ResourceError::NetworkError);
    }
    response
        .json::<T>()
        .await
        .map_err(|_| ResourceError::ParseError)
}

async fn cf_post<T: serde::de::DeserializeOwned, P: serde::Serialize>(
    client: &reqwest::Client,
    url: &str,
    payload: &P,
) -> Result<T, ResourceError> {
    let key = get_cf_api_key();
    let response = client
        .post(url)
        .header("x-api-key", &key)
        .json(payload)
        .send()
        .await
        .map_err(|_| ResourceError::NetworkError)?;
    if !response.status().is_success() {
        return Err(ResourceError::NetworkError);
    }
    response
        .json::<T>()
        .await
        .map_err(|_| ResourceError::ParseError)
}

// ── Search helpers ─────────────────────────────────────────────────────────

fn tokenize_words(text: &str) -> impl Iterator<Item = &str> {
    text.split(|c: char| !c.is_alphanumeric())
        .filter(|token| !token.is_empty())
}

fn levenshtein_distance(a: &str, b: &str) -> usize {
    let b_chars: Vec<char> = b.chars().collect();
    let mut prev: Vec<usize> = (0..=b_chars.len()).collect();
    for (i, a_ch) in a.chars().enumerate() {
        let mut current = Vec::with_capacity(b_chars.len() + 1);
        current.push(i + 1);
        for (j, b_ch) in b_chars.iter().enumerate() {
            let cost = if a_ch == *b_ch { 0 } else { 1 };
            let insertion = current[j] + 1;
            let deletion = prev[j + 1] + 1;
            let substitution = prev[j] + cost;
            current.push(insertion.min(deletion).min(substitution));
        }
        prev = current;
    }
    *prev.last().unwrap_or(&0)
}

// ── Fetch functions ────────────────────────────────────────────────────────

pub async fn fetch_resource_list_by_name_curseforge(
    app: &AppHandle,
    query: &OtherResourceSearchQuery,
) -> Result<OtherResourceSearchRes, ResourceError> {
    let url = get_curseforge_api(OtherResourceApiEndpoint::Search, None)?;
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
    let class_id = cvt_type_to_class_id(resource_type);
    let sort_field = cvt_sort_by_to_id(sort_by);
    let sort_order = match sort_field {
        4 => "asc",
        _ => "desc",
    };
    let mut params = HashMap::new();
    params.insert("gameId".to_string(), MINECRAFT_GAME_ID.to_string());
    params.insert("classId".to_string(), class_id.to_string());
    params.insert("searchFilter".to_string(), handled_search_query.clone());
    if game_version != ALL_FILTER {
        params.insert("gameVersion".to_string(), game_version.to_string());
    }
    if selected_tag != ALL_FILTER {
        params.insert(
            "categoryId".to_string(),
            cvt_category_to_id(selected_tag, class_id).to_string(),
        );
    }
    params.insert("sortField".to_string(), sort_field.to_string());
    params.insert("sortOrder".to_string(), sort_order.to_string());
    params.insert("index".to_string(), (page * page_size).to_string());
    params.insert("pageSize".to_string(), page_size.to_string());
    let client = app.state::<reqwest::Client>();
    let results: CurseForgeSearchRes = cf_get(&client, &url, Some(&params)).await?;
    let mut search_result: OtherResourceSearchRes = results.into();
    let lower_case_search_filter = handled_search_query.to_lowercase();
    let mut search_filter_words = HashMap::new();
    for token in tokenize_words(&lower_case_search_filter) {
        *search_filter_words
            .entry(token.to_string())
            .or_insert(0usize) += 1;
    }
    let mut scored_results: Vec<(OtherResourceInfo, i64)> = search_result
        .list
        .into_iter()
        .map(|resource| {
            let title = resource
                .translated_name
                .as_deref()
                .unwrap_or(resource.name.as_str());
            let lower_case_result = title.to_lowercase();
            let mut diff =
                levenshtein_distance(&lower_case_search_filter, &lower_case_result) as i64;
            for token in tokenize_words(&lower_case_result) {
                if let Some(count) = search_filter_words.get(token) {
                    diff -= (WORD_PERFECT_MATCH_WEIGHT * *count * token.len()) as i64;
                }
            }
            (resource, diff)
        })
        .collect();
    scored_results.sort_by_key(|(_, diff)| *diff);
    search_result.list = scored_results
        .into_iter()
        .map(|(resource, _)| resource)
        .collect();
    for resource_info in &mut search_result.list {
        let _ = apply_other_resource_enhancements(app, resource_info).await;
    }
    Ok(search_result)
}

pub async fn fetch_resource_version_packs_curseforge(
    app: &AppHandle,
    query: &OtherResourceVersionPackQuery,
) -> Result<Vec<OtherResourceVersionPack>, ResourceError> {
    let mut aggregated_files: Vec<CurseForgeFileInfo> = Vec::new();
    let mut page_idx: u32 = 0;
    let page_size: u32 = 50;
    let OtherResourceVersionPackQuery {
        resource_id,
        mod_loader,
        game_versions,
    } = query;
    loop {
        let url = get_curseforge_api(OtherResourceApiEndpoint::VersionPack, Some(resource_id))?;
        let mut params = HashMap::new();
        if mod_loader != ALL_FILTER {
            params.insert(
                "modLoaderType".to_string(),
                cvt_mod_loader_to_id(mod_loader).to_string(),
            );
        }
        if let Some(version) = game_versions.first() {
            if version != ALL_FILTER {
                params.insert(
                    "gameVersionTypeId".to_string(),
                    cvt_version_to_type_id(version).to_string(),
                );
            }
        }
        params.insert("index".to_string(), (page_idx * page_size).to_string());
        params.insert("pageSize".to_string(), page_size.to_string());
        let client = app.state::<reqwest::Client>();
        let results: CurseForgeVersionPackSearchRes = cf_get(&client, &url, Some(&params)).await?;
        let has_more = results.pagination.total_count > ((page_idx + 1) * page_size) as u64;
        aggregated_files.extend(results.data);
        if !has_more {
            break;
        }
        page_idx += 1;
    }
    Ok(map_curseforge_file_to_version_pack(aggregated_files))
}

pub async fn fetch_remote_resource_by_local_curseforge(
    app: &AppHandle,
    file_path: &str,
) -> Result<OtherResourceFileInfo, ResourceError> {
    let file_content = tokio::fs::read(file_path)
        .await
        .map_err(|_| ResourceError::ParseError)?;
    let local_sha1 = hex::encode(sha1_smol::Sha1::from(&file_content).digest().bytes());
    let filtered_bytes: Vec<u8> = file_content
        .into_iter()
        .filter(|&byte| !matches!(byte, 0x09 | 0x0a | 0x0d | 0x20))
        .collect();
    let hash = murmur2::murmur2(&filtered_bytes, 1) as u64;
    let url = get_curseforge_api(OtherResourceApiEndpoint::FromLocal, None)?;
    let payload = serde_json::json!({ "fingerprints": [hash] });
    let client = app.state::<reqwest::Client>();
    let fp_res: CurseForgeFingerprintRes = cf_post(&client, &url, &payload).await?;
    if let Some(exact_match) = fp_res.data.exact_matches.first() {
        let cf_file = &exact_match.file;
        if let Some(remote_sha1) = cf_file.hashes.iter().find(|h| h.algo == 1) {
            if remote_sha1.value.to_lowercase() == local_sha1.to_lowercase() {
                Ok((cf_file, None).into())
            } else {
                Err(ResourceError::ParseError)
            }
        } else {
            Err(ResourceError::ParseError)
        }
    } else {
        Err(ResourceError::ParseError)
    }
}

pub async fn fetch_remote_resource_by_id_curseforge(
    app: &AppHandle,
    resource_id: &str,
) -> Result<OtherResourceInfo, ResourceError> {
    let url = get_curseforge_api(OtherResourceApiEndpoint::ById, Some(resource_id))?;
    let client = app.state::<reqwest::Client>();
    let result: CurseForgeGetProjectRes = cf_get(&client, &url, None).await?;
    let mut resource_info: OtherResourceInfo = result.data.into();
    let _ = apply_other_resource_enhancements(app, &mut resource_info).await;
    Ok(resource_info)
}

// ── Version pack mapping ───────────────────────────────────────────────────

fn extract_versions_and_loaders(game_versions: &[String]) -> (Vec<String>, Vec<String>) {
    let mut versions = Vec::new();
    let mut loaders = Vec::new();

    const ALLOWED_LOADERS: &[&str] = &[
        "Forge", "Fabric", "Quilt", "NeoForge", "Vanilla", "Iris", "Canvas", "OptiFine",
    ];

    for v in game_versions {
        if v.starts_with(|c: char| c.is_ascii_digit()) {
            versions.push(v.clone());
        } else if ALLOWED_LOADERS.contains(&v.as_str()) {
            loaders.push(v.clone());
        }
    }

    (versions, loaders)
}

pub fn map_curseforge_file_to_version_pack(
    res: Vec<CurseForgeFileInfo>,
) -> Vec<OtherResourceVersionPack> {
    let mut version_packs: HashMap<String, OtherResourceVersionPack> = HashMap::new();

    for cf_file in res {
        let (versions, loaders) = extract_versions_and_loaders(&cf_file.game_versions);

        let versions = if versions.is_empty() {
            vec!["".to_string()]
        } else {
            versions
        };

        let loaders: Vec<&str> = if loaders.is_empty() {
            vec![""]
        } else {
            loaders.iter().map(|s| s.as_str()).collect()
        };

        for version in &versions {
            for loader in &loaders {
                let file_info = (
                    &cf_file,
                    if loader.is_empty() {
                        None
                    } else {
                        Some(loader.to_string())
                    },
                )
                    .into();

                version_packs
                    .entry(version.clone())
                    .or_insert_with(|| OtherResourceVersionPack {
                        name: version.clone(),
                        items: Vec::new(),
                    })
                    .items
                    .push(file_info);
            }
        }
    }

    let mut list: Vec<OtherResourceVersionPack> = version_packs.into_values().collect();
    list.sort_by(version_pack_sort);

    list
}

// ── From impls ─────────────────────────────────────────────────────────────

impl From<CurseForgeProject> for OtherResourceInfo {
    fn from(project: CurseForgeProject) -> Self {
        Self {
            id: project.id.to_string(),
            mcmod_id: 0,
            _type: cvt_class_id_to_type(project.class_id.unwrap_or(0)),
            name: project.name,
            slug: project.slug,
            description: project.summary,
            icon_src: project.logo.map_or("".to_string(), |logo| logo.url),
            website_url: project.links.website_url,
            tags: project.categories.iter().map(|c| c.name.clone()).collect(),
            last_updated: project.date_modified,
            downloads: project.download_count,
            source: OtherResourceSource::CurseForge,
            translated_name: None,
            translated_description: None,
            author: project.authors.first().map(|author| author.name.clone()),
        }
    }
}

impl From<(&CurseForgeFileInfo, Option<String>)> for OtherResourceFileInfo {
    fn from((cf_file, loader): (&CurseForgeFileInfo, Option<String>)) -> Self {
        Self {
            resource_id: cf_file.mod_id.to_string(),
            name: cf_file.display_name.clone(),
            release_type: cvt_id_to_release_type(cf_file.release_type),
            downloads: cf_file.download_count,
            file_date: cf_file.file_date.clone(),
            download_url: cf_file.download_url.clone().unwrap_or(format!(
                "https://edge.forgecdn.net/files/{}/{}/{}",
                cf_file.id / 1000,
                cf_file.id % 1000,
                cf_file.file_name.clone()
            )),
            sha1: cf_file
                .hashes
                .iter()
                .find(|h| h.algo == 1)
                .map_or("".to_string(), |h| h.value.clone()),
            file_name: cf_file.file_name.clone(),
            dependencies: cf_file
                .dependencies
                .iter()
                .map(|dep| OtherResourceDependency {
                    resource_id: dep.mod_id.to_string(),
                    relation: cvt_id_to_dependency_type(dep.relation_type),
                })
                .collect(),
            loader,
        }
    }
}

impl From<CurseForgeSearchRes> for OtherResourceSearchRes {
    fn from(res: CurseForgeSearchRes) -> Self {
        let list = res.data.into_iter().map(OtherResourceInfo::from).collect();

        Self {
            list,
            total: res.pagination.total_count,
            page: res.pagination.index / res.pagination.page_size,
            page_size: res.pagination.page_size,
        }
    }
}

// ── API URL builder ────────────────────────────────────────────────────────

pub fn get_curseforge_api(
    endpoint: OtherResourceApiEndpoint,
    id: Option<&str>,
) -> Result<String, ResourceError> {
    let base_url = "https://api.curseforge.com/v1";

    let url_str = match endpoint {
        OtherResourceApiEndpoint::Search => format!("{}/mods/search", base_url),
        OtherResourceApiEndpoint::VersionPack => {
            let mod_id = id.ok_or(ResourceError::ParseError)?;
            format!("{}/mods/{}/files", base_url, mod_id)
        }
        OtherResourceApiEndpoint::FromLocal => format!("{}/fingerprints/432", base_url),
        OtherResourceApiEndpoint::ById => {
            let mod_id = id.ok_or(ResourceError::ParseError)?;
            format!("{}/mods/{}", base_url, mod_id)
        }
        OtherResourceApiEndpoint::TranslateDesc => {
            let mod_id = id.ok_or(ResourceError::ParseError)?;
            format!("https://mod.mcimirror.top/translate/curseforge/{}", mod_id)
        }
    };

    Ok(url_str)
}

// ── Category map ───────────────────────────────────────────────────────────

pub fn cvt_category_to_id(category: &str, class_id: u32) -> u32 {
    let map = get_category_map();
    *map.get(&(category.to_string(), class_id)).unwrap_or(&0)
}

fn get_category_map() -> &'static HashMap<(String, u32), u32> {
    use std::sync::OnceLock;
    static MAP: OnceLock<HashMap<(String, u32), u32>> = OnceLock::new();
    MAP.get_or_init(|| {
        let mut map = HashMap::new();
        // mods
        map.insert(("Food".to_string(), 6), 436);
        map.insert(("Ores and Resources".to_string(), 6), 408);
        map.insert(("Miscellaneous".to_string(), 6), 425);
        map.insert(("Thermal Expansion".to_string(), 6), 427);
        map.insert(("Cosmetic".to_string(), 6), 424);
        map.insert(("Education".to_string(), 6), 5299);
        map.insert(("Buildcraft".to_string(), 6), 432);
        map.insert(("Processing".to_string(), 6), 413);
        map.insert(("Map and Information".to_string(), 6), 423);
        map.insert(("Technology".to_string(), 6), 412);
        map.insert(("Farming".to_string(), 6), 416);
        map.insert(("Structures".to_string(), 6), 409);
        map.insert(("Magic".to_string(), 6), 419);
        map.insert(("Addons".to_string(), 6), 426);
        map.insert(("Dimensions".to_string(), 6), 410);
        map.insert(("Mobs".to_string(), 6), 411);
        map.insert(("Armor, Tools, and Weapons".to_string(), 6), 434);
        map.insert(("Server Utility".to_string(), 6), 435);
        map.insert(("Energy, Fluid, and Item Transport".to_string(), 6), 415);
        map.insert(("World Gen".to_string(), 6), 406);
        map.insert(("Adventure and RPG".to_string(), 6), 422);
        map.insert(("Storage".to_string(), 6), 420);
        map.insert(("Biomes".to_string(), 6), 407);
        map.insert(("API and Library".to_string(), 6), 421);
        map.insert(("Utility & QoL".to_string(), 6), 5191);
        map.insert(("Performance".to_string(), 6), 6814);
        // resource packs
        map.insert(("Photo Realistic".to_string(), 12), 400);
        map.insert(("Traditional".to_string(), 12), 403);
        map.insert(("512x and Higher".to_string(), 12), 398);
        map.insert(("128x".to_string(), 12), 396);
        map.insert(("256x".to_string(), 12), 397);
        map.insert(("64x".to_string(), 12), 395);
        map.insert(("Medieval".to_string(), 12), 402);
        map.insert(("Miscellaneous".to_string(), 12), 405);
        map.insert(("32x".to_string(), 12), 394);
        map.insert(("16x".to_string(), 12), 393);
        map.insert(("Modern".to_string(), 12), 401);
        map.insert(("Mod Support".to_string(), 12), 4465);
        // worlds
        map.insert(("Parkour".to_string(), 17), 251);
        map.insert(("Survival".to_string(), 17), 253);
        map.insert(("Creation".to_string(), 17), 249);
        map.insert(("Game Map".to_string(), 17), 250);
        map.insert(("Adventure".to_string(), 17), 248);
        map.insert(("Puzzle".to_string(), 17), 252);
        // mod packs
        map.insert(("Adventure and RPG".to_string(), 4471), 4475);
        map.insert(("Tech".to_string(), 4471), 4472);
        map.insert(("Magic".to_string(), 4471), 4473);
        map.insert(("Skyblock".to_string(), 4471), 4736);
        // shader packs
        map.insert(("Vanilla".to_string(), 6552), 6555);
        map.insert(("Fantasy".to_string(), 6552), 6554);
        map.insert(("Realistic".to_string(), 6552), 6553);
        // data packs
        map.insert(("Magic".to_string(), 6945), 6952);
        map.insert(("Miscellaneous".to_string(), 6945), 6947);
        map.insert(("Tech".to_string(), 6945), 6951);
        map
    })
}

// ── Conversion helpers ─────────────────────────────────────────────────────

pub fn cvt_class_id_to_type(class_id: i32) -> String {
    match class_id {
        6 => "mod".to_string(),
        12 => "resourcepack".to_string(),
        17 => "world".to_string(),
        4471 => "modpack".to_string(),
        6552 => "shader".to_string(),
        6945 => "datapack".to_string(),
        _ => "unknown".to_string(),
    }
}

pub fn cvt_type_to_class_id(_type: &str) -> u32 {
    match _type {
        "mod" => 6,
        "resourcepack" => 12,
        "world" => 17,
        "modpack" => 4471,
        "shader" => 6552,
        "datapack" => 6945,
        _ => 0,
    }
}

pub fn cvt_sort_by_to_id(sort_by: &str) -> u32 {
    match sort_by {
        "Popularity" => 2,
        "A-Z" => 4,
        "Latest update" => 3,
        "Creation date" => 11,
        "Total downloads" => 6,
        _ => 2,
    }
}

pub fn cvt_mod_loader_to_id(mod_loader: &str) -> u32 {
    match mod_loader {
        "Forge" => 1,
        "Fabric" => 4,
        "Quilt" => 5,
        "NeoForge" => 6,
        _ => 0,
    }
}

pub fn cvt_version_to_type_id(version: &str) -> u32 {
    match version {
        "26.1" => 83806,
        "1.21" => 77784,
        "1.20" => 75125,
        "1.19" => 73407,
        "1.18" => 73250,
        "1.17" => 73242,
        "1.16" => 70886,
        "1.15" => 68722,
        "1.14" => 64806,
        "1.13" => 55023,
        "1.12" => 628,
        "1.11" => 599,
        "1.10" => 572,
        "1.9" => 552,
        "1.8" => 4,
        "1.7" => 5,
        "1.6" => 6,
        "1.5" => 11,
        "1.4" => 12,
        "1.3" => 13,
        "1.2" => 14,
        "1.1" => 15,
        "1.0" => 16,
        _ => 0,
    }
}

pub fn cvt_id_to_release_type(release_type: u32) -> String {
    match release_type {
        1 => "release".to_string(),
        2 => "beta".to_string(),
        _ => "alpha".to_string(),
    }
}

pub fn cvt_id_to_dependency_type(dependency_type: u32) -> String {
    match dependency_type {
        1 => "embedded".to_string(),
        2 => "optional".to_string(),
        3 => "required".to_string(),
        4 => "tool".to_string(),
        5 => "incompatible".to_string(),
        _ => "include".to_string(),
    }
}

// ── Translation ────────────────────────────────────────────────────────────

pub async fn translate_description_curseforge(
    app: &AppHandle,
    resource_id: &str,
) -> Result<Option<String>, ResourceError> {
    let result = async {
        let url = get_curseforge_api(OtherResourceApiEndpoint::TranslateDesc, Some(resource_id))?;
        let client = app.state::<reqwest::Client>();
        let key = get_cf_api_key();

        let translation_res = client
            .get(&url)
            .header("x-api-key", key)
            .send()
            .await
            .map_err(|_| ResourceError::NetworkError)?
            .json::<CurseForgeTranslationRes>()
            .await
            .map_err(|_| ResourceError::ParseError)?;

        Ok::<_, ResourceError>(translation_res.translated)
    }
    .await;

    Ok(result.ok())
}
