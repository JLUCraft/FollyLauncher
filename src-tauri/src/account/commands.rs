use crate::AppState;
use md5::Digest;
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use tauri::State;
use tokio::sync::Mutex;

use super::models::{
    AccountAvatarResult, AccountExportBundle, AddOfflineAccountRequest,
    AddThirdPartyAccountRequest, ImportAccountsRequest, ImportAccountsResult,
    ImportExternalAccountsRequest, ImportExternalAccountsResult, LauncherAccount,
    LauncherAccountKind, MicrosoftDeviceAuthStartResult, MicrosoftLoginResult,
    MicrosoftRefreshResult, ThirdPartyLoginResult, UpdateAccountAvatarRequest,
};

fn accounts_file_path(data_dir: &Path) -> PathBuf {
    data_dir.join("accounts").join("accounts.json")
}

use crate::error::LauncherError;
use crate::utils;





pub fn account_token_file_path(data_dir: &Path) -> PathBuf {
    data_dir.join("accounts").join("account_tokens.json")
}



fn load_all_account_tokens(data_dir: &Path) -> Result<HashMap<String, String>, LauncherError> {
    utils::load_json_map(&account_token_file_path(data_dir))
}


fn save_all_account_tokens(
    data_dir: &Path,
    tokens: &HashMap<String, String>,
) -> Result<(), LauncherError> {
    utils::save_json_map(&account_token_file_path(data_dir), tokens)
}


pub fn load_account_token(
    data_dir: &Path,
    account_id: &str,
) -> Result<Option<String>, LauncherError> {
    let tokens = load_all_account_tokens(data_dir)?;
    Ok(tokens.get(account_id).cloned())
}


pub fn save_account_token(
    data_dir: &Path,
    account_id: &str,
    token: &str,
) -> Result<(), LauncherError> {
    let mut tokens = load_all_account_tokens(data_dir)?;
    tokens.insert(account_id.to_string(), token.to_string());
    save_all_account_tokens(data_dir, &tokens)
}


pub fn delete_account_token(data_dir: &Path, account_id: &str) -> Result<(), LauncherError> {
    let mut tokens = load_all_account_tokens(data_dir)?;
    tokens.remove(account_id);
    save_all_account_tokens(data_dir, &tokens)
}







pub fn account_token_for_local_launch(
    data_dir: &Path,
    account: &LauncherAccount,
) -> Result<String, LauncherError> {
    match account.kind {
        LauncherAccountKind::Offline => Ok("0".to_string()),
        LauncherAccountKind::Microsoft => {
            let token = load_account_token(data_dir, &account.id)?;
            token.ok_or_else(|| LauncherError::from("Microsoft 账户登录已失效，请重新登录"))
        }
        LauncherAccountKind::ThirdParty => {
            let token = load_account_token(data_dir, &account.id)?;
            token.ok_or_else(|| LauncherError::from("第三方账户登录已失效，请重新登录"))
        }
    }
}





pub fn microsoft_refresh_token_file_path(data_dir: &Path) -> PathBuf {
    data_dir
        .join("accounts")
        .join("microsoft_refresh_tokens.json")
}


fn load_all_microsoft_refresh_tokens(
    data_dir: &Path,
) -> Result<HashMap<String, String>, LauncherError> {
    utils::load_json_map(&microsoft_refresh_token_file_path(data_dir))
}


fn save_all_microsoft_refresh_tokens(
    data_dir: &Path,
    tokens: &HashMap<String, String>,
) -> Result<(), LauncherError> {
    utils::save_json_map(&microsoft_refresh_token_file_path(data_dir), tokens)
}


pub fn load_microsoft_refresh_token(
    data_dir: &Path,
    account_id: &str,
) -> Result<Option<String>, LauncherError> {
    let tokens = load_all_microsoft_refresh_tokens(data_dir)?;
    Ok(tokens.get(account_id).cloned())
}


pub fn save_microsoft_refresh_token(
    data_dir: &Path,
    account_id: &str,
    token: &str,
) -> Result<(), LauncherError> {
    let mut tokens = load_all_microsoft_refresh_tokens(data_dir)?;
    tokens.insert(account_id.to_string(), token.to_string());
    save_all_microsoft_refresh_tokens(data_dir, &tokens)
}


pub fn delete_microsoft_refresh_token(
    data_dir: &Path,
    account_id: &str,
) -> Result<(), LauncherError> {
    let mut tokens = load_all_microsoft_refresh_tokens(data_dir)?;
    tokens.remove(account_id);
    save_all_microsoft_refresh_tokens(data_dir, &tokens)
}




pub fn ensure_profile_matches_account(
    profile_uuid: &str,
    account: &LauncherAccount,
) -> Result<(), LauncherError> {
    if profile_uuid != account.uuid {
        Err(LauncherError::from(format!(
            "刷新后 profile UUID ({}) 与账户 UUID ({}) 不匹配，请确认账户无误",
            profile_uuid, account.uuid
        )))
    } else {
        Ok(())
    }
}




#[derive(Debug, serde::Deserialize)]
struct MicrosoftDeviceCodeResponse {
    device_code: String,
    user_code: String,
    verification_uri: String,
    #[serde(default)]
    verification_uri_complete: Option<String>,
    expires_in: u64,
    interval: u64,
    #[serde(default)]
    message: Option<String>,
}

#[derive(Debug, serde::Deserialize)]
struct MicrosoftTokenResponse {
    access_token: String,
    #[serde(default)]
    refresh_token: Option<String>,
}

#[derive(Debug, serde::Deserialize)]
struct MicrosoftTokenError {
    error: String,
    #[serde(default)]
    error_description: Option<String>,
}

#[derive(Debug, serde::Deserialize)]
struct XboxAuthResponse {
    #[serde(rename = "Token")]
    token: String,
    #[serde(rename = "DisplayClaims")]
    display_claims: XboxDisplayClaims,
}

#[derive(Debug, serde::Deserialize)]
struct XboxDisplayClaims {
    xui: Vec<XboxXui>,
}

#[derive(Debug, serde::Deserialize)]
struct XboxXui {
    uhs: String,
}

#[derive(Debug, serde::Deserialize)]
struct MinecraftAuthResponse {
    access_token: String,
    #[serde(default)]
    expires_in: Option<u64>,
}

#[derive(Debug, serde::Deserialize)]
struct MinecraftProfileResponse {
    id: String,
    name: String,
}

#[derive(Debug, serde::Deserialize)]
struct MinecraftProfileError {
    #[serde(default)]
    error: String,
}


pub fn map_microsoft_oauth_error(error_code: &str) -> String {
    match error_code {
        "authorization_pending" => "尚未完成授权，请在浏览器中输入验证码后重试".to_string(),
        "slow_down" => "轮询过于频繁，请等待片刻后重试".to_string(),
        "expired_token" => "设备码已过期，请重新发起登录".to_string(),
        "authorization_declined" => "授权已被拒绝".to_string(),
        "bad_verification_code" => "验证码无效".to_string(),
        "invalid_grant" => "授权已失效，请重新发起登录".to_string(),
        other => format!("Microsoft 认证错误: {other}"),
    }
}

#[cfg(test)]

pub fn extract_uhs_from_xbox_response(json: &serde_json::Value) -> Result<String, LauncherError> {
    json["DisplayClaims"]["xui"]
        .get(0)
        .and_then(|xui| xui["uhs"].as_str())
        .map(|s| s.to_string())
        .ok_or_else(|| LauncherError::from("Xbox Live 认证响应缺少 uhs"))
}

#[cfg(test)]

pub fn extract_uhs_from_xsts_response(json: &serde_json::Value) -> Result<String, LauncherError> {
    json["DisplayClaims"]["xui"]
        .get(0)
        .and_then(|xui| xui["uhs"].as_str())
        .map(|s| s.to_string())
        .ok_or_else(|| LauncherError::from("XSTS 认证响应缺少 uhs"))
}




pub fn profile_id_to_uuid(hex_id: &str) -> Result<String, LauncherError> {
    let hex_id = hex_id.trim();
    if hex_id.len() != 32 {
        return Err(LauncherError::from(format!(
            "无效的 profile id 长度: 期望 32 个十六进制字符，实际 {} 个",
            hex_id.len()
        )));
    }
    if !hex_id.chars().all(|c| c.is_ascii_hexdigit()) {
        return Err(LauncherError::new("ERROR", "profile id 包含非十六进制字符"));
    }

    let lower = hex_id.to_ascii_lowercase();
    Ok(format!(
        "{}-{}-{}-{}-{}",
        &lower[0..8],
        &lower[8..12],
        &lower[12..16],
        &lower[16..20],
        &lower[20..32],
    ))
}

fn form_body(params: &[(&str, &str)]) -> String {
    let mut serializer = url::form_urlencoded::Serializer::new(String::new());
    for (key, value) in params {
        serializer.append_pair(key, value);
    }
    serializer.finish()
}



const MICROSOFT_CLIENT_ID: &str = "00000000402b5328";
const MICROSOFT_SCOPE: &str = "XboxLive.signin offline_access";


struct MicrosoftTokenResult {
    access_token: String,
    refresh_token: Option<String>,
}


async fn post_microsoft_token(
    params: &[(&str, &str)],
) -> Result<MicrosoftTokenResult, LauncherError> {
    let client = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(15))
        .build()
        .map_err(|e| format!("无法创建 HTTP 客户端: {e}"))?;

    let resp = client
        .post("https://login.microsoftonline.com/consumers/oauth2/v2.0/token")
        .header("Content-Type", "application/x-www-form-urlencoded")
        .body(form_body(params))
        .send()
        .await
        .map_err(|e| format!("无法连接 Microsoft 认证服务: {e}"))?;

    let status = resp.status();
    let resp_text = resp
        .text()
        .await
        .map_err(|e| format!("无法读取 Microsoft 认证响应: {e}"))?;

    if !status.is_success() {
        if let Ok(err) = serde_json::from_str::<MicrosoftTokenError>(&resp_text) {
            let message = map_microsoft_oauth_error(&err.error);
            if let Some(description) = err.error_description.filter(|s| !s.trim().is_empty()) {
                return Err(LauncherError::from(format!("{message}: {description}")));
            }
            return Err(message.into());
        }
        return Err(LauncherError::from(format!(
            "Microsoft 令牌请求失败 ({status})"
        )));
    }

    let parsed: MicrosoftTokenResponse =
        serde_json::from_str(&resp_text).map_err(|e| format!("Microsoft 令牌响应格式错误: {e}"))?;
    let MicrosoftTokenResponse {
        access_token,
        refresh_token,
    } = parsed;

    Ok(MicrosoftTokenResult {
        access_token,
        refresh_token,
    })
}


pub async fn start_microsoft_login_in() -> Result<MicrosoftDeviceAuthStartResult, LauncherError> {
    let client = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(15))
        .build()
        .map_err(|e| format!("无法创建 HTTP 客户端: {e}"))?;

    let params = [
        ("client_id", MICROSOFT_CLIENT_ID),
        ("scope", MICROSOFT_SCOPE),
    ];

    let resp = client
        .post("https://login.microsoftonline.com/consumers/oauth2/v2.0/devicecode")
        .header("Content-Type", "application/x-www-form-urlencoded")
        .body(form_body(&params))
        .send()
        .await
        .map_err(|e| format!("无法连接 Microsoft 认证服务: {e}"))?;

    let status = resp.status();
    let resp_text = resp
        .text()
        .await
        .map_err(|e| format!("无法读取 Microsoft 认证响应: {e}"))?;

    if !status.is_success() {

        if let Ok(err) = serde_json::from_str::<MicrosoftTokenError>(&resp_text) {
            return Err(map_microsoft_oauth_error(&err.error).into());
        }
        return Err(LauncherError::from(format!(
            "Microsoft 设备码请求失败 ({status})"
        )));
    }

    let parsed: MicrosoftDeviceCodeResponse = serde_json::from_str(&resp_text)
        .map_err(|e| format!("Microsoft 设备码响应格式错误: {e}"))?;

    Ok(MicrosoftDeviceAuthStartResult {
        device_code: parsed.device_code,
        user_code: parsed.user_code,
        verification_uri: parsed.verification_uri,
        verification_uri_complete: parsed.verification_uri_complete,
        expires_in: parsed.expires_in,
        interval: parsed.interval,
        message: parsed.message,
    })
}




pub async fn poll_microsoft_login_in(
    data_dir: &Path,
    device_code: &str,
) -> Result<MicrosoftLoginResult, LauncherError> {

    let ms_token = poll_microsoft_token(device_code).await?;


    let xbl_token = authenticate_xbox_live(&ms_token.access_token).await?;


    let xsts_token = authorize_xsts(&xbl_token.token, &xbl_token.uhs).await?;


    let mc_auth = login_minecraft_with_xbox(&xsts_token.token, &xsts_token.uhs).await?;


    let profile = get_minecraft_profile(&mc_auth.access_token).await?;


    let uuid = profile_id_to_uuid(&profile.id)?;


    let now = utils::now_iso8601();

    let token_expires_at = mc_auth.expires_in.map(|secs| {
        let expiry = chrono::Utc::now() + chrono::Duration::seconds(secs as i64);
        expiry.to_rfc3339()
    });


    let mut accounts = load_accounts(data_dir)?;



    let saved_account = if let Some(existing_index) = accounts
        .iter()
        .position(|a| a.kind == LauncherAccountKind::Microsoft && a.uuid == uuid)
    {
        let existing_id = accounts[existing_index].id.clone();
        for (index, account) in accounts.iter_mut().enumerate() {
            if index == existing_index {
                account.username = profile.name.clone();
                account.selected = true;
                account.last_validated_at = Some(now.clone());
                account.token_expires_at = token_expires_at.clone();
                account.updated_at = now.clone();
            } else if account.id != existing_id {
                account.selected = false;
            }
        }
        accounts[existing_index].clone()
    } else {
        let account_id = uuid::Uuid::new_v4().to_string();
        let new_account = LauncherAccount {
            id: account_id,
            kind: LauncherAccountKind::Microsoft,
            username: profile.name,
            uuid,
            selected: true,
            auth_server_url: None,
            avatar_url: None,
            created_at: now.clone(),
            updated_at: now.clone(),
            last_validated_at: Some(now),
            token_expires_at,
        };

        for a in accounts.iter_mut() {
            a.selected = false;
        }
        accounts.push(new_account.clone());
        new_account
    };

    save_accounts(data_dir, &accounts)?;


    if let Err(e) = save_account_token(data_dir, &saved_account.id, &mc_auth.access_token) {
        let _ = delete_account_from(data_dir, &saved_account.id);
        return Err(LauncherError::from(format!("无法保存账户凭据: {e}")));
    }


    let refresh_token_saved = if let Some(ref rt) = ms_token.refresh_token {
        save_microsoft_refresh_token(data_dir, &saved_account.id, rt).is_ok()
    } else {
        false
    };

    Ok(MicrosoftLoginResult {
        account: saved_account,
        token_saved: true,
        refresh_token_saved,
    })
}


struct XblTokenResult {
    token: String,
    uhs: String,
}


async fn poll_microsoft_token(device_code: &str) -> Result<MicrosoftTokenResult, LauncherError> {
    let params = [
        ("client_id", MICROSOFT_CLIENT_ID),
        ("grant_type", "urn:ietf:params:oauth:grant-type:device_code"),
        ("device_code", device_code),
    ];
    post_microsoft_token(&params).await
}


async fn authenticate_xbox_live(ms_token: &str) -> Result<XblTokenResult, LauncherError> {
    let client = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(15))
        .build()
        .map_err(|e| format!("无法创建 HTTP 客户端: {e}"))?;

    let body = serde_json::json!({
        "Properties": {
            "AuthMethod": "RPS",
            "SiteName": "user.auth.xboxlive.com",
            "RpsTicket": format!("d={ms_token}"),
        },
        "RelyingParty": "http://auth.xboxlive.com",
        "TokenType": "JWT",
    });

    let resp = client
        .post("https://user.auth.xboxlive.com/user/authenticate")
        .header("Content-Type", "application/json")
        .header("Accept", "application/json")
        .json(&body)
        .send()
        .await
        .map_err(|e| format!("无法连接 Xbox Live 认证服务: {e}"))?;

    let status = resp.status();
    let resp_text = resp
        .text()
        .await
        .map_err(|e| format!("无法读取 Xbox Live 认证响应: {e}"))?;

    if !status.is_success() {
        return Err(LauncherError::from(format!(
            "Xbox Live 认证失败 ({status}): {resp_text}"
        )));
    }

    let parsed: XboxAuthResponse =
        serde_json::from_str(&resp_text).map_err(|e| format!("Xbox Live 认证响应格式错误: {e}"))?;

    let uhs = parsed
        .display_claims
        .xui
        .into_iter()
        .next()
        .map(|xui| xui.uhs)
        .ok_or_else(|| "Xbox Live 认证响应缺少 uhs".to_string())?;

    Ok(XblTokenResult {
        token: parsed.token,
        uhs,
    })
}


async fn authorize_xsts(xbl_token: &str, _uhs: &str) -> Result<XblTokenResult, LauncherError> {
    let client = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(15))
        .build()
        .map_err(|e| format!("无法创建 HTTP 客户端: {e}"))?;

    let body = serde_json::json!({
        "Properties": {
            "SandboxId": "RETAIL",
            "UserTokens": [xbl_token],
        },
        "RelyingParty": "rp://api.minecraftservices.com/",
        "TokenType": "JWT",
    });

    let resp = client
        .post("https://xsts.auth.xboxlive.com/xsts/authorize")
        .header("Content-Type", "application/json")
        .header("Accept", "application/json")
        .json(&body)
        .send()
        .await
        .map_err(|e| format!("无法连接 XSTS 认证服务: {e}"))?;

    let status = resp.status();
    let resp_text = resp
        .text()
        .await
        .map_err(|e| format!("无法读取 XSTS 认证响应: {e}"))?;

    if !status.is_success() {

        if let Ok(val) = serde_json::from_str::<serde_json::Value>(&resp_text) {
            if let Some(err_code) = val["XErr"].as_i64() {
                let msg = match err_code {
                    2148916233 => {
                        "此 Microsoft 账户没有 Xbox Live 账号，请先在 Xbox 上创建".to_string()
                    }
                    2148916238 => "此账户为儿童账户，需要家长授权才能登录 Xbox Live".to_string(),
                    _ => format!("XSTS 认证失败 (XErr: {err_code})"),
                };
                return Err(LauncherError::from(msg));
            }
        }
        return Err(LauncherError::from(format!(
            "XSTS 认证失败 ({status}): {resp_text}"
        )));
    }

    let parsed: XboxAuthResponse =
        serde_json::from_str(&resp_text).map_err(|e| format!("XSTS 认证响应格式错误: {e}"))?;

    let uhs = parsed
        .display_claims
        .xui
        .into_iter()
        .next()
        .map(|xui| xui.uhs)
        .ok_or_else(|| "XSTS 认证响应缺少 uhs".to_string())?;

    Ok(XblTokenResult {
        token: parsed.token,
        uhs,
    })
}


