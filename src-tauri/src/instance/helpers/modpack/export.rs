use sjmcl_types::error::SJMCLResult;
use std::path::PathBuf;
use tauri::AppHandle;

use crate::instance::helpers::modpack::{
  modrinth::build_modrinth_export_bundle, multimc::build_multimc_export_bundle,
};
use crate::instance::models::misc::{Instance, InstanceError, ModpackFileList};

pub use minecraftd::modpack::export::{ExportFormat, ExportModpackOptions};

/// Validate export options before proceeding
pub fn validate_export_options(
  instance: &Instance,
  options: &ExportModpackOptions,
) -> SJMCLResult<()> {
  // Check required fields
  if options.name.trim().is_empty() {
    return Err(InstanceError::InvalidNameError.into());
  }

  if options.version.trim().is_empty() {
    return Err(InstanceError::ModpackManifestParseError.into());
  }

  if !instance.version_path.exists() {
    return Err(InstanceError::FileNotFoundError.into());
  }

  Ok(())
}

// ============================================================================
// Candidate File Listing
// ============================================================================

pub use minecraftd::modpack::export::ModpackExportBundle;
pub fn list_files(instance: &Instance) -> SJMCLResult<ModpackFileList> {
  Ok(minecraftd::modpack::export::list_files(
    &instance.version_path,
    &instance.name,
  )?)
}

pub async fn build_export_bundle(
  app: &AppHandle,
  instance: &Instance,
  options: &ExportModpackOptions,
  selected_files: &[(String, PathBuf)],
) -> SJMCLResult<ModpackExportBundle> {
  match options.format {
    ExportFormat::Modrinth => {
      build_modrinth_export_bundle(app, instance, options, selected_files).await
    }
    ExportFormat::MultiMC => build_multimc_export_bundle(instance, options, selected_files),
  }
}

pub async fn create_modpack_zip(save_path: &str, bundle: ModpackExportBundle) -> SJMCLResult<()> {
  let extra: Vec<(&str, String)> = bundle
    .extra_files
    .iter()
    .map(|(name, contents)| (name.as_str(), contents.clone()))
    .collect();
  Ok(
    minecraftd::modpack::create_modpack_zip(
      std::path::Path::new(save_path),
      std::path::Path::new(""),
      &bundle.overrides_prefix,
      &bundle.overrides_files,
      &extra,
    )
    .await?,
  )
}
