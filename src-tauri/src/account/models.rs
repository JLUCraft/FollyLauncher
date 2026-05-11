use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum LauncherAccountKind {
    Offline,
    Microsoft,
    ThirdParty,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LauncherAccount {
    pub id: String,
    pub kind: LauncherAccountKind,
    pub username: String,
    pub uuid: String,
    #[serde(default)]
    pub selected: bool,
    pub auth_server_url: Option<String>,
    pub avatar_url: Option<String>,
    pub created_at: String,
    pub updated_at: String,
    #[serde(default)]
    pub last_validated_at: Option<String>,
    #[serde(default)]
    pub token_expires_at: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AddOfflineAccountRequest {
    pub username: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AddThirdPartyAccountRequest {
    pub auth_server_url: String,
    pub username_or_email: String,
    pub password: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ThirdPartyLoginResult {
    pub account: LauncherAccount,
    pub token_saved: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MicrosoftDeviceAuthStartResult {
    pub device_code: String,
    pub user_code: String,
    pub verification_uri: String,
    #[serde(default)]
    pub verification_uri_complete: Option<String>,
    pub expires_in: u64,
    pub interval: u64,
    #[serde(default)]
    pub message: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MicrosoftLoginResult {
    pub account: LauncherAccount,
    pub token_saved: bool,
    #[serde(default)]
    pub refresh_token_saved: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MicrosoftRefreshResult {
    pub account: LauncherAccount,
    pub token_saved: bool,
    pub refresh_token_saved: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UpdateAccountAvatarRequest {
    pub account_id: String,
    pub avatar_url: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AccountAvatarResult {
    pub account: LauncherAccount,
    pub avatar_url: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AccountExportBundle {
    pub schema_version: u32,
    pub exported_at: String,
    pub accounts: Vec<LauncherAccount>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ImportAccountsRequest {
    pub bundle_json: String,
    pub dedupe_by_uuid: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ImportAccountsResult {
    pub imported: u32,
    pub skipped: u32,
    pub failed: u32,
    pub total: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ImportExternalAccountsRequest {
    pub source: String,
    pub accounts_json: String,
    pub dedupe_by_uuid: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ImportExternalAccountsResult {
    pub imported: u32,
    pub skipped: u32,
    pub failed: u32,
    pub total: u32,
}
