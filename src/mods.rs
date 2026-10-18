use crate::{instances, loaders, models::InstanceRecord, storage, versions};
use anyhow::{bail, Context, Result};
use futures_util::stream::{self, StreamExt};
use serde::Deserialize;
use std::collections::HashSet;
use std::path::{Component, Path, PathBuf};
use tokio::io::AsyncWriteExt;
use zip::ZipArchive;

const API: &str = "https://api.modrinth.com/v2";
const USER_AGENT: &str = "LumenLauncher/0.1.0 (https://github.com/lumen-launcher)";

#[derive(Debug, Deserialize)]
struct SearchResponse {
    hits: Vec<ModrinthHit>,
}

#[derive(Clone, Debug, Deserialize)]
struct ModrinthHit {
    project_id: String,
    title: String,
    description: String,
    author: String,
    downloads: u64,
    icon_url: Option<String>,
    versions: Option<Vec<String>>,
}

#[derive(Clone, Debug, Deserialize)]
struct ModVersion {
    id: String,
    project_id: String,
    dependencies: Option<Vec<Dependency>>,
    files: Vec<ModFile>,
}

#[derive(Clone, Debug, Deserialize)]
struct Dependency {
    project_id: Option<String>,
    version_id: Option<String>,
    dependency_type: Option<String>,
}

#[derive(Clone, Debug, Deserialize)]
struct ModFile {
    filename: String,
    url: String,
    primary: Option<bool>,
}

#[derive(Debug, Deserialize)]
struct ModpackIndex {
    name: String,
    dependencies: std::collections::HashMap<String, String>,
    files: Vec<ModpackFile>,
}

#[derive(Debug, Deserialize)]
struct ModpackFile {
    path: String,
    downloads: Vec<String>,
    env: Option<serde_json::Value>,
}

pub async fn search(
    query: &str,
    game_version: Option<&str>,
    loader: Option<&str>,
) -> Result<Vec<crate::models::ModSearchResult>> {
    let client = reqwest::Client::new();
    let mut request = client
        .get(format!("{API}/search"))
        .header(reqwest::header::USER_AGENT, USER_AGENT)
        .query(&[("query", query.trim()), ("limit", "30")]);
    let mut facets = vec![vec!["project_type:mod".to_string()]];
    if let Some(version) = game_version.filter(|version| !version.is_empty()) {
        facets.push(vec![format!("versions:{version}")]);
    }
    if let Some(loader) = loader.filter(|loader| !loader.is_empty()) {
        facets.push(vec![format!("categories:{}", loader.to_ascii_lowercase())]);
    }
    request = request.query(&[("facets", serde_json::to_string(&facets)?)]);
    let response: SearchResponse = request
        .send()
        .await
        .context("Could not reach Modrinth")?
        .error_for_status()
        .context("Modrinth search failed")?
        .json()
        .await
        .context("Modrinth returned invalid search results")?;
    Ok(response
        .hits
        .into_iter()
        .map(|hit| crate::models::ModSearchResult {
            project_id: hit.project_id,
            title: hit.title,
            description: hit.description,
            author: hit.author,
            downloads: hit.downloads,
            icon_url: hit.icon_url,
            latest_version: hit
                .versions
                .and_then(|versions| versions.into_iter().next()),
        })
        .collect())
}

pub async fn install_project(
    root: &Path,
    instance: &InstanceRecord,
    project_id: &str,
) -> Result<Vec<String>> {
    if project_id.trim().is_empty()
        || !project_id
            .chars()
            .all(|char| char.is_ascii_alphanumeric() || char == '-' || char == '_')
    {
        bail!("Invalid Modrinth project identifier");
    }
    let mut pending = vec![Dependency {
        project_id: Some(project_id.to_string()),
        version_id: None,
        dependency_type: Some("required".to_string()),
    }];
    let mut installed = HashSet::new();
    let mut downloaded = Vec::new();
    while let Some(current) = pending.pop() {
        let version = if let Some(version_id) = current.version_id.as_deref() {
            get_exact_version(version_id).await?
        } else {
            let project_id = current
                .project_id
                .as_deref()
                .context("Required Modrinth dependency has no project or version id")?;
            get_project_version(
                project_id,
                &instance.game_version_id,
                instance.loader.as_deref(),
            )
            .await?
        };
        let identity = format!("{}:{}", version.project_id, version.id);
        if !installed.insert(identity) {
            continue;
        }
        let file = version
            .files
            .iter()
            .find(|file| file.primary.unwrap_or(false))
            .or_else(|| version.files.first())
            .context("Modrinth version has no downloadable file")?;
        let mods_dir = instances::instance_path(root, &instance.id)?
            .join("game")
            .join("mods");
        let destination = mods_dir.join(safe_filename(&file.filename)?);
        download(&file.url, &destination).await?;
        downloaded.push(file.filename.clone());
        for dependency in version.dependencies.unwrap_or_default() {
            if dependency.dependency_type.as_deref() != Some("required") {
                continue;
            }
            if dependency.version_id.is_some() || dependency.project_id.is_some() {
                pending.push(dependency);
            }
        }
    }
    Ok(downloaded)
}

