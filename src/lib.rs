mod auth;
mod instances;
mod java;
mod launch;
mod loaders;
mod models;
mod mods;
mod storage;
mod versions;

use anyhow::Result;
use models::{AccountRecord, AccountType, InstanceRecord, LauncherSettings, PublicAccount};
use serde::Serialize;
use std::path::Path;
use tauri::{AppHandle, Emitter};

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct LauncherState {
    accounts: Vec<PublicAccount>,
    instances: Vec<InstanceRecord>,
    settings: LauncherSettings,
    data_directory: String,
}

fn root(app: &AppHandle) -> Result<std::path::PathBuf> {
    storage::data_dir(app)
}

fn error_string(error: impl std::fmt::Display) -> String {
    error.to_string()
}

#[tauri::command]
fn get_launcher_state(app: AppHandle) -> std::result::Result<LauncherState, String> {
    let root = root(&app).map_err(error_string)?;
    let accounts = storage::load_accounts(&root).map_err(error_string)?;
    let instances = storage::load_instances(&root).map_err(error_string)?;
    let settings = storage::load_settings(&root).map_err(error_string)?;
    Ok(LauncherState {
        accounts: accounts.iter().map(PublicAccount::from).collect(),
        instances,
        settings,
        data_directory: root.to_string_lossy().to_string(),
    })
}

#[tauri::command]
fn get_accounts(app: AppHandle) -> std::result::Result<Vec<PublicAccount>, String> {
    storage::load_accounts(&root(&app).map_err(error_string)?)
        .map(|accounts| accounts.iter().map(PublicAccount::from).collect())
        .map_err(error_string)
}

#[tauri::command]
fn add_offline_account(
    app: AppHandle,
    username: String,
) -> std::result::Result<PublicAccount, String> {
    let root = root(&app).map_err(error_string)?;
    let (mut account, tokens) = auth::offline_account(&username).map_err(error_string)?;
    let mut accounts = storage::load_accounts(&root).map_err(error_string)?;
    if accounts.iter().any(|existing| existing.id == account.id) {
        return Err("This offline account is already in the account list".to_string());
    }
    account.active = !accounts.iter().any(|existing| existing.active);
    storage::store_tokens(&account.id, &tokens).map_err(error_string)?;
    accounts.push(account.clone());
    storage::save_accounts(&root, &accounts).map_err(error_string)?;
    Ok(PublicAccount::from(&account))
}

#[tauri::command]
async fn begin_microsoft_login() -> std::result::Result<models::DeviceCode, String> {
    auth::begin_device_login().await.map_err(error_string)
}

#[tauri::command]
async fn finish_microsoft_login(
    app: AppHandle,
    device_code: String,
) -> std::result::Result<PublicAccount, String> {
    let root = root(&app).map_err(error_string)?;
    let (mut account, tokens) = auth::finish_device_login(&device_code)
        .await
        .map_err(error_string)?;
    let mut accounts = storage::load_accounts(&root).map_err(error_string)?;
    account.active = accounts
        .iter()
        .find(|existing| existing.id == account.id)
        .map(|existing| existing.active)
        .unwrap_or_else(|| !accounts.iter().any(|existing| existing.active));
    storage::store_tokens(&account.id, &tokens).map_err(error_string)?;
    if let Some(existing) = accounts
        .iter_mut()
        .find(|existing| existing.id == account.id)
    {
        *existing = account.clone();
    } else {
        accounts.push(account.clone());
    }
    storage::save_accounts(&root, &accounts).map_err(error_string)?;
    Ok(PublicAccount::from(&account))
}

#[tauri::command]
fn set_active_account(
    app: AppHandle,
    account_id: String,
) -> std::result::Result<Vec<PublicAccount>, String> {
    let root = root(&app).map_err(error_string)?;
    let mut accounts = storage::load_accounts(&root).map_err(error_string)?;
    if !accounts.iter().any(|account| account.id == account_id) {
        return Err("Account not found".to_string());
    }
    for account in &mut accounts {
        account.active = account.id == account_id;
    }
    storage::save_accounts(&root, &accounts).map_err(error_string)?;
    Ok(accounts.iter().map(PublicAccount::from).collect())
}

#[tauri::command]
fn remove_account(
    app: AppHandle,
    account_id: String,
) -> std::result::Result<Vec<PublicAccount>, String> {
    let root = root(&app).map_err(error_string)?;
    let mut accounts = storage::load_accounts(&root).map_err(error_string)?;
    let was_active = accounts
        .iter()
        .any(|account| account.id == account_id && account.active);
    if !accounts.iter().any(|account| account.id == account_id) {
        return Err("Account not found".to_string());
    }
    storage::delete_tokens(&account_id).map_err(error_string)?;
    accounts.retain(|account| account.id != account_id);
    if was_active {
        if let Some(first) = accounts.first_mut() {
            first.active = true;
        }
    }
    storage::save_accounts(&root, &accounts).map_err(error_string)?;
    Ok(accounts.iter().map(PublicAccount::from).collect())
}

