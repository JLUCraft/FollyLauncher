use strum_macros::Display;

pub use minecraftd::resource::mirrors::{ResourceType, SourceType};

pub use minecraftd::resource::models::*;

pub use minecraftd::minecraft::manifest::GameClientResourceInfo;

pub use minecraftd::minecraft::loader::catalog::ModLoaderResourceInfo;

pub use minecraftd::minecraft::profile::OptiFineResourceInfo;

#[derive(Debug, Display)]
#[strum(serialize_all = "SCREAMING_SNAKE_CASE")]
pub enum ResourceError {
  ParseError,
  NoDownloadApi,
  NetworkError,
  FileOperationError,
  ClientVersionNotFound,
}

impl std::error::Error for ResourceError {}
