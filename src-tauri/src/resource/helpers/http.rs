use minecraftd::resource::{ResourceError, http::ResourceTransport};
use serde_json::Value;
use std::collections::HashMap;
pub struct Transport<'a>(pub &'a tauri_plugin_http::reqwest::Client);
#[async_trait::async_trait]
impl ResourceTransport for Transport<'_> {
  async fn request_json(
    &self,
    url: &str,
    query: &HashMap<String, String>,
    body: Option<&Value>,
    api_key: Option<&str>,
  ) -> Result<Value, ResourceError> {
    let mut request = match body {
      Some(body) => self.0.post(url).json(body),
      None => self.0.get(url),
    }
    .query(query);
    if let Some(key) = api_key {
      request = request.header("x-api-key", key);
    }
    let response = request
      .send()
      .await
      .and_then(|response| response.error_for_status())
      .map_err(|error| ResourceError::Network(error.to_string()))?;
    response
      .json()
      .await
      .map_err(|error| ResourceError::Parse(error.to_string()))
  }
}
