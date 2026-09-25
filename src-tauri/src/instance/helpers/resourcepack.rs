use crate::utils::image::decode_image;
use image::RgbaImage;
use minecraftd::minecraft::packs::{self, PackMetadata};
use sjmcl_types::error::{SJMCLError, SJMCLResult};
use std::path::Path;

fn present(metadata: PackMetadata) -> (String, Option<RgbaImage>) {
  (
    metadata.description,
    metadata.icon.and_then(|bytes| decode_image(bytes).ok()),
  )
}

pub fn load_resourcepack_from_zip(path: &Path) -> SJMCLResult<(String, Option<RgbaImage>)> {
  packs::read_archive(path)
    .map(present)
    .map_err(|error| SJMCLError(error.to_string()))
}

pub async fn load_resourcepack_from_dir(path: &Path) -> SJMCLResult<(String, Option<RgbaImage>)> {
  packs::read_directory(path)
    .await
    .map(present)
    .map_err(|error| SJMCLError(error.to_string()))
}
