use async_trait::async_trait;
use sjmcl_types::error::SJMCLResult;
use std::fs::File;
use std::path::Path;
use tauri::{AppHandle, Manager};
use tauri_plugin_http::reqwest;

use crate::instance::helpers::modpack::import::{ModpackManifest, ModpackMetaInfo};
use crate::instance::models::misc::{InstanceError, ModLoader, ModLoaderType};
use crate::resource::helpers::curseforge::misc::CURSEFORGE_API_KEY;
use crate::resource::models::OtherResourceSource;
use crate::tasks::PTaskParam;

pub use minecraftd::modpack::curseforge::CurseForgeManifest;

#[async_trait]
impl ModpackManifest for CurseForgeManifest {
  fn from_archive(file: &File) -> SJMCLResult<Self> {
    Ok(minecraftd::modpack::read_manifest(file, "manifest.json")?)
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
      version: self.version.clone(),
      description: None,
      author: Some(self.author.clone()),
      modpack_source: OtherResourceSource::CurseForge,
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
    app: &AppHandle,
    instance_path: &Path,
  ) -> SJMCLResult<Vec<PTaskParam>> {
    let client = app.state::<reqwest::Client>();
    let backend = minecraftd::resource::curseforge::CurseForgeClient::with_client(
      crate::resource::helpers::http::Transport(&client),
      CURSEFORGE_API_KEY.to_string(),
    );
    Ok(
      self
        .download_requests(&backend, instance_path)
        .await?
        .into_iter()
        .map(PTaskParam::Download)
        .collect(),
    )
  }

  fn get_overrides_path(&self) -> String {
    self.overrides.clone()
  }
}
