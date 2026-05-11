use std::sync::Arc;
use crate::resource::curseforge::translate_description_curseforge;
use crate::resource::mod_db::ModDataBase;
use crate::resource::models::{
    OtherResourceInfo, OtherResourceSource, OtherResourceVersionPack, ResourceError, ResourceType,
    SourceType,
};
use crate::resource::modrinth::translate_description_modrinth;
use std::cmp::Ordering;
use tauri::{AppHandle, Manager};
use tokio::sync::Mutex;
use url::Url;

pub fn get_use_mirror(app: &AppHandle) -> bool {
    app.try_state::<Arc<Mutex<bool>>>()
        .map(|s| *s.blocking_lock())
        .unwrap_or(false)
}

pub fn get_source_priority_list(use_mirror_first: bool) -> Vec<SourceType> {
    if use_mirror_first {
        vec![SourceType::BMCLAPIMirror, SourceType::Official]
    } else {
        vec![SourceType::Official, SourceType::BMCLAPIMirror]
    }
}

pub fn get_download_api(
    source: SourceType,
    resource_type: ResourceType,
) -> Result<Url, ResourceError> {
    match source {
        SourceType::Official => match resource_type {
            ResourceType::VersionManifest => Ok(Url::parse(
                "https://launchermeta.mojang.com/mc/game/version_manifest.json",
            )
            .expect("static URL")),
            ResourceType::ForgeMeta => Err(ResourceError::NoDownloadApi),
            ResourceType::OptiFine => Err(ResourceError::NoDownloadApi),
            ResourceType::FabricMeta => {
                Ok(Url::parse("https://meta.fabricmc.net/").expect("static URL"))
            }
            ResourceType::NeoforgeMetaForge => Ok(Url::parse(
                "https://maven.neoforged.net/api/maven/versions/releases/net/neoforged/forge/",
            )
            .expect("static URL")),
            ResourceType::NeoforgeMetaNeoforge => Ok(Url::parse(
                "https://maven.neoforged.net/api/maven/versions/releases/net/neoforged/neoforge/",
            )
            .expect("static URL")),
            ResourceType::QuiltMeta => {
                Ok(Url::parse("https://meta.quiltmc.org/").expect("static URL"))
            }
        },
        SourceType::BMCLAPIMirror => match resource_type {
            ResourceType::VersionManifest => Ok(Url::parse(
                "https://bmclapi2.bangbang93.com/mc/game/version_manifest.json",
            )
            .expect("static URL")),
            ResourceType::ForgeMeta => {
                Ok(Url::parse("https://bmclapi2.bangbang93.com/forge/").expect("static URL"))
            }
            ResourceType::FabricMeta => {
                Ok(Url::parse("https://bmclapi2.bangbang93.com/fabric-meta/").expect("static URL"))
            }
            ResourceType::NeoforgeMetaForge | ResourceType::NeoforgeMetaNeoforge => {
                Ok(Url::parse("https://bmclapi2.bangbang93.com/neoforge/").expect("static URL"))
            }
            ResourceType::OptiFine => {
                Ok(Url::parse("https://bmclapi2.bangbang93.com/optifine/").expect("static URL"))
            }
            ResourceType::QuiltMeta => {
                Ok(Url::parse("https://bmclapi2.bangbang93.com/quilt-meta/").expect("static URL"))
            }
        },
    }
}

pub fn version_pack_sort(a: &OtherResourceVersionPack, b: &OtherResourceVersionPack) -> Ordering {
    fn parse_version(version: &str) -> (Vec<u32>, String) {
        let mut version_numbers = Vec::new();
        let mut suffix = String::new();

        for part in version.split('.') {
            if let Some(dash_pos) = part.find('-') {
                let (num_part, suffix_part) = part.split_at(dash_pos);
                if let Ok(num) = num_part.parse::<u32>() {
                    version_numbers.push(num);
                    suffix = suffix_part.to_string();
                }
                break;
            } else if let Ok(num) = part.parse::<u32>() {
                version_numbers.push(num);
            }
        }

        (version_numbers, suffix)
    }

    fn compare_versions_with_suffix(
        v1: &[u32],
        suffix1: &str,
        v2: &[u32],
        suffix2: &str,
    ) -> Ordering {
        for (a, b) in v1.iter().zip(v2.iter()) {
            match a.cmp(b) {
                Ordering::Equal => continue,
                other => return other,
            }
        }

        match v1.len().cmp(&v2.len()) {
            Ordering::Equal => match (suffix1.is_empty(), suffix2.is_empty()) {
                (true, false) => Ordering::Greater,
                (false, true) => Ordering::Less,
                _ => suffix1.cmp(suffix2),
            },
            other => other,
        }
    }

    let (version_a, suffix_a) = parse_version(&a.name);
    let (version_b, suffix_b) = parse_version(&b.name);

    compare_versions_with_suffix(&version_a, &suffix_a, &version_b, &suffix_b).reverse()
}

pub async fn apply_other_resource_enhancements(
    app: &AppHandle,
    resource_info: &mut OtherResourceInfo,
) -> Result<(), ResourceError> {
    let (translated_name, mcmod_id): (Option<String>, Option<u32>) = {
        let state = app.try_state::<Mutex<ModDataBase>>();
        if let Some(cache_state) = state {
            let cache = cache_state.lock().await;
            let translated_name = if resource_info._type == "mod" {
                cache.get_translated_name(&resource_info.slug, &resource_info.source)
            } else {
                None
            };
            let mcmod_id = cache.get_mcmod_id(&resource_info.slug, &resource_info.source);
            (translated_name, mcmod_id)
        } else {
            (None, None)
        }
    };

    if let Some(name) = translated_name {
        if name.chars().any(|c| matches!(c, '\u{4e00}'..='\u{9fbb}')) {
            resource_info.translated_name = Some(name);
        }
    }
    if let Some(id) = mcmod_id {
        resource_info.mcmod_id = id;
    }

    let translated_desc = match resource_info.source {
        OtherResourceSource::Modrinth => {
            translate_description_modrinth(app, &resource_info.id).await
        }
        OtherResourceSource::CurseForge => {
            translate_description_curseforge(app, &resource_info.id).await
        }
        _ => Ok(None),
    };

    if let Ok(Some(desc)) = translated_desc {
        resource_info.translated_description = Some(desc);
    }

    Ok(())
}
