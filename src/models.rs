use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AccountRecord {
    pub id: String,
    pub username: String,
    pub uuid: String,
    pub account_type: AccountType,
    pub active: bool,
    pub skin_url: Option<String>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum AccountType {
    Microsoft,
    Offline,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AuthTokens {
    pub access_token: String,
    pub refresh_token: Option<String>,
    pub expires_at: u64,
    pub user_type: String,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PublicAccount {
    pub id: String,
    pub username: String,
    pub uuid: String,
    pub account_type: AccountType,
    pub active: bool,
    pub skin_url: Option<String>,
}

impl From<&AccountRecord> for PublicAccount {
    fn from(account: &AccountRecord) -> Self {
        Self {
            id: account.id.clone(),
            username: account.username.clone(),
            uuid: account.uuid.clone(),
            account_type: account.account_type.clone(),
            active: account.active,
            skin_url: account.skin_url.clone(),
        }
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DeviceCode {
    pub device_code: String,
    pub user_code: String,
    pub verification_uri: String,
    pub expires_in: u64,
    pub interval: u64,
    pub message: String,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct InstanceRecord {
    pub id: String,
    pub name: String,
    pub version_id: String,
    pub game_version_id: String,
    pub loader: Option<String>,
    pub loader_version: Option<String>,
    pub ram_mb: u32,
    pub java_path: Option<String>,
    pub jvm_args: Vec<String>,
    pub created_at: u64,
    pub last_played_at: Option<u64>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct LauncherSettings {
    pub language: String,
    pub ram_mb: u32,
    pub java_path: Option<String>,
    pub server_address: String,
}

impl Default for LauncherSettings {
    fn default() -> Self {
        Self {
            language: "en".to_string(),
            ram_mb: 4096,
            java_path: None,
            server_address: String::new(),
        }
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct VersionSummary {
    pub id: String,
    pub kind: String,
    pub release_time: String,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ModSearchResult {
    pub project_id: String,
    pub title: String,
    pub description: String,
    pub author: String,
    pub downloads: u64,
    pub icon_url: Option<String>,
    pub latest_version: Option<String>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CrashReport {
    pub instance_id: String,
    pub filename: String,
    pub modified_at: u64,
    pub preview: String,
}