async fn login_minecraft_with_xbox(
    xsts_token: &str,
    uhs: &str,
) -> Result<MinecraftAuthResponse, LauncherError> {
    let client = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(15))
        .build()
        .map_err(|e| format!("无法创建 HTTP 客户端: {e}"))?;

    let body = serde_json::json!({
        "identityToken": format!("XBL3.0 x={uhs};{xsts_token}"),
    });

    let resp = client
        .post("https://api.minecraftservices.com/authentication/login_with_xbox")
        .header("Content-Type", "application/json")
        .json(&body)
        .send()
        .await
        .map_err(|e| format!("无法连接 Minecraft 认证服务: {e}"))?;

    let status = resp.status();
    let resp_text = resp
        .text()
        .await
        .map_err(|e| format!("无法读取 Minecraft 认证响应: {e}"))?;

    if !status.is_success() {
        return Err(LauncherError::from(format!(
            "Minecraft 认证失败 ({status}): {resp_text}"
        )));
    }

    serde_json::from_str(&resp_text)
        .map_err(|e| LauncherError::from(format!("Minecraft 认证响应格式错误: {e}")))
}


async fn get_minecraft_profile(
    access_token: &str,
) -> Result<MinecraftProfileResponse, LauncherError> {
    let client = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(15))
        .build()
        .map_err(|e| format!("无法创建 HTTP 客户端: {e}"))?;

    let resp = client
        .get("https://api.minecraftservices.com/minecraft/profile")
        .header("Authorization", format!("Bearer {access_token}"))
        .send()
        .await
        .map_err(|e| format!("无法连接 Minecraft 个人资料服务: {e}"))?;

    let status = resp.status();
    let resp_text = resp
        .text()
        .await
        .map_err(|e| format!("无法读取 Minecraft 个人资料响应: {e}"))?;

    if !status.is_success() {

        if let Ok(err) = serde_json::from_str::<MinecraftProfileError>(&resp_text) {
            if !err.error.is_empty() {
                return Err(LauncherError::from(format!(
                    "获取 Minecraft 个人资料失败: {}",
                    err.error
                )));
            }
        }
        return Err(LauncherError::from(format!(
            "获取 Minecraft 个人资料失败 ({status}): 该 Microsoft 账户可能未拥有 Minecraft"
        )));
    }

    serde_json::from_str(&resp_text)
        .map_err(|e| LauncherError::from(format!("Minecraft 个人资料响应格式错误: {e}")))
}








fn validate_auth_server_url(raw: &str) -> Result<String, LauncherError> {
    let trimmed = raw.trim();
    if trimmed.is_empty() {
        return Err(LauncherError::new("ERROR", "认证服务器地址不能为空"));
    }
    if !trimmed.starts_with("http://") && !trimmed.starts_with("https://") {
        return Err(LauncherError::new(
            "ERROR",
            "认证服务器地址必须以 http:// 或 https:// 开头",
        ));
    }
    Ok(trimmed.trim_end_matches('/').to_string())
}




#[derive(Debug, serde::Deserialize)]
struct YggdrasilAuthResponse {
    #[serde(rename = "accessToken")]
    access_token: String,
    #[serde(rename = "selectedProfile")]
    selected_profile: YggdrasilProfile,
}

#[derive(Debug, serde::Deserialize)]
struct YggdrasilProfile {
    id: String,
    name: String,
}


#[derive(Debug, serde::Deserialize)]
struct YggdrasilErrorResponse {
    #[serde(rename = "error")]
    _error: String,
    #[serde(rename = "errorMessage")]
    error_message: Option<String>,
}









pub(crate) async fn authenticate_yggdrasil(
    auth_server_url: &str,
    username_or_email: &str,
    password: &str,
) -> Result<(String, String, String), LauncherError> {
    let url = format!("{auth_server_url}/authserver/authenticate");

    let body = serde_json::json!({
        "agent": { "name": "Minecraft", "version": 1 },
        "username": username_or_email,
        "password": password,
        "clientToken": "folly-launcher",
        "requestUser": false,
    });

    let client = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(15))
        .build()
        .map_err(|e| format!("无法创建 HTTP 客户端: {e}"))?;

    let resp = client
        .post(&url)
        .header("Content-Type", "application/json")
        .json(&body)
        .send()
        .await
        .map_err(|e| format!("无法连接认证服务器: {e}"))?;

    let status = resp.status();
    let resp_text = resp
        .text()
        .await
        .map_err(|e| format!("无法读取认证响应: {e}"))?;

    if !status.is_success() {

        if let Ok(err_resp) = serde_json::from_str::<YggdrasilErrorResponse>(&resp_text) {
            let msg = err_resp
                .error_message
                .unwrap_or_else(|| err_resp._error.clone());
            return Err(LauncherError::from(format!("认证失败 ({status}): {msg}")));
        }
        return Err(LauncherError::from(format!("认证失败 ({status})")));
    }

    let auth_resp: YggdrasilAuthResponse =
        serde_json::from_str(&resp_text).map_err(|e| format!("认证响应格式错误: {e}"))?;

    Ok((
        auth_resp.access_token,
        auth_resp.selected_profile.id,
        auth_resp.selected_profile.name,
    ))
}









fn generate_offline_uuid(username: &str) -> String {
    let input = format!("OfflinePlayer:{}", username);
    let digest = md5::Md5::digest(input.as_bytes());
    let mut bytes = digest.to_vec();


    bytes[6] = (bytes[6] & 0x0f) | 0x30;

    bytes[8] = (bytes[8] & 0x3f) | 0x80;

    format!(
        "{:02x}{:02x}{:02x}{:02x}-{:02x}{:02x}-{:02x}{:02x}-{:02x}{:02x}-{:02x}{:02x}{:02x}{:02x}{:02x}{:02x}",
        bytes[0],
        bytes[1],
        bytes[2],
        bytes[3],
        bytes[4],
        bytes[5],
        bytes[6],
        bytes[7],
        bytes[8],
        bytes[9],
        bytes[10],
        bytes[11],
        bytes[12],
        bytes[13],
        bytes[14],
        bytes[15],
    )
}






fn validate_username(username: &str) -> Result<String, LauncherError> {
    let trimmed = username.trim();

    if trimmed.is_empty() {
        return Err(LauncherError::new("ERROR", "玩家名称不能为空"));
    }
    if trimmed.len() < 3 {
        return Err(LauncherError::from(format!(
            "玩家名称长度不能小于 3 个字符（当前 {} 个字符）",
            trimmed.len()
        )));
    }
    if trimmed.len() > 16 {
        return Err(LauncherError::from(format!(
            "玩家名称长度不能超过 16 个字符（当前 {} 个字符）",
            trimmed.len()
        )));
    }

    for (i, ch) in trimmed.char_indices() {
        if !ch.is_ascii_alphanumeric() && ch != '_' {
            return Err(LauncherError::from(format!(
                "玩家名称包含非法字符 '{}'（位置 {}），仅允许字母、数字和下划线",
                ch,
                i + 1
            )));
        }
    }

    Ok(trimmed.to_string())
}





pub fn selected_account_in(data_dir: &Path) -> Result<LauncherAccount, LauncherError> {
    let accounts = load_accounts(data_dir)?;
    if accounts.is_empty() {
        return Err(LauncherError::new(
            "ERROR",
            "请先在「我的」页面添加或选择 Minecraft 账户",
        ));
    }

    if let Some(selected) = accounts.iter().find(|a| a.selected) {
        return Ok(selected.clone());
    }
    Err(LauncherError::new(
        "ERROR",
        "没有选中的 Minecraft 账户，请在「我的」页面选择一个账户",
    ))
}

fn load_accounts(data_dir: &Path) -> Result<Vec<LauncherAccount>, LauncherError> {
    let path = accounts_file_path(data_dir);
    if !path.exists() {
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent).map_err(|e| format!("无法创建账户数据目录: {e}"))?;
        }
        return Ok(Vec::new());
    }

    let content =
        std::fs::read_to_string(&path).map_err(|e| format!("无法读取账户数据文件: {e}"))?;

    if content.trim().is_empty() {
        return Ok(Vec::new());
    }

    serde_json::from_str(&content)
        .map_err(|e| LauncherError::from(format!("账户数据文件已损坏，无法解析: {e}")))
}

fn save_accounts(data_dir: &Path, accounts: &[LauncherAccount]) -> Result<(), LauncherError> {
    let path = accounts_file_path(data_dir);
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent).map_err(|e| format!("无法创建账户数据目录: {e}"))?;
    }

    let json =
        serde_json::to_string_pretty(accounts).map_err(|e| format!("无法序列化账户数据: {e}"))?;

    std::fs::write(&path, json)
        .map_err(|e| LauncherError::from(format!("无法写入账户数据文件: {e}")))
}

pub fn add_offline_account_in(
    data_dir: &Path,
    request: AddOfflineAccountRequest,
) -> Result<LauncherAccount, LauncherError> {
    let username = validate_username(&request.username)?;
    let offline_uuid = generate_offline_uuid(&username);
    let id = uuid::Uuid::new_v4().to_string();
    let now = utils::now_iso8601();

    let account = LauncherAccount {
        id,
        kind: LauncherAccountKind::Offline,
        username: username.clone(),
        uuid: offline_uuid,
        selected: true,
        auth_server_url: None,
        avatar_url: None,
        created_at: now.clone(),
        updated_at: now,
        last_validated_at: None,
        token_expires_at: None,
    };

    let mut accounts = load_accounts(data_dir)?;


    for existing in accounts.iter_mut() {
        existing.selected = false;
    }

    accounts.push(account.clone());
    save_accounts(data_dir, &accounts)?;

    Ok(account)
}

pub fn select_account_in(data_dir: &Path, id: &str) -> Result<LauncherAccount, LauncherError> {
    let mut accounts = load_accounts(data_dir)?;

    let found = accounts
        .iter()
        .position(|a| a.id == id)
        .ok_or_else(|| format!("未找到账户: {id}"))?;


    for account in accounts.iter_mut() {
        account.selected = false;
    }
    accounts[found].selected = true;

    let result = accounts[found].clone();
    save_accounts(data_dir, &accounts)?;

    Ok(result)
}

pub fn delete_account_from(data_dir: &Path, id: &str) -> Result<(), LauncherError> {
    let mut accounts = load_accounts(data_dir)?;

    let idx = accounts
        .iter()
        .position(|a| a.id == id)
        .ok_or_else(|| format!("未找到账户: {id}"))?;

    let was_selected = accounts[idx].selected;
    let account_kind = accounts[idx].kind.clone();
    accounts.remove(idx);



    if was_selected && !accounts.is_empty() {
        accounts[0].selected = true;
    }

    save_accounts(data_dir, &accounts)?;


    let _ = delete_account_token(data_dir, id);


    if account_kind == LauncherAccountKind::Microsoft {
        let _ = delete_microsoft_refresh_token(data_dir, id);
    }

    Ok(())
}

pub async fn add_third_party_account_in(
    data_dir: &Path,
    request: AddThirdPartyAccountRequest,
) -> Result<ThirdPartyLoginResult, LauncherError> {

    let auth_server_url = validate_auth_server_url(&request.auth_server_url)?;

    if request.username_or_email.trim().is_empty() {
        return Err(LauncherError::new("ERROR", "用户名/邮箱不能为空"));
    }
    if request.password.is_empty() {
        return Err(LauncherError::new("ERROR", "密码不能为空"));
    }


    let (access_token, profile_id, profile_name) = authenticate_yggdrasil(
        &auth_server_url,
        request.username_or_email.trim(),
        &request.password,
    )
    .await?;


    let now = utils::now_iso8601();
    let account_id = uuid::Uuid::new_v4().to_string();

    let account = LauncherAccount {
        id: account_id.clone(),
        kind: LauncherAccountKind::ThirdParty,
        username: profile_name,
        uuid: profile_id,
        selected: true,
        auth_server_url: Some(auth_server_url),
        avatar_url: None,
        created_at: now.clone(),
        updated_at: now.clone(),
        last_validated_at: Some(now),
        token_expires_at: None,
    };


    let mut accounts = load_accounts(data_dir)?;
    for existing in accounts.iter_mut() {
        existing.selected = false;
    }
    accounts.push(account.clone());
    save_accounts(data_dir, &accounts)?;


    if let Err(e) = save_account_token(data_dir, &account_id, &access_token) {

        let _ = delete_account_from(data_dir, &account_id);
        return Err(LauncherError::from(format!("无法保存账户凭据: {e}")));
    }

    Ok(ThirdPartyLoginResult {
        account,
        token_saved: true,
    })
}




async fn refresh_microsoft_token(
    refresh_token: &str,
) -> Result<MicrosoftTokenResult, LauncherError> {
    let params = [
        ("client_id", MICROSOFT_CLIENT_ID),
        ("scope", MICROSOFT_SCOPE),
        ("grant_type", "refresh_token"),
        ("refresh_token", refresh_token),
    ];
    post_microsoft_token(&params).await
}









pub async fn refresh_microsoft_account_in(
    data_dir: &Path,
    account_id: &str,
) -> Result<MicrosoftRefreshResult, LauncherError> {

    let mut accounts = load_accounts(data_dir)?;
    let account_index = accounts
        .iter()
        .position(|a| a.id == account_id)
        .ok_or_else(|| format!("未找到账户: {account_id}"))?;

    if accounts[account_index].kind != LauncherAccountKind::Microsoft {
        return Err(LauncherError::new(
            "ERROR",
            "该账户不是 Microsoft 账户，无法刷新",
        ));
    }

    let original_account = accounts[account_index].clone();


    let refresh_token = load_microsoft_refresh_token(data_dir, account_id)?
        .ok_or_else(|| "Microsoft 刷新凭据已失效，请重新登录".to_string())?;


    let ms_token = refresh_microsoft_token(&refresh_token).await?;


    let xbl_token = authenticate_xbox_live(&ms_token.access_token).await?;


    let xsts_token = authorize_xsts(&xbl_token.token, &xbl_token.uhs).await?;


    let mc_auth = login_minecraft_with_xbox(&xsts_token.token, &xsts_token.uhs).await?;


    let profile = get_minecraft_profile(&mc_auth.access_token).await?;


    let profile_uuid = profile_id_to_uuid(&profile.id)?;
    ensure_profile_matches_account(&profile_uuid, &original_account)?;


    let now = utils::now_iso8601();
    let token_expires_at = mc_auth.expires_in.map(|secs| {
        let expiry = chrono::Utc::now() + chrono::Duration::seconds(secs as i64);
        expiry.to_rfc3339()
    });


    for account in accounts.iter_mut() {
        account.selected = false;
    }
    accounts[account_index].username = profile.name;
    accounts[account_index].last_validated_at = Some(now.clone());
    accounts[account_index].token_expires_at = token_expires_at;
    accounts[account_index].updated_at = now;
    accounts[account_index].selected = true;

    let updated_account = accounts[account_index].clone();
    save_accounts(data_dir, &accounts)?;


    if let Err(e) = save_account_token(data_dir, account_id, &mc_auth.access_token) {
        return Err(LauncherError::from(format!("无法保存账户凭据: {e}")));
    }


    let refresh_token_saved = if let Some(ref new_rt) = ms_token.refresh_token {
        save_microsoft_refresh_token(data_dir, account_id, new_rt).is_ok()
    } else {

        true
    };

    Ok(MicrosoftRefreshResult {
        account: updated_account,
        token_saved: true,
        refresh_token_saved,
    })
}



#[tauri::command]
pub async fn list_launcher_accounts(
    state: State<'_, Arc<Mutex<AppState>>>,
) -> Result<Vec<LauncherAccount>, LauncherError> {
    let data_dir = {
        let s = state.lock().await;
        s.data_dir.clone()
    };
    load_accounts(&data_dir)
}

#[tauri::command]
pub async fn add_offline_account(
    state: State<'_, Arc<Mutex<AppState>>>,
    request: AddOfflineAccountRequest,
) -> Result<LauncherAccount, LauncherError> {
    let data_dir = {
        let s = state.lock().await;
        s.data_dir.clone()
    };
    add_offline_account_in(&data_dir, request)
}

#[tauri::command]
pub async fn select_launcher_account(
    state: State<'_, Arc<Mutex<AppState>>>,
    id: String,
) -> Result<LauncherAccount, LauncherError> {
    let data_dir = {
        let s = state.lock().await;
        s.data_dir.clone()
    };
    select_account_in(&data_dir, &id)
}

#[tauri::command]
pub async fn delete_launcher_account(
    state: State<'_, Arc<Mutex<AppState>>>,
    id: String,
) -> Result<(), LauncherError> {
    let data_dir = {
        let s = state.lock().await;
        s.data_dir.clone()
    };
    delete_account_from(&data_dir, &id)
}

#[tauri::command]
pub async fn add_third_party_account(
    state: State<'_, Arc<Mutex<AppState>>>,
    request: AddThirdPartyAccountRequest,
) -> Result<ThirdPartyLoginResult, LauncherError> {
    let data_dir = {
        let s = state.lock().await;
        s.data_dir.clone()
    };
    add_third_party_account_in(&data_dir, request).await
}

#[tauri::command]
pub async fn start_microsoft_login() -> Result<MicrosoftDeviceAuthStartResult, LauncherError> {
    start_microsoft_login_in().await
}

#[tauri::command]
pub async fn poll_microsoft_login(
    state: State<'_, Arc<Mutex<AppState>>>,
    device_code: String,
) -> Result<MicrosoftLoginResult, LauncherError> {
    let data_dir = {
        let s = state.lock().await;
        s.data_dir.clone()
    };
    poll_microsoft_login_in(&data_dir, &device_code).await
}

#[tauri::command]
pub async fn refresh_microsoft_account(
    state: State<'_, Arc<Mutex<AppState>>>,
    account_id: String,
) -> Result<MicrosoftRefreshResult, LauncherError> {
    let data_dir = {
        let s = state.lock().await;
        s.data_dir.clone()
    };
    refresh_microsoft_account_in(&data_dir, &account_id).await
}