async fn get_exact_version(version_id: &str) -> Result<ModVersion> {
    if version_id.is_empty()
        || !version_id
            .chars()
            .all(|char| char.is_ascii_alphanumeric() || char == '-' || char == '_')
    {
        bail!("Invalid Modrinth version identifier");
    }
    reqwest::Client::new()
        .get(format!("{API}/version/{version_id}"))
        .header(reqwest::header::USER_AGENT, USER_AGENT)
        .send()
        .await
        .context("Could not retrieve required Modrinth dependency version")?
        .error_for_status()
        .context("Modrinth required dependency version was not found")?
        .json()
        .await
        .context("Modrinth returned invalid dependency version data")
}

async fn get_project_version(
    project_id: &str,
    game_version: &str,
    loader: Option<&str>,
) -> Result<ModVersion> {
    let client = reqwest::Client::new();
    let mut request = client
        .get(format!("{API}/project/{project_id}/version"))
        .header(reqwest::header::USER_AGENT, USER_AGENT)
        .query(&[("game_versions", serde_json::to_string(&[game_version])?)]);
    if let Some(loader) = loader {
        request = request.query(&[("loaders", serde_json::to_string(&[loader])?)]);
    }
    let versions: Vec<ModVersion> = request
        .send()
        .await
        .context("Could not retrieve Modrinth project versions")?
        .error_for_status()
        .context("Modrinth version lookup failed")?
        .json()
        .await
        .context("Modrinth returned invalid version data")?;
    versions
        .into_iter()
        .next()
        .context("No compatible Modrinth version was found for this instance")
}

pub fn list_installed(root: &Path, instance: &InstanceRecord) -> Result<Vec<String>> {
    let mods_dir = instances::instance_path(root, &instance.id)?
        .join("game")
        .join("mods");
    if !mods_dir.exists() {
        return Ok(Vec::new());
    }
    let mut files = Vec::new();
    for entry in std::fs::read_dir(mods_dir)? {
        let entry = entry?;
        let path = entry.path();
        if path.is_file()
            && (path.extension().is_some_and(|ext| ext == "jar")
                || path.extension().is_some_and(|ext| ext == "disabled"))
        {
            files.push(entry.file_name().to_string_lossy().to_string());
        }
    }
    files.sort();
    Ok(files)
}

pub fn set_enabled(
    root: &Path,
    instance: &InstanceRecord,
    filename: &str,
    enabled: bool,
) -> Result<()> {
    let filename = safe_filename(filename)?;
    let mods_dir = instances::instance_path(root, &instance.id)?
        .join("game")
        .join("mods");
    let current = mods_dir.join(&filename);
    let target_name = if enabled {
        filename
            .strip_suffix(".jar.disabled")
            .map(|base| format!("{base}.jar"))
            .unwrap_or(filename.clone())
    } else if let Some(base) = filename.strip_suffix(".jar") {
        format!("{base}.jar.disabled")
    } else {
        bail!("Only .jar mods can be enabled or disabled");
    };
    let target = mods_dir.join(target_name);
    if !current.is_file() {
        bail!("Mod file was not found");
    }
    std::fs::rename(current, target)?;
    Ok(())
}

