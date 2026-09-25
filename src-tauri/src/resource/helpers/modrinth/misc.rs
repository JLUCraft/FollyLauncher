use crate::resource::helpers::http::Transport;
use minecraftd::resource::modrinth::ModrinthClient;
use sjmcl_types::error::SJMCLResult;
use tauri::{AppHandle, Manager};
use tauri_plugin_http::reqwest;
pub async fn translate_description_modrinth(
  app: &AppHandle,
  resource_id: &str,
) -> SJMCLResult<Option<String>> {
  let client = app.state::<reqwest::Client>();
  Ok(
    ModrinthClient::with_client(Transport(&client))
      .translate_description(resource_id)
      .await?,
  )
}