pub fn default_avatar_url_for(account: &LauncherAccount) -> String {
    let uuid = account.uuid.trim();
    format!("https://crafatar.com/avatars/{uuid}?overlay")
}








fn validate_avatar_url(raw: &str) -> Result<String, LauncherError> {
    let trimmed = raw.trim();
    if trimmed.is_empty() {
        return Err(LauncherError::new("ERROR", "头像URL不能为空"));
    }
    if trimmed.contains('\0') {
        return Err(LauncherError::new("ERROR", "头像URL包含非法字符"));
    }
    if trimmed.len() > 2048 {
        return Err(LauncherError::from(format!(
            "头像URL长度不能超过2048个字符（当前{}个字符）",
            trimmed.len()
        )));
    }
    if !trimmed.starts_with("http://") && !trimmed.starts_with("https://") {
        return Err(LauncherError::new(
            "ERROR",
            "头像URL必须以 http:// 或 https:// 开头",
        ));
    }
    Ok(trimmed.to_string())
}






pub fn update_account_avatar_in(
    data_dir: &Path,
    request: UpdateAccountAvatarRequest,
) -> Result<AccountAvatarResult, LauncherError> {
    let mut accounts = load_accounts(data_dir)?;
    let idx = accounts
        .iter()
        .position(|a| a.id == request.account_id)
        .ok_or_else(|| format!("未找到账户: {}", request.account_id))?;

    match request.avatar_url {
        Some(raw) => {
            let validated = validate_avatar_url(&raw)?;
            accounts[idx].avatar_url = Some(validated.clone());
            accounts[idx].updated_at = utils::now_iso8601();
            save_accounts(data_dir, &accounts)?;
            Ok(AccountAvatarResult {
                account: accounts[idx].clone(),
                avatar_url: Some(validated),
            })
        }
        None => {
            accounts[idx].avatar_url = None;
            accounts[idx].updated_at = utils::now_iso8601();
            save_accounts(data_dir, &accounts)?;
            Ok(AccountAvatarResult {
                account: accounts[idx].clone(),
                avatar_url: None,
            })
        }
    }
}


pub fn refresh_account_avatar_in(
    data_dir: &Path,
    account_id: &str,
) -> Result<AccountAvatarResult, LauncherError> {
    let mut accounts = load_accounts(data_dir)?;
    let idx = accounts
        .iter()
        .position(|a| a.id == account_id)
        .ok_or_else(|| format!("未找到账户: {account_id}"))?;

    let avatar_url = default_avatar_url_for(&accounts[idx]);
    accounts[idx].avatar_url = Some(avatar_url.clone());
    accounts[idx].updated_at = utils::now_iso8601();
    save_accounts(data_dir, &accounts)?;
    Ok(AccountAvatarResult {
        account: accounts[idx].clone(),
        avatar_url: Some(avatar_url),
    })
}



#[tauri::command]
pub async fn update_account_avatar(
    state: State<'_, Arc<Mutex<AppState>>>,
    request: UpdateAccountAvatarRequest,
) -> Result<AccountAvatarResult, LauncherError> {
    let data_dir = {
        let s = state.lock().await;
        s.data_dir.clone()
    };
    update_account_avatar_in(&data_dir, request)
}

#[tauri::command]
pub async fn refresh_account_avatar(
    state: State<'_, Arc<Mutex<AppState>>>,
    account_id: String,
) -> Result<AccountAvatarResult, LauncherError> {
    let data_dir = {
        let s = state.lock().await;
        s.data_dir.clone()
    };
    refresh_account_avatar_in(&data_dir, &account_id)
}










pub fn export_accounts_in(data_dir: &Path) -> Result<AccountExportBundle, LauncherError> {
    let accounts = load_accounts(data_dir)?;
    let mut exported = Vec::with_capacity(accounts.len());
    for mut acc in accounts {
        acc.selected = false;
        exported.push(acc);
    }
    Ok(AccountExportBundle {
        schema_version: 1,
        exported_at: utils::now_iso8601(),
        accounts: exported,
    })
}














pub fn import_accounts_in(
    data_dir: &Path,
    request: ImportAccountsRequest,
) -> Result<ImportAccountsResult, LauncherError> {
    let bundle: AccountExportBundle =
        serde_json::from_str(&request.bundle_json).map_err(|e| format!("无法解析导入文件: {e}"))?;

    if bundle.schema_version != 1 {
        return Err(LauncherError::from(format!(
            "不支持的 schema 版本: {}（仅支持版本 1）",
            bundle.schema_version
        )));
    }

    let mut existing = load_accounts(data_dir)?;
    let now = utils::now_iso8601();

    let mut imported: u32 = 0;
    let mut skipped: u32 = 0;
    let mut failed: u32 = 0;
    let total = bundle.accounts.len() as u32;

    for mut acc in bundle.accounts {

        let username = acc.username.trim().to_string();
        let uuid = acc.uuid.trim().to_string();

        if username.is_empty() || uuid.is_empty() {
            failed += 1;
            continue;
        }
        acc.username = username;
        acc.uuid = uuid;


        if request.dedupe_by_uuid {
            let duplicate = existing
                .iter()
                .any(|e| e.kind == acc.kind && e.uuid == acc.uuid);
            if duplicate {
                skipped += 1;
                continue;
            }
        }


        if acc.id.is_empty() || existing.iter().any(|e| e.id == acc.id) {
            acc.id = uuid::Uuid::new_v4().to_string();
        }


        acc.selected = false;


        if acc.created_at.is_empty() {
            acc.created_at = now.clone();
        }
        if acc.updated_at.is_empty() {
            acc.updated_at = now.clone();
        }


        existing.push(acc);
        imported += 1;
    }

    save_accounts(data_dir, &existing)?;

    Ok(ImportAccountsResult {
        imported,
        skipped,
        failed,
        total,
    })
}



#[tauri::command]
pub async fn export_launcher_accounts(
    state: State<'_, Arc<Mutex<AppState>>>,
) -> Result<AccountExportBundle, LauncherError> {
    let data_dir = {
        let s = state.lock().await;
        s.data_dir.clone()
    };
    export_accounts_in(&data_dir)
}

#[tauri::command]
pub async fn import_launcher_accounts(
    state: State<'_, Arc<Mutex<AppState>>>,
    request: ImportAccountsRequest,
) -> Result<ImportAccountsResult, LauncherError> {
    let data_dir = {
        let s = state.lock().await;
        s.data_dir.clone()
    };
    import_accounts_in(&data_dir, request)
}







fn normalize_external_source(source: &str) -> Result<&'static str, LauncherError> {
    let lower = source.trim().to_lowercase();
    match lower.as_str() {
        "prism" | "multimc" | "prism-multimc" => Ok("prism-multimc"),
        other => Err(LauncherError::from(format!(
            "不支持的外部启动器来源: {other}，当前仅支持 prism、multimc、prism-multimc"
        ))),
    }
}






fn normalize_external_account_kind(raw: &str) -> Option<LauncherAccountKind> {
    let lower = raw.trim().to_lowercase();
    match lower.as_str() {
        "offline" => Some(LauncherAccountKind::Offline),
        "msa" | "microsoft" | "microsoft_account" => Some(LauncherAccountKind::Microsoft),
        _ => None,
    }
}







fn normalize_external_uuid(raw: &str) -> Result<String, LauncherError> {
    let raw = raw.trim();
    if raw.is_empty() {
        return Err(LauncherError::new("ERROR", "UUID 不能为空"));
    }


    if raw.len() == 32 && raw.chars().all(|c| c.is_ascii_hexdigit()) {
        return profile_id_to_uuid(raw);
    }


    let parsed = uuid::Uuid::parse_str(raw)
        .map_err(|_| format!("无效的 UUID 格式: '{raw}'，期望 32 位十六进制或标准 UUID 格式"))?;
    Ok(parsed.to_string())
}












#[cfg(test)]
fn parse_prism_multimc_accounts(json: &str) -> Result<Vec<LauncherAccount>, LauncherError> {
    let parsed: serde_json::Value =
        serde_json::from_str(json).map_err(|e| format!("无法解析 JSON: {e}"))?;

    let accounts_array = parsed
        .get("accounts")
        .and_then(|v| v.as_array())
        .ok_or_else(|| "accounts 字段必须是数组".to_string())?;

    let now = utils::now_iso8601();
    let mut result = Vec::new();

    for entry in accounts_array {

        let raw_type = entry.get("type").and_then(|v| v.as_str()).unwrap_or("");

        let kind = match normalize_external_account_kind(raw_type) {
            Some(k) => k,
            None => continue,
        };


        let username = entry
            .get("profile")
            .and_then(|v| v.get("name"))
            .and_then(|v| v.as_str())
            .or_else(|| entry.get("name").and_then(|v| v.as_str()))
            .unwrap_or("");
        let username = username.trim();
        if username.is_empty() {
            continue;
        }


        let raw_uuid = entry
            .get("profile")
            .and_then(|v| v.get("id"))
            .and_then(|v| v.as_str())
            .or_else(|| entry.get("uuid").and_then(|v| v.as_str()))
            .or_else(|| entry.get("id").and_then(|v| v.as_str()))
            .unwrap_or("");

        let uuid = match normalize_external_uuid(raw_uuid) {
            Ok(u) => u,
            Err(_) => continue,
        };

        result.push(LauncherAccount {
            id: uuid::Uuid::new_v4().to_string(),
            kind,
            username: username.to_string(),
            uuid,
            selected: false,
            auth_server_url: None,
            avatar_url: None,
            created_at: now.clone(),
            updated_at: now.clone(),
            last_validated_at: None,
            token_expires_at: None,
        });
    }

    Ok(result)
}














pub fn import_external_accounts_in(
    data_dir: &Path,
    request: ImportExternalAccountsRequest,
) -> Result<ImportExternalAccountsResult, LauncherError> {

    normalize_external_source(&request.source)?;


    let parsed: serde_json::Value =
        serde_json::from_str(&request.accounts_json).map_err(|e| format!("无法解析 JSON: {e}"))?;

    let accounts_array = parsed
        .get("accounts")
        .and_then(|v| v.as_array())
        .ok_or_else(|| "accounts 字段必须是数组".to_string())?;

    let total = accounts_array.len() as u32;


    let mut existing = load_accounts(data_dir)?;
    let now = utils::now_iso8601();

    let mut imported: u32 = 0;
    let mut skipped: u32 = 0;
    let mut failed: u32 = 0;

    for entry in accounts_array {

        let raw_type = entry.get("type").and_then(|v| v.as_str()).unwrap_or("");

        let kind = match normalize_external_account_kind(raw_type) {
            Some(k) => k,
            None => {
                skipped += 1;
                continue;
            }
        };


        let username = entry
            .get("profile")
            .and_then(|v| v.get("name"))
            .and_then(|v| v.as_str())
            .or_else(|| entry.get("name").and_then(|v| v.as_str()))
            .unwrap_or("");
        let username = username.trim();
        if username.is_empty() {
            failed += 1;
            continue;
        }


        let raw_uuid = entry
            .get("profile")
            .and_then(|v| v.get("id"))
            .and_then(|v| v.as_str())
            .or_else(|| entry.get("uuid").and_then(|v| v.as_str()))
            .or_else(|| entry.get("id").and_then(|v| v.as_str()))
            .unwrap_or("");

        let uuid = match normalize_external_uuid(raw_uuid) {
            Ok(u) => u,
            Err(_) => {
                failed += 1;
                continue;
            }
        };


        if request.dedupe_by_uuid {
            let duplicate = existing.iter().any(|e| e.kind == kind && e.uuid == uuid);
            if duplicate {
                skipped += 1;
                continue;
            }
        }


        existing.push(LauncherAccount {
            id: uuid::Uuid::new_v4().to_string(),
            kind,
            username: username.to_string(),
            uuid,
            selected: false,
            auth_server_url: None,
            avatar_url: None,
            created_at: now.clone(),
            updated_at: now.clone(),
            last_validated_at: None,
            token_expires_at: None,
        });
        imported += 1;
    }


    save_accounts(data_dir, &existing)?;

    Ok(ImportExternalAccountsResult {
        imported,
        skipped,
        failed,
        total,
    })
}

#[tauri::command]
pub async fn import_external_accounts(
    state: State<'_, Arc<Mutex<AppState>>>,
    request: ImportExternalAccountsRequest,
) -> Result<ImportExternalAccountsResult, LauncherError> {
    let data_dir = {
        let s = state.lock().await;
        s.data_dir.clone()
    };
    import_external_accounts_in(&data_dir, request)
}