pub async fn import_mrpack(
    root: &Path,
    archive_path: &Path,
    name: Option<&str>,
) -> Result<InstanceRecord> {
    let archive_file =
        std::fs::File::open(archive_path).context("Could not open the modpack file")?;
    let mut archive =
        ZipArchive::new(archive_file).context("The selected modpack is not a valid ZIP archive")?;
    let mut index = None;
    for i in 0..archive.len() {
        let mut entry = archive.by_index(i)?;
        if entry.name() == "modrinth.index.json" {
            let mut bytes = Vec::new();
            std::io::Read::read_to_end(&mut entry, &mut bytes)?;
            index = Some(
                serde_json::from_slice::<ModpackIndex>(&bytes)
                    .context("The Modrinth pack index is invalid")?,
            );
            break;
        }
    }
    let index = index.context("This ZIP does not contain a Modrinth modpack index")?;
    let game_version = index
        .dependencies
        .get("minecraft")
        .context("Modpack index does not specify a Minecraft version")?
        .clone();
    versions::validate_id(&game_version)?;
    let mut instance = instances::create(root, name.unwrap_or(&index.name), &game_version)?;
    let loader = if index.dependencies.contains_key("fabric-loader") {
        Some("fabric")
    } else if index.dependencies.contains_key("quilt-loader") {
        Some("quilt")
    } else if index.dependencies.contains_key("neoforge") {
        Some("neoforge")
    } else if index.dependencies.contains_key("forge") {
        Some("forge")
    } else {
        None
    };
    instance.loader = loader.map(str::to_string);
    instance.loader_version = loader
        .and_then(|name| {
            index
                .dependencies
                .get(&format!("{name}-loader"))
                .or_else(|| index.dependencies.get(name))
        })
        .cloned();
    storage::save_instance(root, &instance)?;
    versions::install_version(root, &game_version).await?;
    if let Some(loader) = loader {
        let selected = if loader == "forge" {
            index.dependencies.get("forge")
        } else {
            index.dependencies.get(&format!("{loader}-loader"))
        };
        let _ = loaders::install(root, &mut instance, loader, selected.map(String::as_str)).await?;
        storage::save_instance(root, &instance)?;
    }
    let game_dir = instances::instance_path(root, &instance.id)?.join("game");
    let files: Vec<(String, String)> = index
        .files
        .into_iter()
        .filter(|file| {
            file.env
                .as_ref()
                .and_then(|env| env.get("client"))
                .and_then(serde_json::Value::as_str)
                != Some("unsupported")
        })
        .filter_map(|file| file.downloads.first().map(|url| (file.path, url.clone())))
        .collect();
    let unpack_source = std::fs::File::open(archive_path)?;
    let mut unpack_zip = ZipArchive::new(unpack_source)?;
    for i in 0..unpack_zip.len() {
        let mut entry = unpack_zip.by_index(i)?;
        let entry_name = entry.name().to_string();
        if let Some(relative) = entry_name.strip_prefix("overrides/") {
            if relative.is_empty() {
                continue;
            }
            let relative = safe_relative_path(relative)?;
            let destination = game_dir.join(relative);
            if entry.is_dir() {
                std::fs::create_dir_all(destination)?;
            } else {
                if let Some(parent) = destination.parent() {
                    std::fs::create_dir_all(parent)?;
                }
                let mut output = std::fs::File::create(destination)?;
                std::io::copy(&mut entry, &mut output)?;
            }
        }
    }
    stream::iter(files)
        .map(|(relative, url)| {
            let game_dir = game_dir.clone();
            async move {
                let relative = safe_relative_path(&relative)?;
                let path = game_dir.join(relative);
                download(&url, &path).await
            }
        })
        .buffer_unordered(8)
        .collect::<Vec<Result<()>>>()
        .await
        .into_iter()
        .collect::<Result<Vec<_>>>()?;
    Ok(instance)
}

async fn download(url: &str, destination: &Path) -> Result<()> {
    if let Some(parent) = destination.parent() {
        tokio::fs::create_dir_all(parent).await?;
    }
    if destination.exists() {
        return Ok(());
    }
    let bytes = reqwest::Client::new()
        .get(url)
        .header(reqwest::header::USER_AGENT, USER_AGENT)
        .send()
        .await
        .context("Could not download the selected mod file")?
        .error_for_status()
        .context("Mod download failed")?
        .bytes()
        .await?;
    let temporary = destination.with_extension("part");
    let mut output = tokio::fs::File::create(&temporary).await?;
    output.write_all(&bytes).await?;
    output.flush().await?;
    tokio::fs::rename(temporary, destination).await?;
    Ok(())
}

fn safe_filename(name: &str) -> Result<String> {
    if name.is_empty()
        || name.len() > 240
        || name == "."
        || name == ".."
        || name.contains('/')
        || name.contains('\\')
        || name.contains('\0')
    {
        bail!("Unsafe mod filename");
    }
    Ok(name.to_string())
}

fn safe_relative_path(value: &str) -> Result<PathBuf> {
    let path = Path::new(value);
    if path.is_absolute()
        || path
            .components()
            .any(|component| !matches!(component, Component::Normal(_)))
    {
        bail!("Modpack contains an unsafe file path");
    }
    Ok(path.to_path_buf())
}