#[tauri::command]
async fn list_versions() -> std::result::Result<Vec<models::VersionSummary>, String> {
    versions::list_versions().await.map_err(error_string)
}

#[tauri::command]
async fn install_game_version(
    app: AppHandle,
    version_id: String,
) -> std::result::Result<(), String> {
    let root = root(&app).map_err(error_string)?;
    let result = versions::install_version_with_events(&app, &root, &version_id).await;
    match result {
        Ok(()) => {
            let _ = app.emit("install-progress", serde_json::json!({"versionId": version_id, "progress": 100, "message": "Installation complete"}));
            Ok(())
        }
        Err(error) => {
            let _ = app.emit("install-progress", serde_json::json!({"versionId": version_id, "progress": 0, "message": error.to_string()}));
            Err(error_string(error))
        }
    }
}

#[tauri::command]
fn list_instances(app: AppHandle) -> std::result::Result<Vec<InstanceRecord>, String> {
    storage::load_instances(&root(&app).map_err(error_string)?).map_err(error_string)
}

#[tauri::command]
async fn create_instance(
    app: AppHandle,
    name: String,
    version_id: String,
) -> std::result::Result<InstanceRecord, String> {
    let root = root(&app).map_err(error_string)?;
    let mut instance = instances::create(&root, &name, &version_id).map_err(error_string)?;
    if let Ok(settings) = storage::load_settings(&root) {
        instance.ram_mb = settings.ram_mb;
        storage::save_instance(&root, &instance).map_err(error_string)?;
    }
    if let Err(error) = versions::install_version_with_events(&app, &root, &version_id).await {
        let _ = instances::delete(&root, &instance.id);
        return Err(error_string(error));
    }
    Ok(instance)
}

#[tauri::command]
fn clone_instance(
    app: AppHandle,
    instance_id: String,
    name: String,
) -> std::result::Result<InstanceRecord, String> {
    instances::clone_instance(&root(&app).map_err(error_string)?, &instance_id, &name)
        .map_err(error_string)
}

#[tauri::command]
fn delete_instance(app: AppHandle, instance_id: String) -> std::result::Result<(), String> {
    instances::delete(&root(&app).map_err(error_string)?, &instance_id).map_err(error_string)
}

#[tauri::command]
fn update_instance(
    app: AppHandle,
    instance: InstanceRecord,
) -> std::result::Result<InstanceRecord, String> {
    if !(1024..=65536).contains(&instance.ram_mb) {
        return Err("Memory allocation must be between 1024 MB and 65536 MB".to_string());
    }
    if instance.jvm_args.len() > 64
        || instance
            .jvm_args
            .iter()
            .any(|arg| arg.len() > 512 || arg.contains('\0'))
    {
        return Err("JVM arguments contain an invalid value".to_string());
    }
    let root = root(&app).map_err(error_string)?;
    let mut existing = instances::load_instance(&root, &instance.id).map_err(error_string)?;
    existing.name = instance.name.trim().to_string();
    existing.ram_mb = instance.ram_mb;
    existing.java_path = instance.java_path.filter(|value| !value.trim().is_empty());
    existing.jvm_args = instance.jvm_args;
    storage::save_instance(&root, &existing).map_err(error_string)?;
    Ok(existing)
}

#[tauri::command]
async fn install_loader(
    app: AppHandle,
    instance_id: String,
    loader: String,
    loader_version: Option<String>,
) -> std::result::Result<InstanceRecord, String> {
    let root = root(&app).map_err(error_string)?;
    let mut instance = instances::load_instance(&root, &instance_id).map_err(error_string)?;
    let version = loaders::install(&root, &mut instance, &loader, loader_version.as_deref())
        .await
        .map_err(error_string)?;
    instance.version_id = version;
    storage::save_instance(&root, &instance).map_err(error_string)?;
    Ok(instance)
}

#[tauri::command]
async fn search_modrinth(
    query: String,
    game_version: Option<String>,
    loader: Option<String>,
) -> std::result::Result<Vec<models::ModSearchResult>, String> {
    mods::search(&query, game_version.as_deref(), loader.as_deref())
        .await
        .map_err(error_string)
}

#[tauri::command]
async fn install_mod(
    app: AppHandle,
    instance_id: String,
    project_id: String,
) -> std::result::Result<Vec<String>, String> {
    let root = root(&app).map_err(error_string)?;
    let instance = instances::load_instance(&root, &instance_id).map_err(error_string)?;
    mods::install_project(&root, &instance, &project_id)
        .await
        .map_err(error_string)
}

#[tauri::command]
fn list_mods(app: AppHandle, instance_id: String) -> std::result::Result<Vec<String>, String> {
    let root = root(&app).map_err(error_string)?;
    let instance = instances::load_instance(&root, &instance_id).map_err(error_string)?;
    mods::list_installed(&root, &instance).map_err(error_string)
}

