use crate::models::{AccountRecord, AccountType, AuthTokens, DeviceCode};
use anyhow::{bail, Context, Result};
use reqwest::{Client, StatusCode};
use serde::Deserialize;
use serde_json::json;
use std::time::{SystemTime, UNIX_EPOCH};
use uuid::Uuid;

const DEVICE_CODE_URL: &str = "https://login.microsoftonline.com/consumers/oauth2/v2.0/devicecode";
const TOKEN_URL: &str = "https://login.microsoftonline.com/consumers/oauth2/v2.0/token";
const XBOX_AUTH_URL: &str = "https://user.auth.xboxlive.com/user/authenticate";
const XSTS_URL: &str = "https://xsts.auth.xboxlive.com/xsts/authorize";
const MINECRAFT_LOGIN_URL: &str =
    "https://api.minecraftservices.com/authentication/login_with_xbox";
const MINECRAFT_PROFILE_URL: &str = "https://api.minecraftservices.com/minecraft/profile";
const MICROSOFT_CLIENT_ID: Option<&str> = option_env!("LUMEN_MICROSOFT_CLIENT_ID");

#[derive(Debug, Deserialize)]
struct OAuthToken {
    access_token: String,
    refresh_token: Option<String>,
    expires_in: u64,
}

#[derive(Debug, Deserialize)]
struct XboxToken {
    #[serde(rename = "Token")]
    token: String,
    #[serde(rename = "DisplayClaims")]
    display_claims: DisplayClaims,
}

#[derive(Debug, Deserialize)]
struct DisplayClaims {
    xui: Vec<Xui>,
}

#[derive(Debug, Deserialize)]
struct Xui {
    uhs: String,
}

#[derive(Debug, Deserialize)]
struct MinecraftToken {
    access_token: String,
    expires_in: u64,
}

#[derive(Debug, Deserialize)]
struct MinecraftProfile {
    id: String,
    name: String,
    skins: Option<Vec<Skin>>,
}

#[derive(Debug, Deserialize)]
struct Skin {
    url: String,
    state: Option<String>,
}

pub fn client_id() -> Result<&'static str> {
    MICROSOFT_CLIENT_ID
        .filter(|id| !id.trim().is_empty())
        .ok_or_else(|| anyhow::anyhow!("Microsoft sign-in is not configured. Set LUMEN_MICROSOFT_CLIENT_ID at build time to your Azure application client ID."))
}

pub async fn begin_device_login() -> Result<DeviceCode> {
    let response = Client::new()
        .post(DEVICE_CODE_URL)
        .form(&[
            ("client_id", client_id()?),
            ("scope", "XboxLive.signin offline_access"),
        ])
        .send()
        .await
        .context("Could not start Microsoft device sign-in")?
        .error_for_status()
        .context("Microsoft rejected the device sign-in request")?;
    response
        .json()
        .await
        .context("Microsoft returned an invalid device sign-in response")
}

pub async fn finish_device_login(device_code: &str) -> Result<(AccountRecord, AuthTokens)> {
    let client = Client::new();
    let oauth = client
        .post(TOKEN_URL)
        .form(&[
            ("grant_type", "urn:ietf:params:oauth:grant-type:device_code"),
            ("client_id", client_id()?),
            ("device_code", device_code),
        ])
        .send()
        .await
        .context("Could not check Microsoft sign-in status")?;

    if !oauth.status().is_success() {
        let status = oauth.status();
        let body = oauth.text().await.unwrap_or_default();
        if body.contains("authorization_pending") {
            bail!("Microsoft sign-in is still waiting for approval");
        }
        if body.contains("slow_down") {
            bail!("Microsoft sign-in asked the launcher to poll more slowly");
        }
        bail!(
            "Microsoft sign-in failed ({status}): {}",
            oauth_error_message(&body)
        );
    }
    let microsoft: OAuthToken = oauth
        .json()
        .await
        .context("Microsoft returned an invalid token response")?;
    exchange_tokens(client, microsoft).await
}

pub async fn refresh_account(tokens: &AuthTokens) -> Result<AuthTokens> {
    let refresh_token = tokens
        .refresh_token
        .as_deref()
        .context("This account does not have a refresh token")?;
    let response = Client::new()
        .post(TOKEN_URL)
        .form(&[
            ("grant_type", "refresh_token"),
            ("client_id", client_id()?),
            ("refresh_token", refresh_token),
            ("scope", "XboxLive.signin offline_access"),
        ])
        .send()
        .await
        .context("Could not refresh the Microsoft session")?
        .error_for_status()
        .context("Microsoft session refresh was rejected")?;
    let microsoft: OAuthToken = response
        .json()
        .await
        .context("Microsoft returned an invalid refresh response")?;
    let (_, refreshed) = exchange_tokens(Client::new(), microsoft).await?;
    Ok(refreshed)
}

