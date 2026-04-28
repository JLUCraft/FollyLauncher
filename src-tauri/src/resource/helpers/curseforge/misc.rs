use crate::resource::helpers::misc::version_pack_sort;
use crate::resource::models::{
    OtherResourceApiEndpoint, OtherResourceDependency, OtherResourceFileInfo, OtherResourceInfo,
    OtherResourceSearchRes, OtherResourceSource, OtherResourceVersionPack,
    ResourceError,
};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::env;
use tauri::{AppHandle, Manager};

const CURSEFORGE_API_KEY_ENV: &str = "FOLLY_CURSEFORGE_API_KEY";

fn get_cf_api_key() -> String {
    env::var(CURSEFORGE_API_KEY_ENV).unwrap_or_default()
}

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
                let file_info = (&cf_file, if loader.is_empty() { None } else { Some(loader.to_string()) }).into();

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
