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

pub use minecraftd::modpack::multimc::MultiMcManifest;

#[async_trait]
impl ModpackManifest for MultiMcManifest {
  fn from_archive(file: &File) -> SJMCLResult<Self> {
    Ok(Self::read(file)?)
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
      name: self.cfg.get("name").cloned().unwrap_or_default(),
      version: None,
      description: None,
      author: None,
      modpack_source: OtherResourceSource::MultiMc,
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
    _instance_path: &Path,
  ) -> SJMCLResult<Vec<PTaskParam>> {
    // MultiMC Manifests do not include download parameters
    Ok(Vec::new())
  }

  fn get_overrides_path(&self) -> String {
    self.overrides_prefix()
  }
}

pub fn build_multimc_export_bundle(
  instance: &Instance,
  options: &ExportModpackOptions,
  selected_files: &[(String, PathBuf)],
) -> SJMCLResult<ModpackExportBundle> {
  Ok(minecraftd::modpack::multimc::build_multimc_export_bundle(
    &instance.version,
    instance.mod_loader.loader_type,
    &instance.mod_loader.version,
    options,
    selected_files,
  )?)
}
