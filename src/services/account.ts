import { invoke } from "@tauri-apps/api/core";

export type VcHolderState = "Unverified" | "Member" | "Expired" | "Revoked";

export interface VcStatus {
  state: VcHolderState;
  role: string | null;
  club: string | null;
  issuer: string | null;
  verified: boolean;
  crl_stale: boolean;
}

export interface VcImportResult {
  state: VcHolderState;
  verified: boolean;
  expired: boolean;
}

export async function getVcStatus(): Promise<VcStatus> {
  return invoke("get_vc_status");
}

export async function importVc(vcJson: string): Promise<VcImportResult> {
  return invoke("import_vc", { vcJson });
}

export async function clearVc(): Promise<void> {
  return invoke("clear_vc");
}

export interface OnboardingStatus {
  peer_id: string;
  has_identity: boolean;
  vc_state: VcHolderState;
  mua_logged_in: boolean;
  is_guest: boolean;
  is_member: boolean;
  club: string | null;
  crl_stale: boolean;
  mode_label: string;
  next_steps: string[];
  requires_network: boolean;
}

export async function getOnboardingStatus(): Promise<OnboardingStatus> {
  return invoke("get_onboarding_status");
}

export interface MuaLoginStatus {
  logged_in: boolean;
  username: string | null;
  uuid: string | null;
  auth_server_url: string;
  peer_bound: boolean;
  is_guest: boolean;
  is_member: boolean;
}

export interface StartAuthResponse {
  user_code: string;
  verification_uri: string;
}

export async function getMuaStatus(): Promise<MuaLoginStatus> {
  return invoke("get_mua_status");
}

export async function startMuaLogin(): Promise<StartAuthResponse> {
  return invoke("start_mua_login");
}

export async function pollMuaLogin(): Promise<MuaLoginStatus> {
  return invoke("poll_mua_login");
}

export async function logoutMua(): Promise<void> {
  return invoke("logout_mua");
}

export interface SkinTextures {
  skin_url: string | null;
  cape_url: string | null;
}

export async function getSkinTextures(): Promise<SkinTextures> {
  return invoke("get_skin_textures");
}

export type LauncherAccountKind = "Offline" | "Microsoft" | "ThirdParty";

export interface LauncherAccount {
  id: string;
  kind: LauncherAccountKind;
  username: string;
  uuid: string;
  selected: boolean;
  auth_server_url: string | null;
  avatar_url: string | null;
  created_at: string;
  updated_at: string;
  last_validated_at: string | null;
  token_expires_at: string | null;
}

export interface AddOfflineAccountRequest {
  username: string;
}

export async function listLauncherAccounts(): Promise<LauncherAccount[]> {
  return invoke("list_launcher_accounts");
}

export async function addOfflineAccount(
  request: AddOfflineAccountRequest,
): Promise<LauncherAccount> {
  return invoke("add_offline_account", { request });
}

export async function selectLauncherAccount(id: string): Promise<LauncherAccount> {
  return invoke("select_launcher_account", { id });
}

export async function deleteLauncherAccount(id: string): Promise<void> {
  return invoke("delete_launcher_account", { id });
}

export interface AddThirdPartyAccountRequest {
  auth_server_url: string;
  username_or_email: string;
  password: string;
}

export interface ThirdPartyLoginResult {
  account: LauncherAccount;
  token_saved: boolean;
}

export async function addThirdPartyAccount(
  request: AddThirdPartyAccountRequest,
): Promise<ThirdPartyLoginResult> {
  return invoke("add_third_party_account", { request });
}

export interface MicrosoftDeviceAuthStartResult {
  device_code: string;
  user_code: string;
  verification_uri: string;
  verification_uri_complete: string | null;
  expires_in: number;
  interval: number;
  message: string | null;
}

export interface MicrosoftLoginResult {
  account: LauncherAccount;
  token_saved: boolean;
  refresh_token_saved: boolean;
}

export interface MicrosoftRefreshResult {
  account: LauncherAccount;
  token_saved: boolean;
  refresh_token_saved: boolean;
}

export async function startMicrosoftLogin(): Promise<MicrosoftDeviceAuthStartResult> {
  return invoke("start_microsoft_login");
}

export async function pollMicrosoftLogin(
  deviceCode: string,
): Promise<MicrosoftLoginResult> {
  return invoke("poll_microsoft_login", { deviceCode });
}

export async function refreshMicrosoftAccount(
  accountId: string,
): Promise<MicrosoftRefreshResult> {
  return invoke("refresh_microsoft_account", { accountId });
}



export interface UpdateAccountAvatarRequest {
  account_id: string;
  avatar_url: string | null;
}

export interface AccountAvatarResult {
  account: LauncherAccount;
  avatar_url: string | null;
}

export async function updateAccountAvatar(
  request: UpdateAccountAvatarRequest,
): Promise<AccountAvatarResult> {
  return invoke("update_account_avatar", { request });
}

export async function refreshAccountAvatar(
  accountId: string,
): Promise<AccountAvatarResult> {
  return invoke("refresh_account_avatar", { accountId });
}



export interface AccountExportBundle {
  schema_version: number;
  exported_at: string;
  accounts: LauncherAccount[];
}

export interface ImportAccountsRequest {
  bundle_json: string;
  dedupe_by_uuid: boolean;
}

export interface ImportAccountsResult {
  imported: number;
  skipped: number;
  failed: number;
  total: number;
}

export async function exportLauncherAccounts(): Promise<AccountExportBundle> {
  return invoke("export_launcher_accounts");
}

export async function importLauncherAccounts(
  request: ImportAccountsRequest,
): Promise<ImportAccountsResult> {
  return invoke("import_launcher_accounts", { request });
}



export interface ImportExternalAccountsRequest {
  source: string;
  accounts_json: string;
  dedupe_by_uuid: boolean;
}

export interface ImportExternalAccountsResult {
  imported: number;
  skipped: number;
  failed: number;
  total: number;
}

export async function importExternalAccounts(
  request: ImportExternalAccountsRequest,
): Promise<ImportExternalAccountsResult> {
  return invoke("import_external_accounts", { request });
}