#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    fn setup_temp_dir() -> TempDir {
        tempfile::tempdir().expect("failed to create temp dir")
    }



    fn make_test_account(
        kind: LauncherAccountKind,
        username: &str,
        uuid_str: &str,
    ) -> LauncherAccount {
        LauncherAccount {
            id: uuid::Uuid::new_v4().to_string(),
            kind,
            username: username.to_string(),
            uuid: uuid_str.to_string(),
            selected: false,
            auth_server_url: None,
            avatar_url: None,
            created_at: "2025-01-01T00:00:00Z".to_string(),
            updated_at: "2025-01-01T00:00:00Z".to_string(),
            last_validated_at: None,
            token_expires_at: None,
        }
    }

    fn export_bundle_json(accounts: Vec<LauncherAccount>) -> String {
        let bundle = AccountExportBundle {
            schema_version: 1,
            exported_at: "2025-05-04T00:00:00Z".to_string(),
            accounts,
        };
        serde_json::to_string(&bundle).unwrap()
    }



    #[test]
    fn export_bundle_no_tokens_or_passwords() {
        let dir = setup_temp_dir();
        let data_dir = dir.path();


        let account = LauncherAccount {
            id: "tp-export-1".to_string(),
            kind: LauncherAccountKind::ThirdParty,
            username: "ExportPlayer".to_string(),
            uuid: "exp-uuid-1".to_string(),
            selected: true,
            auth_server_url: Some("https://auth.example.com".to_string()),
            avatar_url: None,
            created_at: utils::now_iso8601(),
            updated_at: utils::now_iso8601(),
            last_validated_at: Some(utils::now_iso8601()),
            token_expires_at: None,
        };
        save_accounts(data_dir, std::slice::from_ref(&account)).unwrap();
        save_account_token(data_dir, &account.id, "secret-access-token").unwrap();

        let bundle = export_accounts_in(data_dir).unwrap();


        let json = serde_json::to_string(&bundle).unwrap();
        let lower = json.to_lowercase();
        assert!(
            !lower.contains("access_token"),
            "export must not contain access_token"
        );
        assert!(
            !lower.contains("refresh_token"),
            "export must not contain refresh_token"
        );
        assert!(
            !lower.contains("password"),
            "export must not contain password"
        );
        assert!(
            !lower.contains("device_code"),
            "export must not contain device_code"
        );
        assert!(
            !json.contains("secret-access-token"),
            "export must not contain actual token value"
        );
    }



    #[test]
    fn export_marks_all_accounts_unselected() {
        let dir = setup_temp_dir();
        let data_dir = dir.path();

        add_offline_account_in(
            data_dir,
            AddOfflineAccountRequest {
                username: "Player1".to_string(),
            },
        )
        .unwrap();
        add_offline_account_in(
            data_dir,
            AddOfflineAccountRequest {
                username: "Player2".to_string(),
            },
        )
        .unwrap();


        let loaded = load_accounts(data_dir).unwrap();
        assert!(loaded.iter().any(|a| a.selected));

        let bundle = export_accounts_in(data_dir).unwrap();
        for acc in &bundle.accounts {
            assert!(
                !acc.selected,
                "all exported accounts must have selected=false"
            );
        }


        let loaded2 = load_accounts(data_dir).unwrap();
        assert!(
            loaded2.iter().any(|a| a.selected),
            "original selection should be preserved"
        );
    }



    #[test]
    fn import_rejects_wrong_schema() {
        let dir = setup_temp_dir();
        let data_dir = dir.path();

        let json = serde_json::json!({
            "schema_version": 2,
            "exported_at": "2025-05-04T00:00:00Z",
            "accounts": []
        })
        .to_string();

        let req = ImportAccountsRequest {
            bundle_json: json,
            dedupe_by_uuid: false,
        };
        let err = import_accounts_in(data_dir, req).unwrap_err();
        assert!(err.contains("schema"), "error should mention schema: {err}");
        assert!(
            err.contains("版本 1"),
            "error should mention version 1: {err}"
        );
    }



    #[test]
    fn import_adds_accounts_without_tokens() {
        let dir = setup_temp_dir();
        let data_dir = dir.path();


        let existing = make_test_account(LauncherAccountKind::Offline, "OldPlayer", "old-uuid-1");
        save_accounts(data_dir, std::slice::from_ref(&existing)).unwrap();
        save_account_token(data_dir, &existing.id, "existing-token").unwrap();


        let imported = make_test_account(LauncherAccountKind::Offline, "NewPlayer", "new-uuid-1");
        let json = export_bundle_json(vec![imported.clone()]);
        let req = ImportAccountsRequest {
            bundle_json: json,
            dedupe_by_uuid: false,
        };
        let result = import_accounts_in(data_dir, req).unwrap();

        assert_eq!(result.imported, 1);
        assert_eq!(result.skipped, 0);
        assert_eq!(result.failed, 0);
        assert_eq!(result.total, 1);


        assert!(
            load_account_token(data_dir, &existing.id)
                .unwrap()
                .is_some(),
            "existing token should be preserved"
        );


        let all_tokens = load_all_account_tokens(data_dir).unwrap();
        assert_eq!(all_tokens.len(), 1, "no new token should have been saved");


        let accounts = load_accounts(data_dir).unwrap();
        assert_eq!(accounts.len(), 2);
    }



    #[test]
    fn import_dedupe_by_kind_and_uuid() {
        let dir = setup_temp_dir();
        let data_dir = dir.path();

        let existing = make_test_account(LauncherAccountKind::Offline, "DupPlayer", "dup-uuid");
        save_accounts(data_dir, std::slice::from_ref(&existing)).unwrap();


        let dupe = make_test_account(LauncherAccountKind::Offline, "DupPlayer", "dup-uuid");
        let json = export_bundle_json(vec![dupe]);
        let req = ImportAccountsRequest {
            bundle_json: json,
            dedupe_by_uuid: true,
        };
        let result = import_accounts_in(data_dir, req).unwrap();

        assert_eq!(result.imported, 0);
        assert_eq!(result.skipped, 1);
        assert_eq!(result.failed, 0);

        let accounts = load_accounts(data_dir).unwrap();
        assert_eq!(accounts.len(), 1);
    }

    #[test]
    fn import_no_dedupe_when_flag_false() {
        let dir = setup_temp_dir();
        let data_dir = dir.path();

        let existing = make_test_account(LauncherAccountKind::Offline, "NoDupPlayer", "nodup-uuid");
        save_accounts(data_dir, std::slice::from_ref(&existing)).unwrap();

        let dupe = make_test_account(LauncherAccountKind::Offline, "NoDupPlayer", "nodup-uuid");
        let json = export_bundle_json(vec![dupe]);
        let req = ImportAccountsRequest {
            bundle_json: json,
            dedupe_by_uuid: false,
        };
        let result = import_accounts_in(data_dir, req).unwrap();

        assert_eq!(result.imported, 1);
        assert_eq!(result.skipped, 0);
        assert_eq!(result.failed, 0);

        let accounts = load_accounts(data_dir).unwrap();
        assert_eq!(
            accounts.len(),
            2,
            "dedupe disabled should allow duplicate kind+uuid"
        );
    }



    #[test]
    fn import_generates_new_id_on_collision() {
        let dir = setup_temp_dir();
        let data_dir = dir.path();

        let existing = LauncherAccount {
            id: "fixed-id".to_string(),
            kind: LauncherAccountKind::Offline,
            username: "Collision".to_string(),
            uuid: "coll-uuid".to_string(),
            selected: false,
            auth_server_url: None,
            avatar_url: None,
            created_at: "2025-01-01T00:00:00Z".to_string(),
            updated_at: "2025-01-01T00:00:00Z".to_string(),
            last_validated_at: None,
            token_expires_at: None,
        };
        save_accounts(data_dir, std::slice::from_ref(&existing)).unwrap();

        let mut collider =
            make_test_account(LauncherAccountKind::Offline, "NewCollider", "collider-uuid");
        collider.id = "fixed-id".to_string();

        let json = export_bundle_json(vec![collider]);
        let req = ImportAccountsRequest {
            bundle_json: json,
            dedupe_by_uuid: false,
        };
        let result = import_accounts_in(data_dir, req).unwrap();
        assert_eq!(result.imported, 1);

        let accounts = load_accounts(data_dir).unwrap();
        assert_eq!(accounts.len(), 2);

        let imported = accounts
            .iter()
            .find(|a| a.username == "NewCollider")
            .unwrap();
        assert_ne!(imported.id, "fixed-id");
        assert!(!imported.id.is_empty());
        assert_eq!(accounts[0].id, "fixed-id");
    }

    #[test]
    fn import_generates_new_id_when_empty() {
        let dir = setup_temp_dir();
        let data_dir = dir.path();

        let mut empty_id =
            make_test_account(LauncherAccountKind::Offline, "EmptyId", "empty-id-uuid");
        empty_id.id = String::new();

        let json = export_bundle_json(vec![empty_id]);
        let req = ImportAccountsRequest {
            bundle_json: json,
            dedupe_by_uuid: false,
        };
        let result = import_accounts_in(data_dir, req).unwrap();
        assert_eq!(result.imported, 1);

        let accounts = load_accounts(data_dir).unwrap();
        assert_eq!(accounts.len(), 1);
        assert!(!accounts[0].id.is_empty());
    }



    #[test]
    fn import_fills_missing_timestamps() {
        let dir = setup_temp_dir();
        let data_dir = dir.path();

        let mut acc = make_test_account(LauncherAccountKind::Offline, "NoTime", "no-time-uuid");
        acc.created_at = String::new();
        acc.updated_at = String::new();

        let json = export_bundle_json(vec![acc]);
        let req = ImportAccountsRequest {
            bundle_json: json,
            dedupe_by_uuid: false,
        };
        let result = import_accounts_in(data_dir, req).unwrap();
        assert_eq!(result.imported, 1);

        let accounts = load_accounts(data_dir).unwrap();
        assert!(
            !accounts[0].created_at.is_empty(),
            "created_at should be filled"
        );
        assert!(
            !accounts[0].updated_at.is_empty(),
            "updated_at should be filled"
        );
    }

    #[test]
    fn import_preserves_existing_timestamps() {
        let dir = setup_temp_dir();
        let data_dir = dir.path();

        let acc = make_test_account(LauncherAccountKind::Offline, "HasTime", "has-time-uuid");
        let orig_created = acc.created_at.clone();
        let orig_updated = acc.updated_at.clone();

        let json = export_bundle_json(vec![acc]);
        let req = ImportAccountsRequest {
            bundle_json: json,
            dedupe_by_uuid: false,
        };
        import_accounts_in(data_dir, req).unwrap();

        let accounts = load_accounts(data_dir).unwrap();
        assert_eq!(accounts[0].created_at, orig_created);
        assert_eq!(accounts[0].updated_at, orig_updated);
    }



    #[test]
    fn import_preserves_existing_accounts() {
        let dir = setup_temp_dir();
        let data_dir = dir.path();

        let existing = add_offline_account_in(
            data_dir,
            AddOfflineAccountRequest {
                username: "Existing".to_string(),
            },
        )
        .unwrap();

        let imported = make_test_account(LauncherAccountKind::Offline, "Imported", "imported-uuid");
        let json = export_bundle_json(vec![imported]);
        let req = ImportAccountsRequest {
            bundle_json: json,
            dedupe_by_uuid: false,
        };
        import_accounts_in(data_dir, req).unwrap();

        let accounts = load_accounts(data_dir).unwrap();
        assert_eq!(accounts.len(), 2);
        assert!(
            accounts.iter().any(|a| a.id == existing.id),
            "existing account preserved"
        );
        assert!(
            accounts.iter().any(|a| a.username == "Imported"),
            "imported account added"
        );
    }



    #[test]
    fn import_never_selects_imported_accounts() {
        let dir = setup_temp_dir();
        let data_dir = dir.path();


        let existing = add_offline_account_in(
            data_dir,
            AddOfflineAccountRequest {
                username: "Existing".to_string(),
            },
        )
        .unwrap();


        let mut imp1 = make_test_account(LauncherAccountKind::Offline, "Imp1", "imp1-uuid");
        imp1.selected = true;
        let mut imp2 = make_test_account(LauncherAccountKind::Microsoft, "Imp2", "imp2-uuid");
        imp2.selected = true;

        let json = export_bundle_json(vec![imp1, imp2]);
        let req = ImportAccountsRequest {
            bundle_json: json,
            dedupe_by_uuid: false,
        };
        import_accounts_in(data_dir, req).unwrap();

        let accounts = load_accounts(data_dir).unwrap();

        let selected_count = accounts.iter().filter(|a| a.selected).count();
        assert_eq!(selected_count, 1, "only existing should remain selected");
        assert!(
            accounts
                .iter()
                .find(|a| a.id == existing.id)
                .unwrap()
                .selected
        );


        for a in &accounts {
            if a.id != existing.id {
                assert!(!a.selected, "imported account should not be selected");
            }
        }
    }



    #[test]
    fn import_rejects_empty_username() {
        let dir = setup_temp_dir();
        let data_dir = dir.path();

        let mut acc = make_test_account(LauncherAccountKind::Offline, "WillBeEmpty", "valid-uuid");
        acc.username = "  ".to_string();

        let json = export_bundle_json(vec![acc]);
        let req = ImportAccountsRequest {
            bundle_json: json,
            dedupe_by_uuid: false,
        };
        let result = import_accounts_in(data_dir, req).unwrap();
        assert_eq!(result.failed, 1);
        assert_eq!(result.imported, 0);
        assert_eq!(load_accounts(data_dir).unwrap().len(), 0);
    }

    #[test]
    fn import_rejects_empty_uuid() {
        let dir = setup_temp_dir();
        let data_dir = dir.path();

        let mut acc = make_test_account(LauncherAccountKind::Offline, "ValidName", "WillBeEmpty");
        acc.uuid = "  ".to_string();

        let json = export_bundle_json(vec![acc]);
        let req = ImportAccountsRequest {
            bundle_json: json,
            dedupe_by_uuid: false,
        };
        let result = import_accounts_in(data_dir, req).unwrap();
        assert_eq!(result.failed, 1);
        assert_eq!(result.imported, 0);
    }



    #[test]
    fn import_rejects_invalid_json() {
        let dir = setup_temp_dir();
        let data_dir = dir.path();

        let req = ImportAccountsRequest {
            bundle_json: "not valid json {{{".to_string(),
            dedupe_by_uuid: false,
        };
        let err = import_accounts_in(data_dir, req).unwrap_err();
        assert!(
            err.contains("无法解析"),
            "error should mention parse failure: {err}"
        );
    }



    #[test]
    fn import_mixed_kinds_all_succeed() {
        let dir = setup_temp_dir();
        let data_dir = dir.path();

        let offline = make_test_account(LauncherAccountKind::Offline, "OffPlayer", "off-uuid");
        let ms = make_test_account(LauncherAccountKind::Microsoft, "MsPlayer", "ms-uuid");
        let tp = LauncherAccount {
            id: uuid::Uuid::new_v4().to_string(),
            kind: LauncherAccountKind::ThirdParty,
            username: "TpPlayer".to_string(),
            uuid: "tp-uuid".to_string(),
            selected: false,
            auth_server_url: Some("https://auth.example.com".to_string()),
            avatar_url: None,
            created_at: "2025-01-01T00:00:00Z".to_string(),
            updated_at: "2025-01-01T00:00:00Z".to_string(),
            last_validated_at: None,
            token_expires_at: None,
        };

        let json = export_bundle_json(vec![offline, ms, tp]);
        let req = ImportAccountsRequest {
            bundle_json: json,
            dedupe_by_uuid: false,
        };
        let result = import_accounts_in(data_dir, req).unwrap();
        assert_eq!(result.imported, 3);
        assert_eq!(result.failed, 0);

        let accounts = load_accounts(data_dir).unwrap();
        assert_eq!(accounts.len(), 3);
    }





    #[test]
    fn offline_uuid_is_stable_for_same_username() {
        let u1 = generate_offline_uuid("Steve");
        let u2 = generate_offline_uuid("Steve");
        assert_eq!(u1, u2, "same username must yield same UUID");

        let u3 = generate_offline_uuid("Alex");
        assert_ne!(u1, u3, "different usernames must yield different UUIDs");
    }

    #[test]
    fn offline_uuid_has_version_3_and_rfc4122_variant() {
        let uuid = generate_offline_uuid("TestPlayer");
        let parts: Vec<&str> = uuid.split('-').collect();
        assert_eq!(parts.len(), 5, "must be standard UUID format");


        let version_char = parts[2].chars().next().unwrap();
        assert_eq!(version_char, '3', "UUID must be version 3 (name-based)");


        let variant_char = parts[3].chars().next().unwrap();
        assert!(
            matches!(variant_char, '8' | '9' | 'a' | 'b' | 'A' | 'B'),
            "UUID variant must be RFC 4122 (10xx), got '{variant_char}'"
        );
    }

    #[test]
    fn offline_uuid_matches_java_reference_for_steve() {



        let uuid = generate_offline_uuid("Steve");
        assert_eq!(
            uuid, "5627dd98-e6be-3c21-b8a8-e92344183641",
            "UUID must match Java reference value byte-for-byte"
        );
    }

    #[test]
    fn offline_uuid_matches_java_reference_for_notch() {

        let uuid = generate_offline_uuid("Notch");
        assert_eq!(
            uuid, "b50ad385-829d-3141-a216-7e7d7539ba7f",
            "UUID must match Java reference value byte-for-byte"
        );
    }



    #[test]
    fn validate_username_empty() {
        let err = validate_username("   ").unwrap_err();
        assert!(
            err.contains("不能为空"),
            "error should mention empty: {err}"
        );
    }

    #[test]
    fn validate_username_too_short() {
        let err = validate_username("ab").unwrap_err();
        assert!(
            err.contains("不能小于"),
            "error should mention too short: {err}"
        );
    }

    #[test]
    fn validate_username_too_long() {
        let err = validate_username("abcdefghijklmnopqrstu").unwrap_err();
        assert!(
            err.contains("不能超过"),
            "error should mention too long: {err}"
        );
    }

    #[test]
    fn validate_username_invalid_chars() {
        let err = validate_username("player!name").unwrap_err();
        assert!(
            err.contains("非法字符"),
            "error should mention invalid chars: {err}"
        );
    }

    #[test]
    fn validate_username_valid() {
        let result = validate_username("Player_123").expect("should succeed");
        assert_eq!(result, "Player_123");
    }

    #[test]
    fn validate_username_trims_whitespace() {
        let result = validate_username("  Steve  ").expect("should succeed");
        assert_eq!(result, "Steve");
    }

    #[test]
    fn validate_username_chinese_chars_rejected() {
        let err = validate_username("玩家名称").unwrap_err();
        assert!(
            err.contains("非法字符"),
            "Chinese characters should be rejected: {err}"
        );
    }



    #[test]
    fn empty_repo_returns_empty_list_and_creates_parent_dir() {
        let dir = setup_temp_dir();
        let data_dir = dir.path();

        let result = load_accounts(data_dir).expect("should succeed");
        assert!(result.is_empty());

        let accounts_dir = data_dir.join("accounts");
        assert!(accounts_dir.exists(), "accounts dir should be created");
        assert!(accounts_dir.is_dir(), "accounts path should be a directory");
    }



    #[test]
    fn add_offline_account_generates_uuid_and_selected() {
        let dir = setup_temp_dir();
        let data_dir = dir.path();

        let request = AddOfflineAccountRequest {
            username: "Steve".to_string(),
        };

        let account = add_offline_account_in(data_dir, request).expect("should succeed");

        assert!(!account.id.is_empty(), "id should not be empty");
        assert_eq!(account.kind, LauncherAccountKind::Offline);
        assert_eq!(account.username, "Steve");
        assert!(account.selected, "new account should be selected");


        let expected_uuid = generate_offline_uuid("Steve");
        assert_eq!(account.uuid, expected_uuid);


        let loaded = load_accounts(data_dir).expect("should load");
        assert_eq!(loaded.len(), 1);
        assert_eq!(loaded[0].id, account.id);
    }

    #[test]
    fn add_offline_account_deselects_old_selected() {
        let dir = setup_temp_dir();
        let data_dir = dir.path();


        let req1 = AddOfflineAccountRequest {
            username: "PlayerA".to_string(),
        };
        let a1 = add_offline_account_in(data_dir, req1).expect("should succeed");
        assert!(a1.selected);


        let req2 = AddOfflineAccountRequest {
            username: "PlayerB".to_string(),
        };
        let a2 = add_offline_account_in(data_dir, req2).expect("should succeed");
        assert!(a2.selected);

        let loaded = load_accounts(data_dir).expect("should load");
        assert_eq!(loaded.len(), 2);

        assert!(!loaded.iter().find(|a| a.id == a1.id).unwrap().selected);

        assert!(loaded.iter().find(|a| a.id == a2.id).unwrap().selected);
    }

    #[test]
    fn add_offline_account_empty_username_errors() {
        let dir = setup_temp_dir();
        let data_dir = dir.path();

        let request = AddOfflineAccountRequest {
            username: "   ".to_string(),
        };

        let err = add_offline_account_in(data_dir, request).unwrap_err();
        assert!(
            err.contains("不能为空"),
            "error should mention empty: {err}"
        );
    }

    #[test]
    fn add_offline_account_short_username_errors() {
        let dir = setup_temp_dir();
        let data_dir = dir.path();

        let request = AddOfflineAccountRequest {
            username: "ab".to_string(),
        };

        let err = add_offline_account_in(data_dir, request).unwrap_err();
        assert!(
            err.contains("不能小于"),
            "error should mention too short: {err}"
        );
    }

    #[test]
    fn add_offline_account_long_username_errors() {
        let dir = setup_temp_dir();
        let data_dir = dir.path();

        let request = AddOfflineAccountRequest {
            username: "a".repeat(20),
        };

        let err = add_offline_account_in(data_dir, request).unwrap_err();
        assert!(
            err.contains("不能超过"),
            "error should mention too long: {err}"
        );
    }

    #[test]
    fn add_offline_account_invalid_chars_errors() {
        let dir = setup_temp_dir();
        let data_dir = dir.path();

        let request = AddOfflineAccountRequest {
            username: "bad-player!".to_string(),
        };

        let err = add_offline_account_in(data_dir, request).unwrap_err();
        assert!(
            err.contains("非法字符"),
            "error should mention invalid chars: {err}"
        );
    }

    #[test]
    fn add_offline_account_with_underscores_ok() {
        let dir = setup_temp_dir();
        let data_dir = dir.path();

        let request = AddOfflineAccountRequest {
            username: "Test_Player_1".to_string(),
        };

        let account = add_offline_account_in(data_dir, request).expect("should succeed");
        assert_eq!(account.username, "Test_Player_1");
    }

    #[test]
    fn add_offline_account_trims_whitespace() {
        let dir = setup_temp_dir();
        let data_dir = dir.path();

        let request = AddOfflineAccountRequest {
            username: "  Steve  ".to_string(),
        };

        let account = add_offline_account_in(data_dir, request).expect("should succeed");
        assert_eq!(account.username, "Steve");
    }



    #[test]
    fn select_account_sets_only_one_selected() {
        let dir = setup_temp_dir();
        let data_dir = dir.path();


        let a1 = add_offline_account_in(
            data_dir,
            AddOfflineAccountRequest {
                username: "PlayerA".to_string(),
            },
        )
        .expect("should create");
        let a2 = add_offline_account_in(
            data_dir,
            AddOfflineAccountRequest {
                username: "PlayerB".to_string(),
            },
        )
        .expect("should create");


        let selected = select_account_in(data_dir, &a1.id).expect("should select");
        assert_eq!(selected.id, a1.id);
        assert!(selected.selected);

        let loaded = load_accounts(data_dir).expect("should load");
        assert!(loaded.iter().find(|a| a.id == a1.id).unwrap().selected);
        assert!(!loaded.iter().find(|a| a.id == a2.id).unwrap().selected);
    }

    #[test]
    fn select_nonexistent_id_errors() {
        let dir = setup_temp_dir();
        let data_dir = dir.path();

        let err = select_account_in(data_dir, "nonexistent-id").unwrap_err();
        assert!(
            err.contains("未找到"),
            "error should mention not found: {err}"
        );
    }



    #[test]
    fn delete_account_removes_record() {
        let dir = setup_temp_dir();
        let data_dir = dir.path();

        let account = add_offline_account_in(
            data_dir,
            AddOfflineAccountRequest {
                username: "Steve".to_string(),
            },
        )
        .expect("should create");

        delete_account_from(data_dir, &account.id).expect("should delete");

        let loaded = load_accounts(data_dir).expect("should load");
        assert!(loaded.is_empty(), "accounts list should be empty");
    }

    #[test]
    fn delete_selected_auto_selects_first_remaining() {
        let dir = setup_temp_dir();
        let data_dir = dir.path();


        let a1 = add_offline_account_in(
            data_dir,
            AddOfflineAccountRequest {
                username: "PlayerA".to_string(),
            },
        )
        .expect("should create");
        let a2 = add_offline_account_in(
            data_dir,
            AddOfflineAccountRequest {
                username: "PlayerB".to_string(),
            },
        )
        .expect("should create");
        let _a3 = add_offline_account_in(
            data_dir,
            AddOfflineAccountRequest {
                username: "PlayerC".to_string(),
            },
        )
        .expect("should create");


        select_account_in(data_dir, &a2.id).expect("should select");


        delete_account_from(data_dir, &a2.id).expect("should delete");

        let loaded = load_accounts(data_dir).expect("should load");
        assert_eq!(loaded.len(), 2, "should have 2 remaining accounts");


        assert!(
            loaded.iter().find(|a| a.id == a1.id).unwrap().selected,
            "first remaining account should be auto-selected"
        );
    }

    #[test]
    fn delete_nonselected_account_does_not_change_selection() {
        let dir = setup_temp_dir();
        let data_dir = dir.path();

        let a1 = add_offline_account_in(
            data_dir,
            AddOfflineAccountRequest {
                username: "PlayerA".to_string(),
            },
        )
        .expect("should create");
        let a2 = add_offline_account_in(
            data_dir,
            AddOfflineAccountRequest {
                username: "PlayerB".to_string(),
            },
        )
        .expect("should create");


        delete_account_from(data_dir, &a1.id).expect("should delete");

        let loaded = load_accounts(data_dir).expect("should load");
        assert_eq!(loaded.len(), 1);
        assert!(loaded[0].selected, "a2 should still be selected");
        assert_eq!(loaded[0].id, a2.id);
    }

    #[test]
    fn delete_last_account_works() {
        let dir = setup_temp_dir();
        let data_dir = dir.path();

        let account = add_offline_account_in(
            data_dir,
            AddOfflineAccountRequest {
                username: "SoloPlayer".to_string(),
            },
        )
        .expect("should create");

        delete_account_from(data_dir, &account.id).expect("should delete");

        let loaded = load_accounts(data_dir).expect("should load");
        assert!(loaded.is_empty());
    }

    #[test]
    fn delete_nonexistent_id_errors() {
        let dir = setup_temp_dir();
        let data_dir = dir.path();

        let err = delete_account_from(data_dir, "nonexistent-id").unwrap_err();
        assert!(
            err.contains("未找到"),
            "error should mention not found: {err}"
        );
    }



    #[test]
    fn corrupted_json_returns_error() {
        let dir = setup_temp_dir();
        let data_dir = dir.path();

        let json_path = accounts_file_path(data_dir);
        std::fs::create_dir_all(json_path.parent().unwrap()).expect("should create dir");
        std::fs::write(&json_path, "this is not valid json{{{").expect("should write");

        let err = load_accounts(data_dir).unwrap_err();
        assert!(
            err.contains("损坏") || err.contains("无法解析"),
            "error should indicate corruption: {err}"
        );
    }



    #[test]
    fn multiple_accounts_persistence() {
        let dir = setup_temp_dir();
        let data_dir = dir.path();

        let a1 = add_offline_account_in(
            data_dir,
            AddOfflineAccountRequest {
                username: "PlayerA".to_string(),
            },
        )
        .expect("should create");
        let a2 = add_offline_account_in(
            data_dir,
            AddOfflineAccountRequest {
                username: "PlayerB".to_string(),
            },
        )
        .expect("should create");

        let loaded = load_accounts(data_dir).expect("should load");
        assert_eq!(loaded.len(), 2);
        assert_ne!(a1.id, a2.id, "ids should be unique");

        delete_account_from(data_dir, &a1.id).expect("should delete");

        let loaded = load_accounts(data_dir).expect("should load");
        assert_eq!(loaded.len(), 1);
        assert_eq!(loaded[0].id, a2.id);
    }

    #[test]
    fn select_keeps_only_one_selected_after_multiple_adds() {
        let dir = setup_temp_dir();
        let data_dir = dir.path();

        for i in 1..=5 {
            add_offline_account_in(
                data_dir,
                AddOfflineAccountRequest {
                    username: format!("Player{i}"),
                },
            )
            .expect("should create");
        }

        let loaded = load_accounts(data_dir).expect("should load");
        let selected_count = loaded.iter().filter(|a| a.selected).count();
        assert_eq!(selected_count, 1, "exactly one account should be selected");
    }



    #[test]
    fn selected_account_in_empty_repo_returns_error() {
        let dir = setup_temp_dir();
        let data_dir = dir.path();

        let err = selected_account_in(data_dir).unwrap_err();
        assert!(
            err.contains("请先在"),
            "error should mention adding account: {err}"
        );
    }

    #[test]
    fn selected_account_in_returns_selected() {
        let dir = setup_temp_dir();
        let data_dir = dir.path();


        let a1 = add_offline_account_in(
            data_dir,
            AddOfflineAccountRequest {
                username: "PlayerA".to_string(),
            },
        )
        .expect("should create");
        let a2 = add_offline_account_in(
            data_dir,
            AddOfflineAccountRequest {
                username: "PlayerB".to_string(),
            },
        )
        .expect("should create");


        let selected = selected_account_in(data_dir).expect("should find selected");
        assert_eq!(selected.id, a2.id);
        assert!(selected.selected);


        select_account_in(data_dir, &a1.id).expect("should select a1");

        let selected = selected_account_in(data_dir).expect("should find selected");
        assert_eq!(selected.id, a1.id);
        assert!(selected.selected);
    }

    #[test]
    fn selected_account_in_errors_if_none_selected() {
        let dir = setup_temp_dir();
        let data_dir = dir.path();


        let a1 = LauncherAccount {
            id: "id-a".to_string(),
            kind: LauncherAccountKind::Offline,
            username: "PlayerA".to_string(),
            uuid: generate_offline_uuid("PlayerA"),
            selected: false,
            auth_server_url: None,
            avatar_url: None,
            created_at: utils::now_iso8601(),
            updated_at: utils::now_iso8601(),
            last_validated_at: None,
            token_expires_at: None,
        };
        let a2 = LauncherAccount {
            id: "id-b".to_string(),
            kind: LauncherAccountKind::Offline,
            username: "PlayerB".to_string(),
            uuid: generate_offline_uuid("PlayerB"),
            selected: false,
            auth_server_url: None,
            avatar_url: None,
            created_at: utils::now_iso8601(),
            updated_at: utils::now_iso8601(),
            last_validated_at: None,
            token_expires_at: None,
        };

        save_accounts(data_dir, &[a1.clone(), a2.clone()]).expect("should save");

        let err = selected_account_in(data_dir).unwrap_err();
        assert!(
            err.contains("没有选中"),
            "error should mention missing selection: {err}"
        );
    }



    #[test]
    fn validate_url_empty() {
        let err = validate_auth_server_url("   ").unwrap_err();
        assert!(
            err.contains("不能为空"),
            "error should mention empty: {err}"
        );
    }

    #[test]
    fn validate_url_missing_scheme() {
        let err = validate_auth_server_url("example.com").unwrap_err();
        assert!(
            err.contains("http:// 或 https://"),
            "error should mention http/https: {err}"
        );
    }

    #[test]
    fn validate_url_ftp_rejected() {
        let err = validate_auth_server_url("ftp://example.com").unwrap_err();
        assert!(
            err.contains("http:// 或 https://"),
            "error should mention http/https: {err}"
        );
    }

    #[test]
    fn validate_url_trims_and_removes_trailing_slash() {
        let result = validate_auth_server_url("  https://example.com/  ").unwrap();
        assert_eq!(result, "https://example.com");
    }

    #[test]
    fn validate_url_http_allowed() {
        let result = validate_auth_server_url("http://localhost:8080").unwrap();
        assert_eq!(result, "http://localhost:8080");
    }

    #[test]
    fn validate_url_no_trailing_slash_kept() {
        let result = validate_auth_server_url("https://auth.example.com").unwrap();
        assert_eq!(result, "https://auth.example.com");
    }



    #[test]
    fn token_save_and_load_roundtrip() {
        let dir = setup_temp_dir();
        let data_dir = dir.path();

        save_account_token(data_dir, "acc-1", "token-abc").unwrap();
        let loaded = load_account_token(data_dir, "acc-1").unwrap();
        assert_eq!(loaded, Some("token-abc".to_string()));
    }

    #[test]
    fn token_load_missing_returns_none() {
        let dir = setup_temp_dir();
        let data_dir = dir.path();

        let loaded = load_account_token(data_dir, "nonexistent").unwrap();
        assert!(loaded.is_none());
    }

    #[test]
    fn token_delete_removes_entry() {
        let dir = setup_temp_dir();
        let data_dir = dir.path();

        save_account_token(data_dir, "acc-x", "secret-token").unwrap();
        delete_account_token(data_dir, "acc-x").unwrap();

        let loaded = load_account_token(data_dir, "acc-x").unwrap();
        assert!(loaded.is_none());
    }

    #[test]
    fn token_save_and_load_multiple_accounts() {
        let dir = setup_temp_dir();
        let data_dir = dir.path();

        save_account_token(data_dir, "a", "ta").unwrap();
        save_account_token(data_dir, "b", "tb").unwrap();

        assert_eq!(
            load_account_token(data_dir, "a").unwrap(),
            Some("ta".to_string())
        );
        assert_eq!(
            load_account_token(data_dir, "b").unwrap(),
            Some("tb".to_string())
        );
    }

    #[test]
    fn token_file_path_is_correct() {
        let path = account_token_file_path(Path::new("/data"));
        assert_eq!(path, PathBuf::from("/data/accounts/account_tokens.json"));
    }



    #[test]
    fn accounts_json_no_password_or_token() {
        let dir = setup_temp_dir();
        let data_dir = dir.path();


        let account = LauncherAccount {
            id: "tp-1".to_string(),
            kind: LauncherAccountKind::ThirdParty,
            username: "Player".to_string(),
            uuid: "uuid-1".to_string(),
            selected: true,
            auth_server_url: Some("https://auth.example.com".to_string()),
            avatar_url: None,
            created_at: utils::now_iso8601(),
            updated_at: utils::now_iso8601(),
            last_validated_at: Some(utils::now_iso8601()),
            token_expires_at: None,
        };

        save_accounts(data_dir, &[account]).unwrap();


        let path = accounts_file_path(data_dir);
        let raw = std::fs::read_to_string(&path).unwrap();

        assert!(
            !raw.contains("password"),
            "accounts.json should not contain 'password'"
        );
        assert!(
            !raw.contains("accessToken"),
            "accounts.json should not contain 'accessToken'"
        );
        assert!(
            !raw.contains("access_token"),
            "accounts.json should not contain 'access_token'"
        );
    }



    #[test]
    fn delete_account_also_removes_token() {
        let dir = setup_temp_dir();
        let data_dir = dir.path();


        let account = LauncherAccount {
            id: "del-tp".to_string(),
            kind: LauncherAccountKind::ThirdParty,
            username: "DelPlayer".to_string(),
            uuid: "del-uuid".to_string(),
            selected: true,
            auth_server_url: Some("https://auth.example.com".to_string()),
            avatar_url: None,
            created_at: utils::now_iso8601(),
            updated_at: utils::now_iso8601(),
            last_validated_at: Some(utils::now_iso8601()),
            token_expires_at: None,
        };
        save_accounts(data_dir, std::slice::from_ref(&account)).unwrap();
        save_account_token(data_dir, &account.id, "some-token").unwrap();


        assert!(load_account_token(data_dir, &account.id).unwrap().is_some());


        delete_account_from(data_dir, &account.id).unwrap();


        assert!(
            load_account_token(data_dir, &account.id).unwrap().is_none(),
            "token should be deleted with account"
        );
    }



    #[test]
    fn token_for_offline_is_zero() {
        let dir = setup_temp_dir();
        let data_dir = dir.path();
        let account = LauncherAccount {
            id: "off-1".to_string(),
            kind: LauncherAccountKind::Offline,
            username: "Steve".to_string(),
            uuid: generate_offline_uuid("Steve"),
            selected: true,
            auth_server_url: None,
            avatar_url: None,
            created_at: utils::now_iso8601(),
            updated_at: utils::now_iso8601(),
            last_validated_at: None,
            token_expires_at: None,
        };
        let token = account_token_for_local_launch(data_dir, &account).unwrap();
        assert_eq!(token, "0");
    }

    #[test]
    fn token_for_microsoft_requires_saved_token() {
        let dir = setup_temp_dir();
        let data_dir = dir.path();
        let account = LauncherAccount {
            id: "ms-1".to_string(),
            kind: LauncherAccountKind::Microsoft,
            username: "Steve".to_string(),
            uuid: generate_offline_uuid("Steve"),
            selected: true,
            auth_server_url: None,
            avatar_url: None,
            created_at: utils::now_iso8601(),
            updated_at: utils::now_iso8601(),
            last_validated_at: None,
            token_expires_at: None,
        };

        let err = account_token_for_local_launch(data_dir, &account).unwrap_err();
        assert!(
            err.contains("已失效") || err.contains("重新登录"),
            "missing MS token should error: {err}"
        );


        save_account_token(data_dir, &account.id, "mc-ms-token").unwrap();
        let token = account_token_for_local_launch(data_dir, &account).unwrap();
        assert_eq!(token, "mc-ms-token");
    }

    #[test]
    fn token_for_third_party_returns_saved_token() {
        let dir = setup_temp_dir();
        let data_dir = dir.path();
        let account = LauncherAccount {
            id: "tp-tok".to_string(),
            kind: LauncherAccountKind::ThirdParty,
            username: "TPPlayer".to_string(),
            uuid: "tp-uuid".to_string(),
            selected: true,
            auth_server_url: Some("https://auth.example.com".to_string()),
            avatar_url: None,
            created_at: utils::now_iso8601(),
            updated_at: utils::now_iso8601(),
            last_validated_at: Some(utils::now_iso8601()),
            token_expires_at: None,
        };

        save_account_token(data_dir, &account.id, "my-access-token").unwrap();
        let token = account_token_for_local_launch(data_dir, &account).unwrap();
        assert_eq!(token, "my-access-token");
    }

    #[test]
    fn token_for_third_party_missing_token_errors() {
        let dir = setup_temp_dir();
        let data_dir = dir.path();
        let account = LauncherAccount {
            id: "tp-notok".to_string(),
            kind: LauncherAccountKind::ThirdParty,
            username: "NoToken".to_string(),
            uuid: "no-tok-uuid".to_string(),
            selected: true,
            auth_server_url: Some("https://auth.example.com".to_string()),
            avatar_url: None,
            created_at: utils::now_iso8601(),
            updated_at: utils::now_iso8601(),
            last_validated_at: Some(utils::now_iso8601()),
            token_expires_at: None,
        };

        let err = account_token_for_local_launch(data_dir, &account).unwrap_err();
        assert!(
            err.contains("已失效") || err.contains("重新登录"),
            "error should mention expired login: {err}"
        );
    }



    #[test]
    fn historical_json_without_new_fields_deserializes() {

        let old_json = r#"[
            {
                "id": "old-1",
                "kind": "Offline",
                "username": "OldPlayer",
                "uuid": "old-uuid",
                "selected": true,
                "auth_server_url": null,
                "avatar_url": null,
                "created_at": "2024-01-01T00:00:00Z",
                "updated_at": "2024-01-01T00:00:00Z"
            }
        ]"#;

        let accounts: Vec<LauncherAccount> =
            serde_json::from_str(old_json).expect("old JSON should deserialize");
        assert_eq!(accounts.len(), 1);
        assert_eq!(accounts[0].username, "OldPlayer");
        assert!(accounts[0].last_validated_at.is_none());
        assert!(accounts[0].token_expires_at.is_none());
    }

    #[test]
    fn historical_json_mixed_fields_deserializes() {
        let mixed_json = r#"[
            {
                "id": "mix-1",
                "kind": "ThirdParty",
                "username": "MixPlayer",
                "uuid": "mix-uuid",
                "selected": true,
                "auth_server_url": "https://auth.example.com",
                "avatar_url": null,
                "created_at": "2024-01-01T00:00:00Z",
                "updated_at": "2024-01-01T00:00:00Z",
                "last_validated_at": "2025-01-01T00:00:00Z"
            },
            {
                "id": "mix-2",
                "kind": "Offline",
                "username": "MixOffline",
                "uuid": "mix-off-uuid",
                "selected": false,
                "auth_server_url": null,
                "avatar_url": null,
                "created_at": "2024-01-01T00:00:00Z",
                "updated_at": "2024-01-01T00:00:00Z"
            }
        ]"#;

        let accounts: Vec<LauncherAccount> =
            serde_json::from_str(mixed_json).expect("mixed JSON should deserialize");
        assert_eq!(accounts.len(), 2);
        assert_eq!(
            accounts[0].last_validated_at.as_deref(),
            Some("2025-01-01T00:00:00Z")
        );
        assert!(accounts[0].token_expires_at.is_none());
        assert!(accounts[1].last_validated_at.is_none());
        assert!(accounts[1].token_expires_at.is_none());
    }






    async fn start_mock_yggdrasil_server(
        status: u16,
        body: &str,
    ) -> (String, tokio::task::JoinHandle<()>) {
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        let addr = listener.local_addr().unwrap();
        let url = format!("http://127.0.0.1:{}", addr.port());
        let body = body.to_string();
        let handle = tokio::spawn(async move {
            let (mut socket, _) = listener.accept().await.unwrap();
            use tokio::io::AsyncReadExt;
            let mut buf = [0u8; 4096];
            let _ = tokio::time::timeout(std::time::Duration::from_secs(5), socket.read(&mut buf))
                .await;
            let response = format!(
                "HTTP/1.1 {status} OK\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{body}",
                body.len()
            );
            use tokio::io::AsyncWriteExt;
            let _ = socket.write_all(response.as_bytes()).await;
        });
        (url, handle)
    }

    #[tokio::test]
    async fn authenticate_parses_success_response() {
        let response = serde_json::json!({
            "accessToken": "test-access-token-123",
            "selectedProfile": {
                "id": "profile-uuid-123",
                "name": "TestPlayer"
            }
        });
        let (url, _handle) =
            start_mock_yggdrasil_server(200, &serde_json::to_string(&response).unwrap()).await;

        let (token, profile_id, profile_name) =
            authenticate_yggdrasil(&url, "user@example.com", "secret123")
                .await
                .unwrap();

        assert_eq!(token, "test-access-token-123");
        assert_eq!(profile_id, "profile-uuid-123");
        assert_eq!(profile_name, "TestPlayer");
    }

    #[tokio::test]
    async fn authenticate_error_response_returns_err() {
        let error_body = serde_json::json!({
            "error": "ForbiddenOperationException",
            "errorMessage": "Invalid credentials. Invalid username or password."
        });
        let (url, _handle) =
            start_mock_yggdrasil_server(403, &serde_json::to_string(&error_body).unwrap()).await;

        let result = authenticate_yggdrasil(&url, "bad", "wrong").await;
        assert!(result.is_err());
        let err = result.unwrap_err();
        assert!(
            err.contains("Invalid credentials") || err.contains("403"),
            "error should contain credential message: {err}"
        );
    }

    #[tokio::test]
    async fn authenticate_missing_fields_returns_err() {

        let response = serde_json::json!({
            "accessToken": "token-only"
        });
        let (url, _handle) =
            start_mock_yggdrasil_server(200, &serde_json::to_string(&response).unwrap()).await;

        let result = authenticate_yggdrasil(&url, "user", "pass").await;
        assert!(result.is_err());
        assert!(
            result.unwrap_err().contains("格式错误"),
            "missing selectedProfile should cause parse error"
        );
    }

    #[tokio::test]
    async fn authenticate_connection_refused_returns_err() {

        let result = authenticate_yggdrasil("http://127.0.0.1:1", "user", "pass").await;
        assert!(result.is_err());
    }



    #[test]
    fn profile_id_32hex_to_uuid_valid() {
        let result = profile_id_to_uuid("069a79f444e94726a5befca90e38aaf5").unwrap();
        assert_eq!(result, "069a79f4-44e9-4726-a5be-fca90e38aaf5");
    }

    #[test]
    fn profile_id_32hex_to_uuid_uppercase() {
        let result = profile_id_to_uuid("069A79F444E94726A5BEFCA90E38AAF5").unwrap();
        assert_eq!(result, "069a79f4-44e9-4726-a5be-fca90e38aaf5");
    }

    #[test]
    fn profile_id_too_short_errors() {
        let err = profile_id_to_uuid("abc").unwrap_err();
        assert!(err.contains("长度"), "error should mention length: {err}");
    }

    #[test]
    fn profile_id_too_long_errors() {
        let err = profile_id_to_uuid(&"a".repeat(33)).unwrap_err();
        assert!(err.contains("长度"), "error should mention length: {err}");
    }

    #[test]
    fn profile_id_non_hex_errors() {
        let err = profile_id_to_uuid("069a79f444e94726a5befca90e38aafg").unwrap_err();
        assert!(
            err.contains("十六进制"),
            "error should mention non-hex: {err}"
        );
    }

    #[test]
    fn profile_id_empty_errors() {
        let err = profile_id_to_uuid("").unwrap_err();
        assert!(err.contains("长度"), "error should mention length: {err}");
    }

    #[test]
    fn profile_id_with_whitespace_trims() {
        let result = profile_id_to_uuid("  069a79f444e94726a5befca90e38aaf5  ").unwrap();
        assert_eq!(result, "069a79f4-44e9-4726-a5be-fca90e38aaf5");
    }



    #[test]
    fn oauth_error_authorization_pending_is_chinese() {
        let msg = map_microsoft_oauth_error("authorization_pending");
        assert!(
            msg.contains("尚未完成授权"),
            "should be pending message: {msg}"
        );
    }

    #[test]
    fn oauth_error_slow_down_is_chinese() {
        let msg = map_microsoft_oauth_error("slow_down");
        assert!(
            msg.contains("轮询过于频繁"),
            "should be slow_down message: {msg}"
        );
    }

    #[test]
    fn oauth_error_expired_token_is_chinese() {
        let msg = map_microsoft_oauth_error("expired_token");
        assert!(msg.contains("已过期"), "should be expired message: {msg}");
    }

    #[test]
    fn oauth_error_authorization_declined_is_chinese() {
        let msg = map_microsoft_oauth_error("authorization_declined");
        assert!(msg.contains("拒绝"), "should be declined message: {msg}");
    }

    #[test]
    fn oauth_error_unknown_code_contains_code() {
        let msg = map_microsoft_oauth_error("unknown_error_xyz");
        assert!(
            msg.contains("unknown_error_xyz"),
            "unknown code should appear in message: {msg}"
        );
    }

    #[test]
    fn oauth_error_bad_verification_code_is_chinese() {
        let msg = map_microsoft_oauth_error("bad_verification_code");
        assert!(
            msg.contains("验证码无效"),
            "should be bad code message: {msg}"
        );
    }

    #[test]
    fn oauth_error_invalid_grant_is_chinese() {
        let msg = map_microsoft_oauth_error("invalid_grant");
        assert!(
            msg.contains("已失效"),
            "should be invalid grant message: {msg}"
        );
    }



    #[test]
    fn extract_uhs_from_xbox_response_valid() {
        let json = serde_json::json!({
            "DisplayClaims": {
                "xui": [{"uhs": "1234567890abcdef"}]
            }
        });
        let uhs = extract_uhs_from_xbox_response(&json).unwrap();
        assert_eq!(uhs, "1234567890abcdef");
    }

    #[test]
    fn extract_uhs_from_xbox_missing_xui_errors() {
        let json = serde_json::json!({
            "DisplayClaims": {"xui": []}
        });
        let err = extract_uhs_from_xbox_response(&json).unwrap_err();
        assert!(err.contains("uhs"), "error should mention uhs: {err}");
    }

    #[test]
    fn extract_uhs_from_xbox_missing_display_claims_errors() {
        let json = serde_json::json!({"Token": "abc"});
        let err = extract_uhs_from_xbox_response(&json).unwrap_err();
        assert!(err.contains("uhs"), "error should mention uhs: {err}");
    }

    #[test]
    fn extract_uhs_from_xsts_response_valid() {
        let json = serde_json::json!({
            "DisplayClaims": {
                "xui": [{"uhs": "fedcba0987654321"}]
            }
        });
        let uhs = extract_uhs_from_xsts_response(&json).unwrap();
        assert_eq!(uhs, "fedcba0987654321");
    }



    #[test]
    fn parse_microsoft_device_code_response_full() {
        let json = r#"{
            "device_code": "dc-123",
            "user_code": "ABC123",
            "verification_uri": "https://microsoft.com/link",
            "verification_uri_complete": "https://microsoft.com/link?otc=ABC123",
            "expires_in": 900,
            "interval": 5,
            "message": "To sign in..."
        }"#;
        let parsed: MicrosoftDeviceCodeResponse = serde_json::from_str(json).expect("should parse");
        assert_eq!(parsed.device_code, "dc-123");
        assert_eq!(parsed.user_code, "ABC123");
        assert_eq!(parsed.verification_uri, "https://microsoft.com/link");
        assert_eq!(
            parsed.verification_uri_complete,
            Some("https://microsoft.com/link?otc=ABC123".to_string())
        );
        assert_eq!(parsed.expires_in, 900);
        assert_eq!(parsed.interval, 5);
        assert_eq!(parsed.message, Some("To sign in...".to_string()));
    }

    #[test]
    fn parse_microsoft_device_code_response_minimal() {
        let json = r#"{
            "device_code": "dc-min",
            "user_code": "XYZ",
            "verification_uri": "https://example.com",
            "expires_in": 300,
            "interval": 5
        }"#;
        let parsed: MicrosoftDeviceCodeResponse = serde_json::from_str(json).expect("should parse");
        assert_eq!(parsed.device_code, "dc-min");
        assert!(parsed.verification_uri_complete.is_none());
        assert!(parsed.message.is_none());
    }

    #[test]
    fn parse_microsoft_token_response() {
        let json = r#"{
            "token_type": "Bearer",
            "scope": "XboxLive.signin",
            "access_token": "ms-access-token-abc",
            "expires_in": 3600
        }"#;
        let parsed: MicrosoftTokenResponse = serde_json::from_str(json).expect("should parse");
        assert_eq!(parsed.access_token, "ms-access-token-abc");
    }

    #[test]
    fn parse_microsoft_token_error() {
        let json = r#"{
            "error": "authorization_pending",
            "error_description": "The user has not yet completed authorization"
        }"#;
        let parsed: MicrosoftTokenError = serde_json::from_str(json).expect("should parse");
        assert_eq!(parsed.error, "authorization_pending");
        assert_eq!(
            parsed.error_description,
            Some("The user has not yet completed authorization".to_string())
        );
    }



    #[test]
    fn parse_xbox_auth_response() {
        let json = r#"{
            "IssueInstant": "2024-01-01T00:00:00Z",
            "NotAfter": "2024-01-02T00:00:00Z",
            "Token": "xbl-token",
            "DisplayClaims": {
                "xui": [{"uhs": "test-uhs-123"}]
            }
        }"#;
        let parsed: XboxAuthResponse = serde_json::from_str(json).expect("should parse");
        assert_eq!(parsed.token, "xbl-token");
        assert_eq!(parsed.display_claims.xui[0].uhs, "test-uhs-123");
    }



    #[test]
    fn parse_minecraft_auth_response() {
        let json = r#"{
            "username": "some-uuid",
            "access_token": "mc-access-token",
            "token_type": "Bearer",
            "expires_in": 86400
        }"#;
        let parsed: MinecraftAuthResponse = serde_json::from_str(json).expect("should parse");
        assert_eq!(parsed.access_token, "mc-access-token");
        assert_eq!(parsed.expires_in, Some(86400));
    }

    #[test]
    fn parse_minecraft_auth_minimal() {
        let json = r#"{
            "access_token": "mc-min-token"
        }"#;
        let parsed: MinecraftAuthResponse = serde_json::from_str(json).expect("should parse");
        assert_eq!(parsed.access_token, "mc-min-token");
        assert!(parsed.expires_in.is_none());
    }

    #[test]
    fn parse_minecraft_profile_response() {
        let json = r#"{
            "id": "069a79f444e94726a5befca90e38aaf5",
            "name": "TestPlayer"
        }"#;
        let parsed: MinecraftProfileResponse = serde_json::from_str(json).expect("should parse");
        assert_eq!(parsed.id, "069a79f444e94726a5befca90e38aaf5");
        assert_eq!(parsed.name, "TestPlayer");
    }

    #[test]
    fn parse_minecraft_profile_error() {
        let json = r#"{
            "error": "NOT_FOUND",
            "path": "/minecraft/profile"
        }"#;
        let parsed: MinecraftProfileError = serde_json::from_str(json).expect("should parse");
        assert_eq!(parsed.error, "NOT_FOUND");
    }



    #[test]
    fn microsoft_token_save_and_launch_token() {
        let dir = setup_temp_dir();
        let data_dir = dir.path();

        let account = LauncherAccount {
            id: "ms-tok-test".to_string(),
            kind: LauncherAccountKind::Microsoft,
            username: "MsPlayer".to_string(),
            uuid: "069a79f4-44e9-4726-a5be-fca90e38aaf5".to_string(),
            selected: true,
            auth_server_url: None,
            avatar_url: None,
            created_at: utils::now_iso8601(),
            updated_at: utils::now_iso8601(),
            last_validated_at: Some(utils::now_iso8601()),
            token_expires_at: Some(utils::now_iso8601()),
        };


        let err = account_token_for_local_launch(data_dir, &account).unwrap_err();
        assert!(
            err.contains("已失效") || err.contains("重新登录"),
            "missing token should error: {err}"
        );


        save_account_token(data_dir, &account.id, "mc-access-token-ms").unwrap();


        let token = account_token_for_local_launch(data_dir, &account).unwrap();
        assert_eq!(token, "mc-access-token-ms");
    }

    #[test]
    fn microsoft_token_missing_for_launch_errors() {
        let dir = setup_temp_dir();
        let data_dir = dir.path();

        let account = LauncherAccount {
            id: "ms-no-tok".to_string(),
            kind: LauncherAccountKind::Microsoft,
            username: "MsNoToken".to_string(),
            uuid: "aaaaaaaa-bbbb-cccc-dddd-eeeeeeeeeeee".to_string(),
            selected: true,
            auth_server_url: None,
            avatar_url: None,
            created_at: utils::now_iso8601(),
            updated_at: utils::now_iso8601(),
            last_validated_at: Some(utils::now_iso8601()),
            token_expires_at: None,
        };

        let err = account_token_for_local_launch(data_dir, &account).unwrap_err();
        assert!(
            err.contains("Microsoft"),
            "error should mention Microsoft: {err}"
        );
        assert!(
            err.contains("已失效") || err.contains("重新登录"),
            "error should mention expired: {err}"
        );
    }



    #[test]
    fn historical_microsoft_account_json_deserializes() {


        let old_json = r#"[
            {
                "id": "ms-old-1",
                "kind": "Microsoft",
                "username": "OldMsPlayer",
                "uuid": "069a79f4-44e9-4726-a5be-fca90e38aaf5",
                "selected": true,
                "auth_server_url": null,
                "avatar_url": null,
                "created_at": "2024-01-01T00:00:00Z",
                "updated_at": "2024-01-01T00:00:00Z"
            }
        ]"#;

        let accounts: Vec<LauncherAccount> =
            serde_json::from_str(old_json).expect("old MS JSON should deserialize");
        assert_eq!(accounts.len(), 1);
        assert_eq!(accounts[0].kind, LauncherAccountKind::Microsoft);
        assert_eq!(accounts[0].username, "OldMsPlayer");
        assert!(accounts[0].last_validated_at.is_none());
        assert!(accounts[0].token_expires_at.is_none());
    }

    #[test]
    fn historical_microsoft_with_new_fields_deserializes() {
        let json = r#"[
            {
                "id": "ms-new-1",
                "kind": "Microsoft",
                "username": "NewMsPlayer",
                "uuid": "069a79f4-44e9-4726-a5be-fca90e38aaf5",
                "selected": true,
                "auth_server_url": null,
                "avatar_url": null,
                "created_at": "2024-01-01T00:00:00Z",
                "updated_at": "2024-01-01T00:00:00Z",
                "last_validated_at": "2025-05-01T00:00:00Z",
                "token_expires_at": "2025-05-02T00:00:00Z"
            }
        ]"#;

        let accounts: Vec<LauncherAccount> =
            serde_json::from_str(json).expect("should deserialize");
        assert_eq!(accounts.len(), 1);
        assert_eq!(accounts[0].kind, LauncherAccountKind::Microsoft);
        assert_eq!(
            accounts[0].last_validated_at.as_deref(),
            Some("2025-05-01T00:00:00Z")
        );
        assert_eq!(
            accounts[0].token_expires_at.as_deref(),
            Some("2025-05-02T00:00:00Z")
        );
    }



    #[test]
    fn microsoft_device_auth_result_serialize_deserialize() {
        let result = MicrosoftDeviceAuthStartResult {
            device_code: "dc-1".to_string(),
            user_code: "USER123".to_string(),
            verification_uri: "https://microsoft.com/link".to_string(),
            verification_uri_complete: Some("https://microsoft.com/link?otc=USER123".to_string()),
            expires_in: 900,
            interval: 5,
            message: Some("Sign in".to_string()),
        };

        let json = serde_json::to_string(&result).unwrap();
        let roundtripped: MicrosoftDeviceAuthStartResult = serde_json::from_str(&json).unwrap();
        assert_eq!(roundtripped.device_code, "dc-1");
        assert_eq!(roundtripped.user_code, "USER123");
        assert_eq!(roundtripped.expires_in, 900);
        assert_eq!(roundtripped.interval, 5);
    }

    #[test]
    fn microsoft_login_result_serialize_deserialize() {
        let account = LauncherAccount {
            id: "ms-res-1".to_string(),
            kind: LauncherAccountKind::Microsoft,
            username: "TestPlayer".to_string(),
            uuid: "069a79f4-44e9-4726-a5be-fca90e38aaf5".to_string(),
            selected: true,
            auth_server_url: None,
            avatar_url: None,
            created_at: "2024-01-01T00:00:00Z".to_string(),
            updated_at: "2024-01-01T00:00:00Z".to_string(),
            last_validated_at: Some("2024-01-01T00:00:00Z".to_string()),
            token_expires_at: Some("2024-01-02T00:00:00Z".to_string()),
        };

        let result = MicrosoftLoginResult {
            account: account.clone(),
            token_saved: true,
            refresh_token_saved: true,
        };

        let json = serde_json::to_string(&result).unwrap();
        let roundtripped: MicrosoftLoginResult = serde_json::from_str(&json).unwrap();
        assert!(roundtripped.token_saved);
        assert!(roundtripped.refresh_token_saved);
        assert_eq!(roundtripped.account.username, "TestPlayer");
        assert_eq!(roundtripped.account.kind, LauncherAccountKind::Microsoft);
    }

    #[test]
    fn microsoft_device_auth_result_minimal_no_optional_fields() {
        let json = r#"{
            "device_code": "dc-min",
            "user_code": "UC",
            "verification_uri": "https://x.com",
            "expires_in": 300,
            "interval": 5
        }"#;
        let parsed: MicrosoftDeviceAuthStartResult =
            serde_json::from_str(json).expect("should parse without optional fields");
        assert_eq!(parsed.device_code, "dc-min");
        assert!(parsed.verification_uri_complete.is_none());
        assert!(parsed.message.is_none());
    }

    #[test]
    fn microsoft_login_result_minimal_no_refresh_token_saved() {

        let json = r#"{
            "account": {
                "id": "ms-min-1",
                "kind": "Microsoft",
                "username": "MinPlayer",
                "uuid": "069a79f4-44e9-4726-a5be-fca90e38aaf5",
                "selected": true,
                "auth_server_url": null,
                "avatar_url": null,
                "created_at": "2024-01-01T00:00:00Z",
                "updated_at": "2024-01-01T00:00:00Z"
            },
            "token_saved": true
        }"#;
        let parsed: MicrosoftLoginResult =
            serde_json::from_str(json).expect("should parse without refresh_token_saved");
        assert!(parsed.token_saved);
        assert!(
            !parsed.refresh_token_saved,
            "refresh_token_saved should default to false"
        );
    }



    #[test]
    fn microsoft_refresh_result_serialize_deserialize() {
        let account = LauncherAccount {
            id: "ms-refresh-1".to_string(),
            kind: LauncherAccountKind::Microsoft,
            username: "RefreshPlayer".to_string(),
            uuid: "069a79f4-44e9-4726-a5be-fca90e38aaf5".to_string(),
            selected: true,
            auth_server_url: None,
            avatar_url: None,
            created_at: "2024-01-01T00:00:00Z".to_string(),
            updated_at: "2024-01-01T00:00:00Z".to_string(),
            last_validated_at: Some("2024-01-01T00:00:00Z".to_string()),
            token_expires_at: Some("2024-01-02T00:00:00Z".to_string()),
        };

        let result = MicrosoftRefreshResult {
            account: account.clone(),
            token_saved: true,
            refresh_token_saved: false,
        };

        let json = serde_json::to_string(&result).unwrap();
        let roundtripped: MicrosoftRefreshResult = serde_json::from_str(&json).unwrap();
        assert!(roundtripped.token_saved);
        assert!(!roundtripped.refresh_token_saved);
        assert_eq!(roundtripped.account.username, "RefreshPlayer");
        assert_eq!(roundtripped.account.kind, LauncherAccountKind::Microsoft);
    }



    #[test]
    fn microsoft_refresh_token_file_path_is_correct() {
        let path = microsoft_refresh_token_file_path(Path::new("/data"));
        assert_eq!(
            path,
            PathBuf::from("/data/accounts/microsoft_refresh_tokens.json")
        );
    }



    #[test]
    fn microsoft_refresh_token_save_and_load_roundtrip() {
        let dir = setup_temp_dir();
        let data_dir = dir.path();

        save_microsoft_refresh_token(data_dir, "ms-acc-1", "rt-abc").unwrap();
        let loaded = load_microsoft_refresh_token(data_dir, "ms-acc-1").unwrap();
        assert_eq!(loaded, Some("rt-abc".to_string()));
    }

    #[test]
    fn microsoft_refresh_token_load_missing_returns_none() {
        let dir = setup_temp_dir();
        let data_dir = dir.path();

        let loaded = load_microsoft_refresh_token(data_dir, "nonexistent").unwrap();
        assert!(loaded.is_none());
    }

    #[test]
    fn microsoft_refresh_token_delete_removes_entry() {
        let dir = setup_temp_dir();
        let data_dir = dir.path();

        save_microsoft_refresh_token(data_dir, "ms-x", "rt-secret").unwrap();
        delete_microsoft_refresh_token(data_dir, "ms-x").unwrap();

        let loaded = load_microsoft_refresh_token(data_dir, "ms-x").unwrap();
        assert!(loaded.is_none());
    }

    #[test]
    fn microsoft_refresh_token_save_and_load_multiple_accounts() {
        let dir = setup_temp_dir();
        let data_dir = dir.path();

        save_microsoft_refresh_token(data_dir, "a", "rta").unwrap();
        save_microsoft_refresh_token(data_dir, "b", "rtb").unwrap();

        assert_eq!(
            load_microsoft_refresh_token(data_dir, "a").unwrap(),
            Some("rta".to_string())
        );
        assert_eq!(
            load_microsoft_refresh_token(data_dir, "b").unwrap(),
            Some("rtb".to_string())
        );
    }

    #[test]
    fn microsoft_refresh_token_empty_file_returns_none() {
        let dir = setup_temp_dir();
        let data_dir = dir.path();


        let path = microsoft_refresh_token_file_path(data_dir);
        std::fs::create_dir_all(path.parent().unwrap()).unwrap();
        std::fs::write(&path, "").unwrap();

        let loaded = load_microsoft_refresh_token(data_dir, "any").unwrap();
        assert!(loaded.is_none());
    }



    #[test]
    fn parse_microsoft_token_response_with_refresh_token() {
        let json = r#"{
            "token_type": "Bearer",
            "scope": "XboxLive.signin offline_access",
            "access_token": "ms-access-token-abc",
            "refresh_token": "ms-refresh-token-xyz",
            "expires_in": 3600
        }"#;
        let parsed: MicrosoftTokenResponse = serde_json::from_str(json).expect("should parse");
        assert_eq!(parsed.access_token, "ms-access-token-abc");
        assert_eq!(
            parsed.refresh_token,
            Some("ms-refresh-token-xyz".to_string())
        );
    }

    #[test]
    fn parse_microsoft_token_response_without_refresh_token() {

        let json = r#"{
            "token_type": "Bearer",
            "scope": "XboxLive.signin",
            "access_token": "ms-access-token-abc",
            "expires_in": 3600
        }"#;
        let parsed: MicrosoftTokenResponse = serde_json::from_str(json).expect("should parse");
        assert_eq!(parsed.access_token, "ms-access-token-abc");
        assert!(parsed.refresh_token.is_none());
    }



    #[test]
    fn accounts_json_no_refresh_token() {
        let dir = setup_temp_dir();
        let data_dir = dir.path();

        let account = LauncherAccount {
            id: "ms-no-rt".to_string(),
            kind: LauncherAccountKind::Microsoft,
            username: "NoRtPlayer".to_string(),
            uuid: "069a79f4-44e9-4726-a5be-fca90e38aaf5".to_string(),
            selected: true,
            auth_server_url: None,
            avatar_url: None,
            created_at: utils::now_iso8601(),
            updated_at: utils::now_iso8601(),
            last_validated_at: Some(utils::now_iso8601()),
            token_expires_at: Some(utils::now_iso8601()),
        };

        save_accounts(data_dir, &[account]).unwrap();

        let path = accounts_file_path(data_dir);
        let raw = std::fs::read_to_string(&path).unwrap();

        assert!(
            !raw.to_lowercase().contains("refresh_token"),
            "accounts.json should not contain refresh_token"
        );
        assert!(
            !raw.contains("access_token"),
            "accounts.json should not contain access_token"
        );
        assert!(
            !raw.contains("password"),
            "accounts.json should not contain password"
        );
    }



    #[test]
    fn delete_microsoft_account_also_removes_refresh_token() {
        let dir = setup_temp_dir();
        let data_dir = dir.path();


        let account = LauncherAccount {
            id: "del-ms-refresh".to_string(),
            kind: LauncherAccountKind::Microsoft,
            username: "DelMsRefresh".to_string(),
            uuid: "069a79f4-44e9-4726-a5be-fca90e38aaf5".to_string(),
            selected: true,
            auth_server_url: None,
            avatar_url: None,
            created_at: utils::now_iso8601(),
            updated_at: utils::now_iso8601(),
            last_validated_at: Some(utils::now_iso8601()),
            token_expires_at: None,
        };
        save_accounts(data_dir, std::slice::from_ref(&account)).unwrap();
        save_microsoft_refresh_token(data_dir, &account.id, "rt-for-delete").unwrap();


        assert!(load_microsoft_refresh_token(data_dir, &account.id)
            .unwrap()
            .is_some());


        delete_account_from(data_dir, &account.id).unwrap();


        assert!(
            load_microsoft_refresh_token(data_dir, &account.id)
                .unwrap()
                .is_none(),
            "Microsoft refresh token should be deleted with account"
        );
    }

    #[test]
    fn delete_non_microsoft_account_does_not_touch_refresh_token() {
        let dir = setup_temp_dir();
        let data_dir = dir.path();


        let account = LauncherAccount {
            id: "off-no-rt".to_string(),
            kind: LauncherAccountKind::Offline,
            username: "OffPlayer".to_string(),
            uuid: generate_offline_uuid("OffPlayer"),
            selected: true,
            auth_server_url: None,
            avatar_url: None,
            created_at: utils::now_iso8601(),
            updated_at: utils::now_iso8601(),
            last_validated_at: None,
            token_expires_at: None,
        };
        save_accounts(data_dir, std::slice::from_ref(&account)).unwrap();


        delete_account_from(data_dir, &account.id).unwrap();

        let remaining = load_accounts(data_dir).unwrap();
        assert!(remaining.is_empty());
    }



    #[test]
    fn ensure_profile_matches_account_match_ok() {
        let account = LauncherAccount {
            id: "test-1".to_string(),
            kind: LauncherAccountKind::Microsoft,
            username: "TestPlayer".to_string(),
            uuid: "aaaaaaaa-bbbb-cccc-dddd-eeeeeeeeeeee".to_string(),
            selected: true,
            auth_server_url: None,
            avatar_url: None,
            created_at: "2024-01-01T00:00:00Z".to_string(),
            updated_at: "2024-01-01T00:00:00Z".to_string(),
            last_validated_at: None,
            token_expires_at: None,
        };

        ensure_profile_matches_account("aaaaaaaa-bbbb-cccc-dddd-eeeeeeeeeeee", &account)
            .expect("matching UUID should succeed");
    }

    #[test]
    fn ensure_profile_matches_account_mismatch_errors() {
        let account = LauncherAccount {
            id: "test-2".to_string(),
            kind: LauncherAccountKind::Microsoft,
            username: "TestPlayer".to_string(),
            uuid: "aaaaaaaa-bbbb-cccc-dddd-eeeeeeeeeeee".to_string(),
            selected: true,
            auth_server_url: None,
            avatar_url: None,
            created_at: "2024-01-01T00:00:00Z".to_string(),
            updated_at: "2024-01-01T00:00:00Z".to_string(),
            last_validated_at: None,
            token_expires_at: None,
        };

        let err = ensure_profile_matches_account("ffffffff-ffff-ffff-ffff-ffffffffffff", &account)
            .unwrap_err();
        assert!(
            err.contains("不匹配"),
            "error should mention UUID mismatch: {err}"
        );
        assert!(
            err.contains("aaaaaaaa"),
            "error should contain original UUID"
        );
        assert!(
            err.contains("ffffffff"),
            "error should contain profile UUID"
        );
    }



    #[test]
    fn default_avatar_url_uses_uuid() {
        let account = LauncherAccount {
            id: "av-1".to_string(),
            kind: LauncherAccountKind::Microsoft,
            username: "AvatarPlayer".to_string(),
            uuid: "069a79f4-44e9-4726-a5be-fca90e38aaf5".to_string(),
            selected: true,
            auth_server_url: None,
            avatar_url: None,
            created_at: utils::now_iso8601(),
            updated_at: utils::now_iso8601(),
            last_validated_at: None,
            token_expires_at: None,
        };
        let url = default_avatar_url_for(&account);
        assert_eq!(
            url,
            "https://crafatar.com/avatars/069a79f4-44e9-4726-a5be-fca90e38aaf5?overlay"
        );
    }

    #[test]
    fn default_avatar_url_trims_uuid() {
        let account = LauncherAccount {
            id: "av-2".to_string(),
            kind: LauncherAccountKind::Offline,
            username: "TrimPlayer".to_string(),
            uuid: "  069a79f4-44e9-4726-a5be-fca90e38aaf5  ".to_string(),
            selected: false,
            auth_server_url: None,
            avatar_url: None,
            created_at: utils::now_iso8601(),
            updated_at: utils::now_iso8601(),
            last_validated_at: None,
            token_expires_at: None,
        };
        let url = default_avatar_url_for(&account);

        assert_eq!(
            url,
            "https://crafatar.com/avatars/069a79f4-44e9-4726-a5be-fca90e38aaf5?overlay"
        );
    }



    #[test]
    fn validate_avatar_url_accepts_https() {
        let result = validate_avatar_url("https://example.com/skin.png").unwrap();
        assert_eq!(result, "https://example.com/skin.png");
    }

    #[test]
    fn validate_avatar_url_accepts_http() {
        let result = validate_avatar_url("http://localhost/skin.png").unwrap();
        assert_eq!(result, "http://localhost/skin.png");
    }

    #[test]
    fn validate_avatar_url_rejects_empty() {
        let err = validate_avatar_url("").unwrap_err();
        assert!(
            err.contains("不能为空"),
            "error should mention empty: {err}"
        );
    }

    #[test]
    fn validate_avatar_url_rejects_whitespace_only() {
        let err = validate_avatar_url("   \t ").unwrap_err();
        assert!(
            err.contains("不能为空"),
            "error should mention empty for whitespace: {err}"
        );
    }

    #[test]
    fn validate_avatar_url_rejects_nul() {
        let err = validate_avatar_url("https://example.com/\0skin.png").unwrap_err();
        assert!(
            err.contains("非法字符"),
            "error should mention illegal chars for NUL: {err}"
        );
    }

    #[test]
    fn validate_avatar_url_rejects_ftp() {
        let err = validate_avatar_url("ftp://example.com/skin.png").unwrap_err();
        assert!(
            err.contains("http:// 或 https://"),
            "error should mention http/https: {err}"
        );
    }

    #[test]
    fn validate_avatar_url_rejects_too_long() {
        let long = format!("https://example.com/{}", "a".repeat(2048));
        let err = validate_avatar_url(&long).unwrap_err();
        assert!(err.contains("2048"), "error should mention length: {err}");
    }

    #[test]
    fn validate_avatar_url_trims_whitespace() {
        let result = validate_avatar_url("  https://example.com/skin.png  ").unwrap();
        assert_eq!(result, "https://example.com/skin.png");
    }



    #[test]
    fn update_account_avatar_set_url() {
        let dir = setup_temp_dir();
        let data_dir = dir.path();

        let account = add_offline_account_in(
            data_dir,
            AddOfflineAccountRequest {
                username: "Steve".to_string(),
            },
        )
        .unwrap();

        let result = update_account_avatar_in(
            data_dir,
            UpdateAccountAvatarRequest {
                account_id: account.id.clone(),
                avatar_url: Some("https://example.com/avatar.png".to_string()),
            },
        )
        .unwrap();

        assert_eq!(
            result.avatar_url,
            Some("https://example.com/avatar.png".to_string())
        );
        assert_eq!(
            result.account.avatar_url,
            Some("https://example.com/avatar.png".to_string())
        );


        let loaded = load_accounts(data_dir).unwrap();
        assert_eq!(
            loaded[0].avatar_url,
            Some("https://example.com/avatar.png".to_string())
        );
    }

    #[test]
    fn update_account_avatar_clear_url() {
        let dir = setup_temp_dir();
        let data_dir = dir.path();

        let account = add_offline_account_in(
            data_dir,
            AddOfflineAccountRequest {
                username: "Steve".to_string(),
            },
        )
        .unwrap();


        update_account_avatar_in(
            data_dir,
            UpdateAccountAvatarRequest {
                account_id: account.id.clone(),
                avatar_url: Some("https://example.com/avatar.png".to_string()),
            },
        )
        .unwrap();


        let result = update_account_avatar_in(
            data_dir,
            UpdateAccountAvatarRequest {
                account_id: account.id.clone(),
                avatar_url: None,
            },
        )
        .unwrap();

        assert_eq!(result.avatar_url, None);
        assert_eq!(result.account.avatar_url, None);

        let loaded = load_accounts(data_dir).unwrap();
        assert_eq!(loaded[0].avatar_url, None);
    }

    #[test]
    fn update_account_avatar_nonexistent_account_err() {
        let dir = setup_temp_dir();
        let data_dir = dir.path();

        let err = update_account_avatar_in(
            data_dir,
            UpdateAccountAvatarRequest {
                account_id: "nonexistent-id".to_string(),
                avatar_url: Some("https://example.com/avatar.png".to_string()),
            },
        )
        .unwrap_err();

        assert!(
            err.contains("未找到"),
            "error should mention account not found: {err}"
        );
    }

    #[test]
    fn update_account_avatar_preserves_token_metadata() {
        let dir = setup_temp_dir();
        let data_dir = dir.path();

        let account = add_offline_account_in(
            data_dir,
            AddOfflineAccountRequest {
                username: "Steve".to_string(),
            },
        )
        .unwrap();


        update_account_avatar_in(
            data_dir,
            UpdateAccountAvatarRequest {
                account_id: account.id.clone(),
                avatar_url: Some("https://example.com/avatar.png".to_string()),
            },
        )
        .unwrap();

        let loaded = load_accounts(data_dir).unwrap();
        let updated = &loaded[0];


        assert!(updated.selected, "selected should not change");

        assert!(
            updated.last_validated_at.is_none(),
            "last_validated_at should not change"
        );

        assert!(
            updated.token_expires_at.is_none(),
            "token_expires_at should not change"
        );

        assert_ne!(
            updated.updated_at, account.updated_at,
            "updated_at should change"
        );
    }



    #[test]
    fn refresh_account_avatar_default_url() {
        let dir = setup_temp_dir();
        let data_dir = dir.path();

        let account = add_offline_account_in(
            data_dir,
            AddOfflineAccountRequest {
                username: "Steve".to_string(),
            },
        )
        .unwrap();

        let result = refresh_account_avatar_in(data_dir, &account.id).unwrap();

        let expected = default_avatar_url_for(&account);
        assert_eq!(result.avatar_url, Some(expected.clone()));
        assert_eq!(result.account.avatar_url, Some(expected));

        let loaded = load_accounts(data_dir).unwrap();
        assert!(loaded[0].avatar_url.is_some());
    }

    #[test]
    fn refresh_account_avatar_nonexistent_account_err() {
        let dir = setup_temp_dir();
        let data_dir = dir.path();

        let err = refresh_account_avatar_in(data_dir, "nonexistent-id").unwrap_err();

        assert!(
            err.contains("未找到"),
            "error should mention account not found: {err}"
        );
    }

    #[test]
    fn refresh_account_avatar_overwrites_previous_url() {
        let dir = setup_temp_dir();
        let data_dir = dir.path();

        let account = add_offline_account_in(
            data_dir,
            AddOfflineAccountRequest {
                username: "Steve".to_string(),
            },
        )
        .unwrap();


        update_account_avatar_in(
            data_dir,
            UpdateAccountAvatarRequest {
                account_id: account.id.clone(),
                avatar_url: Some("https://custom.example.com/skin.png".to_string()),
            },
        )
        .unwrap();


        let result = refresh_account_avatar_in(data_dir, &account.id).unwrap();

        let expected = default_avatar_url_for(&account);
        assert_eq!(result.avatar_url, Some(expected));
    }



    #[test]
    fn accounts_json_no_token_or_password_with_avatar() {
        let dir = setup_temp_dir();
        let data_dir = dir.path();

        let account = add_offline_account_in(
            data_dir,
            AddOfflineAccountRequest {
                username: "Steve".to_string(),
            },
        )
        .unwrap();


        update_account_avatar_in(
            data_dir,
            UpdateAccountAvatarRequest {
                account_id: account.id.clone(),
                avatar_url: Some("https://example.com/avatar.png".to_string()),
            },
        )
        .unwrap();

        let path = accounts_file_path(data_dir);
        let raw = std::fs::read_to_string(&path).unwrap();

        assert!(
            !raw.to_lowercase().contains("access_token"),
            "accounts.json should not contain access_token"
        );
        assert!(
            !raw.to_lowercase().contains("refresh_token"),
            "accounts.json should not contain refresh_token"
        );
        assert!(
            !raw.contains("password"),
            "accounts.json should not contain password"
        );

        assert!(
            raw.contains("https://example.com/avatar.png"),
            "accounts.json should contain the avatar_url"
        );
    }




    fn prism_accounts_json(entries: &[serde_json::Value]) -> String {
        let obj = serde_json::json!({ "accounts": entries });
        serde_json::to_string(&obj).unwrap()
    }


    fn prism_offline_entry(username: &str, uuid: &str) -> serde_json::Value {
        serde_json::json!({
            "type": "Offline",
            "profile": { "id": uuid, "name": username }
        })
    }


    fn prism_msa_entry(username: &str, id: &str) -> serde_json::Value {
        serde_json::json!({
            "type": "MSA",
            "profile": { "id": id, "name": username }
        })
    }

    #[test]
    fn normalize_external_source_accepts_prism_multimc() {
        assert!(normalize_external_source("prism").is_ok());
        assert!(normalize_external_source("Prism").is_ok());
        assert!(normalize_external_source("  prism  ").is_ok());
        assert!(normalize_external_source("multimc").is_ok());
        assert!(normalize_external_source("MultiMC").is_ok());
        assert!(normalize_external_source("prism-multimc").is_ok());
        assert!(normalize_external_source("PRISM-MULTIMC").is_ok());
    }

    #[test]
    fn normalize_external_source_rejects_unknown() {
        let err = normalize_external_source("hmcl").unwrap_err();
        assert!(
            err.contains("不支持") || err.contains("仅支持"),
            "error should mention unsupported: {err}"
        );

        let err = normalize_external_source("pcl").unwrap_err();
        assert!(
            err.contains("不支持") || err.contains("仅支持"),
            "error should mention unsupported: {err}"
        );

        let err = normalize_external_source("").unwrap_err();
        assert!(err.contains("不支持"), "empty source should error: {err}");
    }

    #[test]
    fn normalize_external_kind_offline() {
        let kind = normalize_external_account_kind("Offline");
        assert_eq!(kind, Some(LauncherAccountKind::Offline));

        let kind = normalize_external_account_kind("offline");
        assert_eq!(kind, Some(LauncherAccountKind::Offline));

        let kind = normalize_external_account_kind("  Offline  ");
        assert_eq!(kind, Some(LauncherAccountKind::Offline));
    }

    #[test]
    fn normalize_external_kind_microsoft_variants() {
        let kind = normalize_external_account_kind("MSA");
        assert_eq!(kind, Some(LauncherAccountKind::Microsoft));

        let kind = normalize_external_account_kind("msa");
        assert_eq!(kind, Some(LauncherAccountKind::Microsoft));

        let kind = normalize_external_account_kind("microsoft");
        assert_eq!(kind, Some(LauncherAccountKind::Microsoft));

        let kind = normalize_external_account_kind("Microsoft");
        assert_eq!(kind, Some(LauncherAccountKind::Microsoft));

        let kind = normalize_external_account_kind("microsoft_account");
        assert_eq!(kind, Some(LauncherAccountKind::Microsoft));


        let kind = normalize_external_account_kind("mojang");
        assert!(kind.is_none());

        let kind = normalize_external_account_kind("");
        assert!(kind.is_none());
    }

    #[test]
    fn normalize_external_uuid_32hex_to_hyphenated() {
        let result = normalize_external_uuid("069a79f444e94726a5befca90e38aaf5").unwrap();
        assert_eq!(result, "069a79f4-44e9-4726-a5be-fca90e38aaf5");
    }

    #[test]
    fn normalize_external_uuid_accepts_hyphenated() {
        let result = normalize_external_uuid("069a79f4-44e9-4726-a5be-fca90e38aaf5").unwrap();
        assert_eq!(result, "069a79f4-44e9-4726-a5be-fca90e38aaf5");


        let result = normalize_external_uuid("069A79F4-44E9-4726-A5BE-FCA90E38AAF5").unwrap();
        assert_eq!(result, "069a79f4-44e9-4726-a5be-fca90e38aaf5");
    }

    #[test]
    fn normalize_external_uuid_rejects_invalid() {

        assert!(normalize_external_uuid("abc").is_err());


        assert!(normalize_external_uuid("").is_err());


        assert!(normalize_external_uuid("069a79f444e94726a5befca90e38aafg").is_err());


        assert!(normalize_external_uuid("xxxxxxx-xxxx-xxxx-xxxx-xxxxxxxxxxxx").is_err());
    }

    #[test]
    fn parse_prism_multimc_accounts_parses_valid_entries() {
        let entries = vec![
            prism_offline_entry("Steve", "069a79f444e94726a5befca90e38aaf5"),
            prism_msa_entry("Alex", "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa1"),
        ];
        let json = prism_accounts_json(&entries);

        let accounts = parse_prism_multimc_accounts(&json).unwrap();
        assert_eq!(accounts.len(), 2);

        assert_eq!(accounts[0].username, "Steve");
        assert_eq!(accounts[0].kind, LauncherAccountKind::Offline);
        assert!(accounts[0].uuid.contains('-'), "UUID should be hyphenated");
        assert!(!accounts[0].selected, "drafts must not be selected");
        assert!(accounts[0].auth_server_url.is_none());
        assert!(accounts[0].avatar_url.is_none());
        assert!(accounts[0].last_validated_at.is_none());
        assert!(accounts[0].token_expires_at.is_none());

        assert_eq!(accounts[1].username, "Alex");
        assert_eq!(accounts[1].kind, LauncherAccountKind::Microsoft);
        assert!(!accounts[1].selected);
    }

    #[test]
    fn parse_prism_multimc_accounts_rejects_non_array() {
        let json = r#"{"accounts": "not-an-array"}"#;
        let err = parse_prism_multimc_accounts(json).unwrap_err();
        assert!(
            err.contains("数组"),
            "error should mention array requirement: {err}"
        );
    }

    #[test]
    fn parse_prism_multimc_accounts_drops_unknown_type() {
        let entries = vec![
            prism_offline_entry("Keep", "069a79f444e94726a5befca90e38aaf5"),
            serde_json::json!({
                "type": "mojang",
                "profile": {"id": "aaaabbbbccccddddeeeeffff00000001", "name": "Drop"}
            }),
        ];
        let json = prism_accounts_json(&entries);

        let accounts = parse_prism_multimc_accounts(&json).unwrap();
        assert_eq!(accounts.len(), 1, "unknown type should be dropped");
        assert_eq!(accounts[0].username, "Keep");
    }

    #[test]
    fn import_prism_accounts_imports_offline_and_msa() {
        let dir = setup_temp_dir();
        let data_dir = dir.path();

        let entries = vec![
            prism_offline_entry("Steve", "069a79f444e94726a5befca90e38aaf5"),
            prism_msa_entry("Alex", "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa1"),
        ];
        let json = prism_accounts_json(&entries);

        let result = import_external_accounts_in(
            data_dir,
            ImportExternalAccountsRequest {
                source: "prism".to_string(),
                accounts_json: json,
                dedupe_by_uuid: false,
            },
        )
        .unwrap();

        assert_eq!(result.imported, 2);
        assert_eq!(result.skipped, 0);
        assert_eq!(result.failed, 0);
        assert_eq!(result.total, 2);

        let accounts = load_accounts(data_dir).unwrap();
        assert_eq!(accounts.len(), 2);
        assert!(accounts
            .iter()
            .any(|a| a.kind == LauncherAccountKind::Offline && a.username == "Steve"));
        assert!(accounts
            .iter()
            .any(|a| a.kind == LauncherAccountKind::Microsoft && a.username == "Alex"));
    }

    #[test]
    fn import_prism_accounts_skips_unknown_type() {
        let dir = setup_temp_dir();
        let data_dir = dir.path();

        let entries = vec![
            prism_offline_entry("Steve", "069a79f444e94726a5befca90e38aaf5"),
            serde_json::json!({
                "type": "mojang",
                "profile": { "id": "aaaabbbbccccddddeeeeffff00000001", "name": "Mojang" }
            }),
        ];
        let json = prism_accounts_json(&entries);

        let result = import_external_accounts_in(
            data_dir,
            ImportExternalAccountsRequest {
                source: "multimc".to_string(),
                accounts_json: json,
                dedupe_by_uuid: false,
            },
        )
        .unwrap();

        assert_eq!(result.imported, 1);
        assert_eq!(result.skipped, 1);
        assert_eq!(result.failed, 0);
        assert_eq!(result.total, 2);

        let accounts = load_accounts(data_dir).unwrap();
        assert_eq!(accounts.len(), 1);
        assert_eq!(accounts[0].username, "Steve");
    }

    #[test]
    fn import_prism_accounts_counts_missing_profile_as_failed() {
        let dir = setup_temp_dir();
        let data_dir = dir.path();


        let entries = vec![
            prism_offline_entry("Steve", "069a79f444e94726a5befca90e38aaf5"),
            serde_json::json!({
                "type": "Offline"

            }),
            serde_json::json!({
                "type": "MSA",
                "profile": { "id": "invalid-uuid-here-zz", "name": "BadUUID" }
            }),
            serde_json::json!({
                "type": "MSA",
                "profile": { "id": "aaaabbbbccccddddeeeeffff00000001" }

            }),
        ];
        let json = prism_accounts_json(&entries);

        let result = import_external_accounts_in(
            data_dir,
            ImportExternalAccountsRequest {
                source: "prism".to_string(),
                accounts_json: json,
                dedupe_by_uuid: false,
            },
        )
        .unwrap();

        assert_eq!(result.imported, 1);
        assert_eq!(result.skipped, 0);
        assert_eq!(result.failed, 3);
        assert_eq!(result.total, 4);

        let accounts = load_accounts(data_dir).unwrap();
        assert_eq!(accounts.len(), 1);
        assert_eq!(accounts[0].username, "Steve");
    }

    #[test]
    fn import_external_dedupe_by_kind_uuid() {
        let dir = setup_temp_dir();
        let data_dir = dir.path();


        let existing = LauncherAccount {
            id: "pre-seed-1".to_string(),
            kind: LauncherAccountKind::Offline,
            username: "Existing".to_string(),
            uuid: "069a79f4-44e9-4726-a5be-fca90e38aaf5".to_string(),
            selected: true,
            auth_server_url: None,
            avatar_url: None,
            created_at: utils::now_iso8601(),
            updated_at: utils::now_iso8601(),
            last_validated_at: None,
            token_expires_at: None,
        };
        save_accounts(data_dir, std::slice::from_ref(&existing)).unwrap();


        let entries = vec![prism_offline_entry(
            "Steve",
            "069a79f444e94726a5befca90e38aaf5",
        )];
        let json = prism_accounts_json(&entries);

        let result = import_external_accounts_in(
            data_dir,
            ImportExternalAccountsRequest {
                source: "prism".to_string(),
                accounts_json: json,
                dedupe_by_uuid: true,
            },
        )
        .unwrap();

        assert_eq!(result.imported, 0);
        assert_eq!(result.skipped, 1);
        assert_eq!(result.failed, 0);
        assert_eq!(result.total, 1);

        let accounts = load_accounts(data_dir).unwrap();
        assert_eq!(accounts.len(), 1);
        assert_eq!(accounts[0].username, "Existing");
    }

    #[test]
    fn import_external_does_not_touch_token_files() {
        let dir = setup_temp_dir();
        let data_dir = dir.path();


        let existing = LauncherAccount {
            id: "tok-acc".to_string(),
            kind: LauncherAccountKind::Microsoft,
            username: "TokenKeeper".to_string(),
            uuid: "aaaaaaaa-bbbb-cccc-dddd-eeeeeeeeeeee".to_string(),
            selected: true,
            auth_server_url: None,
            avatar_url: None,
            created_at: utils::now_iso8601(),
            updated_at: utils::now_iso8601(),
            last_validated_at: None,
            token_expires_at: None,
        };
        save_accounts(data_dir, std::slice::from_ref(&existing)).unwrap();
        save_account_token(data_dir, "tok-acc", "my-precious-token").unwrap();


        let entries = vec![prism_offline_entry(
            "Steve",
            "069a79f444e94726a5befca90e38aaf5",
        )];
        let json = prism_accounts_json(&entries);

        let result = import_external_accounts_in(
            data_dir,
            ImportExternalAccountsRequest {
                source: "prism".to_string(),
                accounts_json: json,
                dedupe_by_uuid: false,
            },
        )
        .unwrap();

        assert_eq!(result.imported, 1);


        let token = load_account_token(data_dir, "tok-acc").unwrap();
        assert_eq!(token, Some("my-precious-token".to_string()));


        let all_tokens = load_all_account_tokens(data_dir).unwrap();
        assert_eq!(all_tokens.len(), 1, "no new tokens should be saved");
    }

    #[test]
    fn import_external_never_selects_imported_accounts() {
        let dir = setup_temp_dir();
        let data_dir = dir.path();


        let existing = add_offline_account_in(
            data_dir,
            AddOfflineAccountRequest {
                username: "Existing".to_string(),
            },
        )
        .unwrap();
        assert!(existing.selected);


        let entries = vec![
            prism_offline_entry("Steve", "069a79f444e94726a5befca90e38aaf5"),
            prism_msa_entry("Alex", "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa1"),
        ];
        let json = prism_accounts_json(&entries);

        let result = import_external_accounts_in(
            data_dir,
            ImportExternalAccountsRequest {
                source: "multimc".to_string(),
                accounts_json: json,
                dedupe_by_uuid: false,
            },
        )
        .unwrap();

        assert_eq!(result.imported, 2);

        let accounts = load_accounts(data_dir).unwrap();
        assert_eq!(accounts.len(), 3);


        let selected_count = accounts.iter().filter(|a| a.selected).count();
        assert_eq!(
            selected_count, 1,
            "exactly one account should remain selected"
        );
        assert!(
            accounts
                .iter()
                .find(|a| a.id == existing.id)
                .unwrap()
                .selected
        );


        for a in &accounts {
            if a.id != existing.id {
                assert!(
                    !a.selected,
                    "imported account '{}' should not be selected",
                    a.username
                );
            }
        }
    }

    #[test]
    fn import_external_rejects_invalid_source() {
        let dir = setup_temp_dir();
        let data_dir = dir.path();

        let json = prism_accounts_json(&[]);
        let err = import_external_accounts_in(
            data_dir,
            ImportExternalAccountsRequest {
                source: "bad-launcher".to_string(),
                accounts_json: json,
                dedupe_by_uuid: false,
            },
        )
        .unwrap_err();

        assert!(
            err.contains("不支持") || err.contains("仅支持"),
            "error should mention unsupported source: {err}"
        );
    }

    #[test]
    fn import_external_rejects_accounts_not_array() {
        let dir = setup_temp_dir();
        let data_dir = dir.path();

        let json = r#"{"accounts": "not-an-array"}"#;
        let err = import_external_accounts_in(
            data_dir,
            ImportExternalAccountsRequest {
                source: "prism".to_string(),
                accounts_json: json.to_string(),
                dedupe_by_uuid: false,
            },
        )
        .unwrap_err();

        assert!(
            err.contains("数组"),
            "error should mention array requirement: {err}"
        );
    }

    #[test]
    fn import_external_compat_top_level_name_and_uuid() {
        let dir = setup_temp_dir();
        let data_dir = dir.path();


        let entries = vec![serde_json::json!({
            "type": "Offline",
            "name": "TopLevel",
            "uuid": "069a79f444e94726a5befca90e38aaf5"
        })];
        let json = prism_accounts_json(&entries);

        let result = import_external_accounts_in(
            data_dir,
            ImportExternalAccountsRequest {
                source: "prism".to_string(),
                accounts_json: json,
                dedupe_by_uuid: false,
            },
        )
        .unwrap();

        assert_eq!(result.imported, 1);
        let accounts = load_accounts(data_dir).unwrap();
        assert_eq!(accounts[0].username, "TopLevel");
    }

    #[test]
    fn import_external_compat_top_level_id_instead_of_uuid() {
        let dir = setup_temp_dir();
        let data_dir = dir.path();


        let entries = vec![serde_json::json!({
            "type": "MSA",
            "profile": {
                "id": "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa1",
                "name": "MsIdPlayer"
            }
        })];
        let json = prism_accounts_json(&entries);

        let result = import_external_accounts_in(
            data_dir,
            ImportExternalAccountsRequest {
                source: "multimc".to_string(),
                accounts_json: json,
                dedupe_by_uuid: false,
            },
        )
        .unwrap();

        assert_eq!(result.imported, 1);
        let accounts = load_accounts(data_dir).unwrap();
        assert_eq!(accounts[0].username, "MsIdPlayer");

        assert!(accounts[0].uuid.contains('-'));
    }

    #[test]
    fn import_external_preserves_existing_accounts() {
        let dir = setup_temp_dir();
        let data_dir = dir.path();

        let existing = add_offline_account_in(
            data_dir,
            AddOfflineAccountRequest {
                username: "PreExisting".to_string(),
            },
        )
        .unwrap();

        let entries = vec![prism_offline_entry(
            "Imported",
            "069a79f444e94726a5befca90e38aaf5",
        )];
        let json = prism_accounts_json(&entries);

        import_external_accounts_in(
            data_dir,
            ImportExternalAccountsRequest {
                source: "prism".to_string(),
                accounts_json: json,
                dedupe_by_uuid: false,
            },
        )
        .unwrap();

        let accounts = load_accounts(data_dir).unwrap();
        assert_eq!(accounts.len(), 2);
        assert!(
            accounts.iter().any(|a| a.id == existing.id),
            "existing account should be preserved"
        );
    }

    #[test]
    fn import_external_empty_accounts_array_ok() {
        let dir = setup_temp_dir();
        let data_dir = dir.path();

        let json = prism_accounts_json(&[]);
        let result = import_external_accounts_in(
            data_dir,
            ImportExternalAccountsRequest {
                source: "prism".to_string(),
                accounts_json: json,
                dedupe_by_uuid: false,
            },
        )
        .unwrap();

        assert_eq!(result.imported, 0);
        assert_eq!(result.skipped, 0);
        assert_eq!(result.failed, 0);
        assert_eq!(result.total, 0);
    }
}
