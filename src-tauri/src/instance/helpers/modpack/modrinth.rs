use async_trait::async_trait;
use sjmcl_types::error::SJMCLResult;
use std::fs::File;
use std::path::{Path, PathBuf};
use tauri::AppHandle;

use crate::instance::helpers::modpack::export::{ExportModpackOptions, ModpackExportBundle};
use crate::instance::helpers::modpack::import::{ModpackManifest, ModpackMetaInfo};
use crate::instance::models::misc::{Instance, InstanceError, ModLoader, ModLoaderType};
use crate::resource::models::OtherResourceSource;
use crate::tasks::PTaskParam;

pub use minecraftd::modpack::modrinth::ModrinthManifest;

#[async_trait]
impl ModpackManifest for ModrinthManifest {
  fn from_archive(file: &File) -> SJMCLResult<Self> {
    Ok(minecraftd::modpack::read_manifest(
      file,
      "modrinth.index.json",
    )?)
  }

  async fn get_meta_info(&self, app: &AppHandle) -> SJMCLResult<ModpackMetaInfo> {
    let client_version = self.get_client_version()?;
    let mod_loader = if let Ok((loader_type, version)) = self.get_mod_loader_type_version() {
      Some(
        ModLoader {
          loader_type,
          version,
          ..Default::default()
        }
        .with_branch(app, client_version.clone())
        .await?,
      )
    } else {
      None
    };
    Ok(ModpackMetaInfo {
      name: self.name.clone(),
      version: Some(self.version_id.clone()),
      description: self.summary.clone(),
      author: None,
      modpack_source: OtherResourceSource::Modrinth,
      client_version,
      mod_loader,
    })
  }

  fn get_client_version(&self) -> SJMCLResult<String> {
    Ok(minecraftd::modpack::ModpackManifest::client_version(self)?)
  }
  fn get_mod_loader_type_version(&self) -> SJMCLResult<(ModLoaderType, String)> {
    minecraftd::modpack::ModpackManifest::mod_loader(self)?
      .ok_or_else(|| InstanceError::ModLoaderVersionParseError.into())
  }

  async fn get_download_params(
    &self,
    _app: &AppHandle,
    instance_path: &Path,
  ) -> SJMCLResult<Vec<PTaskParam>> {
    Ok(
      self
        .download_requests(instance_path)?
        .into_iter()
        .map(PTaskParam::Download)
        .collect(),
    )
  }

  fn get_overrides_path(&self) -> String {
    "overrides/".to_string()
  }
}

pub async fn build_modrinth_export_bundle(
  app: &AppHandle,
  instance: &Instance,
  options: &ExportModpackOptions,
  selected_files: &[(String, PathBuf)],
) -> SJMCLResult<ModpackExportBundle> {
  use tauri::Manager;
  let client = app.state::<tauri_plugin_http::reqwest::Client>();
  Ok(
    minecraftd::modpack::modrinth::build_modrinth_export_bundle(
      &crate::resource::helpers::http::Transport(&client),
      Some(crate::resource::helpers::curseforge::misc::CURSEFORGE_API_KEY.as_str()),
      &instance.version,
      instance.mod_loader.loader_type,
      &instance.mod_loader.version,
      options,
      selected_files,
    )
    .await?,
  )
}