#[tauri::command]
fn set_mod_enabled(
    app: AppHandle,
    instance_id: String,
    filename: String,
    enabled: bool,
) -> std::result::Result<(), String> {
    let root = root(&app).map_err(error_string)?;
    let instance = instances::load_instance(&root, &instance_id).map_err(error_string)?;
    mods::set_enabled(&root, &instance, &filename, enabled).map_err(error_string)
}

#[tauri::command]
async fn import_modpack(
    app: AppHandle,
    source_path: String,
    name: Option<String>,
) -> std::result::Result<InstanceRecord, String> {
    mods::import_mrpack(
        &root(&app).map_err(error_string)?,
        Path::new(&source_path),
        name.as_deref(),
    )
    .await
    .map_err(error_string)
}

#[tauri::command]
fn export_instance(
    app: AppHandle,
    instance_id: String,
    destination: String,
) -> std::result::Result<(), String> {
    instances::export_zip(
        &root(&app).map_err(error_string)?,
        &instance_id,
        Path::new(&destination),
    )
    .map_err(error_string)
}

#[tauri::command]
fn import_instance(
    app: AppHandle,
    source_path: String,
    name: String,
    version_id: String,
) -> std::result::Result<InstanceRecord, String> {
    instances::import_zip(
        &root(&app).map_err(error_string)?,
        Path::new(&source_path),
        &name,
        &version_id,
    )
    .map_err(error_string)
}

#[tauri::command]
async fn launch_instance(app: AppHandle, instance_id: String) -> std::result::Result<u32, String> {
    let root = root(&app).map_err(error_string)?;
    let mut instance = instances::load_instance(&root, &instance_id).map_err(error_string)?;
    launch::launch(&app, &root, &mut instance)
        .await
        .map_err(error_string)
}

#[tauri::command]
fn get_settings(app: AppHandle) -> std::result::Result<LauncherSettings, String> {
    storage::load_settings(&root(&app).map_err(error_string)?).map_err(error_string)
}

#[tauri::command]
fn save_settings(
    app: AppHandle,
    settings: LauncherSettings,
) -> std::result::Result<LauncherSettings, String> {
    if !(1024..=65536).contains(&settings.ram_mb) {
        return Err("Memory allocation must be between 1024 MB and 65536 MB".to_string());
    }
    if !matches!(settings.language.as_str(), "en" | "ar") {
        return Err("Unsupported language".to_string());
    }
    if settings.server_address.len() > 255 || settings.server_address.contains('\0') {
        return Err("Server address is too long or invalid".to_string());
    }
    let root = root(&app).map_err(error_string)?;
    storage::save_settings(&root, &settings).map_err(error_string)?;
    Ok(settings)
}

#[tauri::command]
fn list_crash_reports(app: AppHandle) -> std::result::Result<Vec<models::CrashReport>, String> {
    launch::crash_reports(&root(&app).map_err(error_string)?).map_err(error_string)
}

#[tauri::command]
fn read_crash_report(
    app: AppHandle,
    instance_id: String,
    filename: String,
) -> std::result::Result<String, String> {
    launch::read_crash_report(&root(&app).map_err(error_string)?, &instance_id, &filename)
        .map_err(error_string)
}

#[tauri::command]
async fn install_java_runtime(app: AppHandle, major: u8) -> std::result::Result<String, String> {
    if !matches!(major, 8 | 17 | 21) {
        return Err("Supported managed Java versions are 8, 17, and 21".to_string());
    }
    java::ensure_runtime(&root(&app).map_err(error_string)?, major)
        .await
        .map(|path| path.to_string_lossy().to_string())
        .map_err(error_string)
}

#[tauri::command]
fn get_account_type(account: AccountRecord) -> String {
    match account.account_type {
        AccountType::Microsoft => "microsoft".to_string(),
        AccountType::Offline => "offline".to_string(),
    }
}

#[tauri::command]
fn get_install_directory(app: AppHandle) -> std::result::Result<String, String> {
    root(&app)
        .map(|path| path.to_string_lossy().to_string())
        .map_err(error_string)
}

pub fn run() {
    let result = tauri::Builder::default()
        .invoke_handler(tauri::generate_handler![
            get_launcher_state,
            get_accounts,
            add_offline_account,
            begin_microsoft_login,
            finish_microsoft_login,
            set_active_account,
            remove_account,
            list_versions,
            install_game_version,
            list_instances,
            create_instance,
            clone_instance,
            delete_instance,
            update_instance,
            install_loader,
            search_modrinth,
            install_mod,
            list_mods,
            set_mod_enabled,
            import_modpack,
            export_instance,
            import_instance,
            launch_instance,
            get_settings,
            save_settings,
            list_crash_reports,
            read_crash_report,
            install_java_runtime,
            get_account_type,
            get_install_directory
        ])
        .run(tauri::generate_context!());
    if let Err(error) = result {
        eprintln!("Lumen Launcher could not start: {error}");
    }
}
