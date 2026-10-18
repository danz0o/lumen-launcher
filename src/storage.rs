use crate::models::{AccountRecord, AuthTokens, InstanceRecord, LauncherSettings};
use anyhow::{Context, Result};
use keyring::Entry;
use std::path::{Path, PathBuf};
use tauri::{AppHandle, Manager};
use uuid::Uuid;

const KEYRING_SERVICE: &str = "com.lumenlauncher.desktop";

pub fn data_dir(app: &AppHandle) -> Result<PathBuf> {
    let path = app
        .path()
        .app_data_dir()
        .context("Could not locate the app data directory")?;
    std::fs::create_dir_all(&path)
        .with_context(|| format!("Could not create {}", path.display()))?;
    Ok(path)
}

fn read_json<T: serde::de::DeserializeOwned>(path: &Path) -> Result<T> {
    let content =
        std::fs::read(path).with_context(|| format!("Could not read {}", path.display()))?;
    serde_json::from_slice(&content).with_context(|| format!("Invalid JSON in {}", path.display()))
}

fn write_json<T: serde::Serialize>(path: &Path, value: &T) -> Result<()> {
    let parent = path
        .parent()
        .context("Storage path has no parent directory")?;
    std::fs::create_dir_all(parent)?;
    let temporary = path.with_extension(format!("json.tmp.{}", Uuid::new_v4().simple()));
    let bytes = serde_json::to_vec_pretty(value)?;
    std::fs::write(&temporary, bytes)?;
    #[cfg(windows)]
    if path.exists() {
        std::fs::remove_file(path)?;
    }
    std::fs::rename(&temporary, path).with_context(|| format!("Could not save {}", path.display()))
}

pub fn accounts_file(root: &Path) -> PathBuf {
    root.join("accounts.json")
}

pub fn load_accounts(root: &Path) -> Result<Vec<AccountRecord>> {
    let path = accounts_file(root);
    if !path.exists() {
        return Ok(Vec::new());
    }
    read_json(&path)
}

pub fn save_accounts(root: &Path, accounts: &[AccountRecord]) -> Result<()> {
    write_json(&accounts_file(root), &accounts)
}

pub fn load_settings(root: &Path) -> Result<LauncherSettings> {
    let path = root.join("settings.json");
    if !path.exists() {
        return Ok(LauncherSettings::default());
    }
    read_json(&path)
}

pub fn save_settings(root: &Path, settings: &LauncherSettings) -> Result<()> {
    write_json(&root.join("settings.json"), settings)
}

pub fn save_instance(root: &Path, instance: &InstanceRecord) -> Result<()> {
    let folder = root.join("instances").join(&instance.id);
    std::fs::create_dir_all(&folder)?;
    write_json(&folder.join("lumen-instance.json"), instance)
}

pub fn load_instances(root: &Path) -> Result<Vec<InstanceRecord>> {
    let folder = root.join("instances");
    if !folder.exists() {
        return Ok(Vec::new());
    }
    let mut instances = Vec::new();
    for entry in std::fs::read_dir(folder)? {
        let entry = entry?;
        let manifest = entry.path().join("lumen-instance.json");
        if manifest.is_file() {
            let instance: InstanceRecord = read_json(&manifest)?;
            instances.push(instance);
        }
    }
    instances.sort_by(|a, b| a.name.to_lowercase().cmp(&b.name.to_lowercase()));
    Ok(instances)
}

pub fn delete_instance(root: &Path, id: &str) -> Result<()> {
    let base = root.join("instances").canonicalize().or_else(|_| {
        std::fs::create_dir_all(root.join("instances"))?;
        root.join("instances").canonicalize()
    })?;
    let target = root.join("instances").join(id);
    if !target.exists() {
        anyhow::bail!("Instance does not exist");
    }
    let resolved = target.canonicalize()?;
    if !resolved.starts_with(&base) || resolved == base {
        anyhow::bail!("Refusing to remove a path outside the instances folder");
    }
    std::fs::remove_dir_all(resolved)?;
    Ok(())
}

pub fn store_tokens(account_id: &str, tokens: &AuthTokens) -> Result<()> {
    let entry = Entry::new(KEYRING_SERVICE, account_id)
        .context("Could not access the operating system keyring")?;
    entry
        .set_password(&serde_json::to_string(tokens)?)
        .context("Could not store account credentials in the operating system keyring")
}

pub fn read_tokens(account_id: &str) -> Result<AuthTokens> {
    let entry = Entry::new(KEYRING_SERVICE, account_id)
        .context("Could not access the operating system keyring")?;
    let value = entry
        .get_password()
        .context("No saved credentials were found for this account")?;
    serde_json::from_str(&value).context("Saved credentials could not be decoded")
}

pub fn delete_tokens(account_id: &str) -> Result<()> {
    let entry = Entry::new(KEYRING_SERVICE, account_id)
        .context("Could not access the operating system keyring")?;
    match entry.delete_credential() {
        Ok(()) => Ok(()),
        Err(keyring::Error::NoEntry) => Ok(()),
        Err(error) => {
            Err(error).context("Could not remove credentials from the operating system keyring")
        }
    }
}
