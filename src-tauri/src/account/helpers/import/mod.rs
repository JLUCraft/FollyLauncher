pub mod hmcl;
pub mod legacy_hmcl;
pub mod misc;
pub mod multimc;

use serde::Deserialize;

// other launchers we support import accounts from
#[derive(Debug, Clone, Deserialize)]
pub enum ImportLauncherType {
  #[serde(rename = "HMCL")]
  Hmcl,
  #[serde(rename = "LegacyHMCL")]
  LegacyHmcl,
  #[serde(rename = "SCL")]
  Scl, // only on macOS
  #[serde(rename = "MultiMC")]
  MultiMc,
}
