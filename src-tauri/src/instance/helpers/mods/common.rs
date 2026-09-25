use crate::instance::constants::COMPRESSED_ICON_SIZE;
use crate::instance::models::misc::{LocalModInfo, ModLoaderType};
use crate::utils::image::ImageWrapper;
use image::imageops::FilterType;
pub use minecraftd::minecraft::metadata::discover_local_mod_paths;
use minecraftd::minecraft::metadata::{self, ModMetadata};
use sjmcl_types::error::SJMCLResult;
use std::path::Path;
fn present(info: ModMetadata) -> LocalModInfo {
  LocalModInfo {
    icon_src: image::load_from_memory(&info.icon)
      .ok()
      .map(|image| ImageWrapper::from(image.to_rgba8()))
      .map(compress_icon)
      .unwrap_or_default(),
    enabled: info.enabled,
    mod_id: info.mod_id,
    name: info.name,
    version: info.version,
    loader_type: info.loader_type,
    file_name: info.file_name,
    file_path: info.file_path,
    description: info.description,
    potential_incompatibility: info.potential_incompatibility,
    ..Default::default()
  }
}
pub async fn get_mod_info_from_jar(
  path: &Path,
  preferred: Option<ModLoaderType>,
) -> SJMCLResult<LocalModInfo> {
  Ok(present(
    metadata::get_mod_info_from_jar(path, preferred).await?,
  ))
}
pub async fn get_mod_info_from_dir(
  path: &Path,
  preferred: Option<ModLoaderType>,
) -> SJMCLResult<LocalModInfo> {
  Ok(present(
    metadata::get_mod_info_from_dir(path, preferred).await?,
  ))
}
pub fn check_potential_incompatibility(
  infos: &mut [LocalModInfo],
  loader: Option<ModLoaderType>,
  version: &str,
) {
  let mut metadata: Vec<ModMetadata> = infos
    .iter()
    .map(|info| ModMetadata {
      mod_id: info.mod_id.clone(),
      loader_type: info.loader_type,
      ..Default::default()
    })
    .collect();
  metadata::check_potential_incompatibility(&mut metadata, loader, version);
  for (info, meta) in infos.iter_mut().zip(metadata) {
    info.potential_incompatibility = meta.potential_incompatibility;
  }
}
pub fn compress_icon(wrapper: ImageWrapper) -> ImageWrapper {
  let resized_image = image::imageops::resize(
    &wrapper.image,
    COMPRESSED_ICON_SIZE.0,
    COMPRESSED_ICON_SIZE.1,
    FilterType::Nearest,
  );
  ImageWrapper {
    image: resized_image,
  }
}
