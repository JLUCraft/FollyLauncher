use crate::error::LauncherError;
use crate::resource::validator::{library_artifact_path, LibraryEntry};
use serde::Deserialize;
use std::path::{Path, PathBuf};

#[derive(Debug, Clone, Default)]
pub struct MinecraftOptions {
    pub username: Option<String>,
    pub uuid: Option<String>,
    pub token: Option<String>,
    pub server: Option<String>,
    pub port: Option<String>,
    pub launcher_name: Option<String>,
    pub executable_path: Option<String>,
    pub jvm_arguments: Option<Vec<String>>,
    pub game_directory: Option<String>,
    pub custom_resolution: Option<bool>,
    pub resolution_width: Option<String>,
    pub resolution_height: Option<String>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct VersionManifest {
    #[serde(default)]
    main_class: String,
    #[serde(default)]
    libraries: Vec<LibraryEntry>,
    #[serde(default)]
    arguments: Option<Arguments>,
    #[serde(default)]
    minecraft_arguments: Option<String>,
}

#[derive(Debug, Deserialize, Default)]
struct Arguments {
    #[serde(default)]
    game: Vec<ArgumentValue>,
    #[serde(default)]
    jvm: Vec<ArgumentValue>,
}

#[derive(Debug, Deserialize)]
#[serde(untagged)]
enum ArgumentValue {
    String(String),
    Ruled { value: serde_json::Value },
}

pub fn get_minecraft_command(
    version: &str,
    game_dir: &Path,
    options: &MinecraftOptions,
) -> Result<Vec<String>, LauncherError> {
    let version_dir = game_dir.join("versions").join(version);
    let version_json_path = version_dir.join(format!("{version}.json"));
    let manifest = read_manifest(&version_json_path)?;
    let main_class = if manifest.main_class.is_empty() {
        "net.minecraft.client.main.Main"
    } else {
        &manifest.main_class
    };
    let natives_dir = version_dir.join("natives");
    let classpath = build_classpath(game_dir, &version_dir, version, &manifest.libraries);
    if classpath.is_empty() {
        return Err(LauncherError::from(format!(
            "版本 {version} 没有可用 classpath，请先安装客户端 jar 和 libraries"
        )));
    }

    let java = options
        .executable_path
        .clone()
        .filter(|value| !value.trim().is_empty())
        .unwrap_or_else(|| "java".to_string());

    let mut command = vec![java];
    command.extend(options.jvm_arguments.clone().unwrap_or_default());
    command.extend(resolve_jvm_arguments(
        manifest.arguments.as_ref(),
        game_dir,
        &natives_dir,
        &classpath,
        options,
    ));
    command.push(main_class.to_string());
    command.extend(resolve_game_arguments(
        version,
        manifest.arguments.as_ref(),
        manifest.minecraft_arguments.as_deref(),
        game_dir,
        options,
    ));
    Ok(command)
}

fn read_manifest(path: &Path) -> Result<VersionManifest, LauncherError> {
    let content = std::fs::read_to_string(path).map_err(|error| {
        LauncherError::from(format!(
            "cannot read version JSON at {}: {error}",
            path.display()
        ))
    })?;
    serde_json::from_str(&content).map_err(|error| {
        LauncherError::from(format!(
            "invalid version JSON at {}: {error}",
            path.display()
        ))
    })
}

fn build_classpath(
    game_dir: &Path,
    version_dir: &Path,
    version: &str,
    libraries: &[LibraryEntry],
) -> Vec<String> {
    let libraries_dir = game_dir.join("libraries");
    let mut classpath = Vec::new();
    for library in libraries {
        let path = library
            .downloads
            .as_ref()
            .and_then(|downloads| downloads.artifact.as_ref())
            .and_then(|artifact| artifact.path.as_ref())
            .map(|path| libraries_dir.join(path))
            .or_else(|| library_artifact_path(&library.name, &libraries_dir));
        if let Some(path) = path {
            classpath.push(path_to_string(path));
        }
    }
    classpath.push(path_to_string(version_dir.join(format!("{version}.jar"))));
    dedupe(classpath)
}

fn resolve_jvm_arguments(
    arguments: Option<&Arguments>,
    game_dir: &Path,
    natives_dir: &Path,
    classpath: &[String],
    options: &MinecraftOptions,
) -> Vec<String> {
    let mut values = argument_values(arguments.map(|args| args.jvm.as_slice()).unwrap_or(&[]));
    if values.is_empty() {
        values = vec![
            "-Djava.library.path=${natives_directory}".to_string(),
            "-cp".to_string(),
            "${classpath}".to_string(),
        ];
    }
    expand_arguments(values, game_dir, natives_dir, classpath, options)
}

fn resolve_game_arguments(
    version: &str,
    arguments: Option<&Arguments>,
    legacy_args: Option<&str>,
    game_dir: &Path,
    options: &MinecraftOptions,
) -> Vec<String> {
    let values = if let Some(args) = arguments {
        argument_values(&args.game)
    } else if let Some(legacy) = legacy_args {
        legacy.split_whitespace().map(str::to_string).collect()
    } else {
        vec![
            "--username".to_string(),
            "${auth_player_name}".to_string(),
            "--version".to_string(),
            "${version_name}".to_string(),
            "--gameDir".to_string(),
            "${game_directory}".to_string(),
            "--assetsDir".to_string(),
            "${assets_root}".to_string(),
            "--assetIndex".to_string(),
            "${assets_index_name}".to_string(),
            "--uuid".to_string(),
            "${auth_uuid}".to_string(),
            "--accessToken".to_string(),
            "${auth_access_token}".to_string(),
            "--userType".to_string(),
            "${user_type}".to_string(),
        ]
    };

    let mut expanded = expand_arguments(
        values,
        game_dir,
        &game_dir.join("versions").join(version).join("natives"),
        &[],
        options,
    );
    if let Some(server) = &options.server {
        expanded.push("--server".to_string());
        expanded.push(server.clone());
        if let Some(port) = &options.port {
            expanded.push("--port".to_string());
            expanded.push(port.clone());
        }
    }
    if options.custom_resolution.unwrap_or(false) {
        expanded.push("--width".to_string());
        expanded.push(
            options
                .resolution_width
                .clone()
                .unwrap_or_else(|| "854".to_string()),
        );
        expanded.push("--height".to_string());
        expanded.push(
            options
                .resolution_height
                .clone()
                .unwrap_or_else(|| "480".to_string()),
        );
    }
    expanded
}

fn argument_values(values: &[ArgumentValue]) -> Vec<String> {
    values
        .iter()
        .flat_map(|value| match value {
            ArgumentValue::String(value) => vec![value.clone()],
            ArgumentValue::Ruled { value } => match value {
                serde_json::Value::String(value) => vec![value.clone()],
                serde_json::Value::Array(values) => values
                    .iter()
                    .filter_map(|value| value.as_str().map(str::to_string))
                    .collect(),
                _ => Vec::new(),
            },
        })
        .collect()
}

fn expand_arguments(
    values: Vec<String>,
    game_dir: &Path,
    natives_dir: &Path,
    classpath: &[String],
    options: &MinecraftOptions,
) -> Vec<String> {
    values
        .into_iter()
        .map(|value| expand_placeholders(value, game_dir, natives_dir, classpath, options))
        .filter(|value| !value.is_empty())
        .collect()
}

fn expand_placeholders(
    value: String,
    game_dir: &Path,
    natives_dir: &Path,
    classpath: &[String],
    options: &MinecraftOptions,
) -> String {
    let sep = if cfg!(target_os = "windows") {
        ";"
    } else {
        ":"
    };
    let launcher = options
        .launcher_name
        .clone()
        .unwrap_or_else(|| "FollyLauncher".to_string());
    let game_dir_value = options
        .game_directory
        .clone()
        .unwrap_or_else(|| path_to_string(game_dir));
    value
        .replace("${natives_directory}", &path_to_string(natives_dir))
        .replace("${launcher_name}", &launcher)
        .replace("${launcher_version}", env!("CARGO_PKG_VERSION"))
        .replace("${classpath}", &classpath.join(sep))
        .replace("${classpath_separator}", sep)
        .replace(
            "${library_directory}",
            &path_to_string(game_dir.join("libraries")),
        )
        .replace(
            "${auth_player_name}",
            options.username.as_deref().unwrap_or("Player"),
        )
        .replace("${auth_uuid}", options.uuid.as_deref().unwrap_or(""))
        .replace(
            "${auth_access_token}",
            options.token.as_deref().unwrap_or(""),
        )
        .replace("${clientid}", "")
        .replace("${auth_xuid}", "")
        .replace("${user_type}", "msa")
        .replace("${version_name}", &launcher)
        .replace("${game_directory}", &game_dir_value)
        .replace("${assets_root}", &path_to_string(game_dir.join("assets")))
        .replace("${assets_index_name}", "")
        .replace(
            "${resolution_width}",
            options.resolution_width.as_deref().unwrap_or("854"),
        )
        .replace(
            "${resolution_height}",
            options.resolution_height.as_deref().unwrap_or("480"),
        )
}

fn dedupe(values: Vec<String>) -> Vec<String> {
    let mut seen = std::collections::HashSet::new();
    let mut out = Vec::new();
    for value in values {
        if seen.insert(value.clone()) {
            out.push(value);
        }
    }
    out
}

fn path_to_string(path: impl Into<PathBuf>) -> String {
    path.into().to_string_lossy().to_string()
}
