use lazy_static::lazy_static;
use std::env;
lazy_static! {
  pub static ref CURSEFORGE_API_KEY: String = {
    env::var("SJMCL_CURSEFORGE_API_KEY")
      .unwrap_or_else(|_| env!("SJMCL_CURSEFORGE_API_KEY").to_string())
  };
}

use crate::resource::helpers::http::Transport;
use minecraftd::resource::curseforge::CurseForgeClient;
pub use minecraftd::resource::curseforge::is_curseforge_authenticated_url;
use sjmcl_types::error::SJMCLResult;
use tauri::{AppHandle, Manager};
use tauri_plugin_http::reqwest;
pub async fn translate_description_curseforge(
  app: &AppHandle,
  resource_id: &str,
) -> SJMCLResult<Option<String>> {
  let client = app.state::<reqwest::Client>();
  Ok(
    CurseForgeClient::with_client(Transport(&client), CURSEFORGE_API_KEY.to_string())
      .translate_description(resource_id.parse::<u64>()?)
      .await?,
  )
}
