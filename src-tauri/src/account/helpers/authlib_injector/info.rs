use sjmcl_types::error::SJMCLResult;
use std::collections::BTreeSet;
use std::sync::Mutex;
use tauri::{AppHandle, Manager};
use tauri_plugin_http::reqwest;
use url::Url;

use crate::account::helpers::authlib_injector::constants::CLIENT_IDS;
use crate::account::models::{AccountError, AccountInfo, AuthServerInfo};
use crate::utils::web::normalize_url;

pub async fn fetch_auth_server_info(
  app: &AppHandle,
  auth_url: String,
) -> SJMCLResult<AuthServerInfo> {
  let client = app.state::<reqwest::Client>();
  match client.get(&auth_url).send().await {
    Ok(response) => {
      let json: serde_json::Value = response.json().await.map_err(|_| AccountError::Invalid)?;

      let mut client_id = None;

      let openid_configuration_url = json["meta"]["feature.openid_configuration_url"]
        .as_str()
        .unwrap_or_default()
        .to_string();

      if !openid_configuration_url.is_empty() {
        let url = Url::parse(&auth_url).map_err(|_| AccountError::Invalid)?;

        if let Some(domain) = url.domain() {
          client_id = get_client_id(domain.to_string());
        }

        if client_id.is_none() {
          let response = client.get(&openid_configuration_url).send().await?;
          let data: serde_json::Value = response.json().await.map_err(|_| AccountError::Invalid)?;
          client_id = data["shared_client_id"].as_str().map(|s| s.to_string());
        }
      }

      Ok(AuthServerInfo {
        auth_url: auth_url.clone(),
        client_id,
        metadata: json,
        timestamp: chrono::Utc::now().timestamp_millis() as u64,
      })
    }
    Err(_) => Err(AccountError::Invalid.into()),
  }
}

pub fn get_client_id(domain: String) -> Option<String> {
  CLIENT_IDS
    .iter()
    .find(|(first, _)| first == &domain)
    .map(|(_, id)| id.to_string())
}

pub async fn fetch_auth_url(app: &AppHandle, root: Url) -> SJMCLResult<String> {
  let client = app.state::<reqwest::Client>();
  let response = client
    .get(root.clone())
    .send()
    .await
    .map_err(|_| AccountError::Invalid)?;

  if let Some(auth_url) = response.headers().get("X-Authlib-Injector-API-Location") {
    let auth_url_str = auth_url.to_str().unwrap_or_default();
    // try to parse auth_url_str as a relative URL and append it to the base URL or return it as is
    let full_url = root
      .join(auth_url_str)
      .map(|url| url.to_string())
      .unwrap_or_else(|_| auth_url_str.to_string());

    Ok(full_url)
  } else {
    Ok(root.to_string())
  }
}

pub async fn refresh_and_update_auth_servers(app: &AppHandle) -> SJMCLResult<()> {
  let account_binding = app.state::<Mutex<AccountInfo>>();
  let cloned_account_state = account_binding.lock()?.clone();

  let mut refreshed_auth_server_info_list =
    futures::future::join_all(cloned_account_state.auth_servers.iter().map(|info| async {
      if let Ok(refreshed_info) = fetch_auth_server_info(app, info.auth_url.clone()).await {
        refreshed_info
      } else {
        info.clone()
      }
    }))
    .await;

  refreshed_auth_server_info_list.retain(|info| !info.metadata.is_null()); // remove invalid servers

  let mut account_state = account_binding.lock()?;
  account_state.auth_servers = refreshed_auth_server_info_list;

  Ok(())
}

pub fn get_auth_server_info_by_url(
  app: &AppHandle,
  auth_url: String,
) -> SJMCLResult<AuthServerInfo> {
  let target = normalize_url(&auth_url);

  let account_binding = app.state::<Mutex<AccountInfo>>();
  let account_state = account_binding.lock()?;

  account_state
    .auth_servers
    .iter()
    .find(|server| normalize_url(&server.auth_url) == target)
    .cloned()
    .ok_or(AccountError::NotFound.into())
}

fn is_skin_domain(domain: &str) -> bool {
  !domain.is_empty()
    && domain.len() <= 253
    && domain
      .chars()
      .all(|c| c.is_ascii_alphanumeric() || c == '.' || c == '-')
}

/// Hosts the game may fetch textures from: every known auth server and the skin domains its
/// metadata advertises.
fn skin_domains(servers: &[AuthServerInfo]) -> Vec<String> {
  let mut domains = BTreeSet::new();
  for server in servers {
    if let Some(host) = Url::parse(&server.auth_url)
      .ok()
      .and_then(|url| url.domain().map(str::to_string))
    {
      domains.insert(host);
    }
    for domain in server.metadata["skinDomains"]
      .as_array()
      .into_iter()
      .flatten()
    {
      if let Some(domain) = domain.as_str().filter(|d| is_skin_domain(d)) {
        domains.insert(domain.to_string());
      }
    }
  }
  domains.into_iter().collect()
}

pub fn federation_skin_domains(app: &AppHandle) -> SJMCLResult<Vec<String>> {
  let account_binding = app.state::<Mutex<AccountInfo>>();
  let account_state = account_binding.lock()?;
  Ok(skin_domains(&account_state.auth_servers))
}

#[cfg(test)]
mod tests {
  use super::*;
  use serde_json::json;

  fn server(auth_url: &str, metadata: serde_json::Value) -> AuthServerInfo {
    AuthServerInfo {
      auth_url: auth_url.into(),
      metadata,
      ..Default::default()
    }
  }

  #[test]
  fn skin_domains_merge_hosts_and_advertised_domains_and_drop_junk() {
    let servers = [
      server(
        "https://skin.jlucraft.com",
        json!({"skinDomains": ["skin.jlucraft.com", ".mualliance.ltd", "bad domain", "", "a/b", 7]}),
      ),
      server("https://skin.mualliance.ltd/api/yggdrasil", json!(null)),
      server("not a url", json!({"skinDomains": ["x.example"]})),
    ];
    assert_eq!(
      skin_domains(&servers),
      [
        ".mualliance.ltd",
        "skin.jlucraft.com",
        "skin.mualliance.ltd",
        "x.example"
      ]
    );
    assert!(skin_domains(&[]).is_empty());
  }
}