async fn exchange_tokens(
    client: Client,
    microsoft: OAuthToken,
) -> Result<(AccountRecord, AuthTokens)> {
    let xbox_response = client
        .post(XBOX_AUTH_URL)
        .json(&json!({
            "Properties": {
                "AuthMethod": "RPS",
                "SiteName": "user.auth.xboxlive.com",
                "RpsTicket": format!("d={}", microsoft.access_token)
            },
            "RelyingParty": "http://auth.xboxlive.com",
            "TokenType": "JWT"
        }))
        .send()
        .await
        .context("Xbox Live authentication request failed")?
        .error_for_status()
        .context("Xbox Live rejected this Microsoft account")?
        .json::<XboxToken>()
        .await
        .context("Xbox Live returned an invalid response")?;
    let user_hash = xbox_response
        .display_claims
        .xui
        .first()
        .context("Xbox Live did not return a user identifier")?
        .uhs
        .clone();

    let xsts_response = client
        .post(XSTS_URL)
        .json(&json!({
            "Properties": { "SandboxId": "RETAIL", "UserTokens": [xbox_response.token] },
            "RelyingParty": "rp://api.minecraftservices.com/",
            "TokenType": "JWT"
        }))
        .send()
        .await
        .context("Xbox XSTS authentication request failed")?;
    if !xsts_response.status().is_success() {
        let status = xsts_response.status();
        let body = xsts_response.text().await.unwrap_or_default();
        bail!(
            "Xbox XSTS rejected this account ({status}): {}",
            oauth_error_message(&body)
        );
    }
    let xsts: XboxToken = xsts_response
        .json()
        .await
        .context("Xbox XSTS returned an invalid response")?;
    let xsts_hash = xsts
        .display_claims
        .xui
        .first()
        .context("Xbox XSTS did not return a user identifier")?
        .uhs
        .clone();

    let minecraft_response = client
        .post(MINECRAFT_LOGIN_URL)
        .json(&json!({
            "identityToken": format!("XBL3.0 x={xsts_hash};{}", xsts.token)
        }))
        .send()
        .await
        .context("Minecraft Services login request failed")?
        .error_for_status()
        .context("Minecraft Services rejected the Xbox session")?;
    let minecraft: MinecraftToken = minecraft_response
        .json()
        .await
        .context("Minecraft Services returned an invalid token")?;
    let profile_response = client
        .get(MINECRAFT_PROFILE_URL)
        .bearer_auth(&minecraft.access_token)
        .send()
        .await
        .context("Could not check Minecraft Java ownership")?;
    if profile_response.status() == StatusCode::NOT_FOUND {
        bail!("This Microsoft account does not own Minecraft: Java Edition.");
    }
    let profile: MinecraftProfile = profile_response
        .error_for_status()
        .context("Could not verify Minecraft Java profile ownership")?
        .json()
        .await
        .context("Minecraft Services returned an invalid profile")?;
    let skin_url = profile.skins.and_then(|skins| {
        skins
            .iter()
            .find(|skin| skin.state.as_deref() == Some("ACTIVE"))
            .or_else(|| skins.first())
            .map(|skin| skin.url.clone())
    });
    let now = SystemTime::now().duration_since(UNIX_EPOCH)?.as_secs();
    let account = AccountRecord {
        id: profile.id.clone(),
        username: profile.name,
        uuid: profile.id,
        account_type: AccountType::Microsoft,
        active: false,
        skin_url,
    };
    let tokens = AuthTokens {
        access_token: minecraft.access_token,
        refresh_token: microsoft.refresh_token,
        expires_at: now + minecraft.expires_in.min(microsoft.expires_in),
        user_type: "msa".to_string(),
    };
    let _ = user_hash;
    Ok((account, tokens))
}

fn oauth_error_message(body: &str) -> String {
    serde_json::from_str::<serde_json::Value>(body)
        .ok()
        .and_then(|v| {
            v.get("error_description")
                .or_else(|| v.get("XErr"))
                .map(|v| v.to_string())
        })
        .unwrap_or_else(|| "check the Microsoft account and try again".to_string())
}

pub fn offline_account(username: &str) -> Result<(AccountRecord, AuthTokens)> {
    let username = username.trim();
    if username.is_empty() || username.chars().count() > 16 {
        bail!("Offline usernames must be between 1 and 16 characters");
    }
    if !username
        .chars()
        .all(|ch| ch.is_ascii_alphanumeric() || ch == '_')
    {
        bail!("Offline usernames may contain only letters, numbers, and underscores");
    }
    let mut digest = md5::compute(format!("OfflinePlayer:{username}").as_bytes()).0;
    digest[6] = (digest[6] & 0x0f) | 0x30;
    digest[8] = (digest[8] & 0x3f) | 0x80;
    let uuid = Uuid::from_bytes(digest).to_string();
    let id = format!("offline-{uuid}");
    Ok((
        AccountRecord {
            id: id.clone(),
            username: username.to_string(),
            uuid,
            account_type: AccountType::Offline,
            active: false,
            skin_url: None,
        },
        AuthTokens {
            access_token: format!("offline_access_token_{}", Uuid::new_v4().simple()),
            refresh_token: None,
            expires_at: u64::MAX,
            user_type: "legacy".to_string(),
        },
    ))
}

#[cfg(test)]
mod tests {
    use super::offline_account;

    #[test]
    fn offline_uuid_matches_minecraft_java_name_uuid() {
        let (account, _) = offline_account("Notch").expect("valid offline account");
        assert_eq!(account.uuid, "b50ad385-829d-3141-a216-7e7d7539ba7f");
    }

    #[test]
    fn offline_names_are_validated() {
        assert!(offline_account("bad name").is_err());
        assert!(offline_account("").is_err());
        assert!(offline_account("A_valid_name").is_ok());
    }
}
