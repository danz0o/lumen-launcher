use crate::models::VersionSummary;
use anyhow::{bail, Context, Result};
use futures_util::stream::{self, StreamExt, TryStreamExt};
use serde::{Deserialize, Serialize};
use serde_json::{json, Map, Value};
use sha1::{Digest, Sha1};
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use tokio::io::AsyncWriteExt;

const MANIFEST_URL: &str = "https://piston-meta.mojang.com/mc/game/version_manifest_v2.json";

#[derive(Debug, Deserialize)]
struct VersionManifest {
    versions: Vec<ManifestVersion>,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
struct ManifestVersion {
    id: String,
    #[serde(rename = "type")]
    kind: String,
    url: String,
    release_time: String,
}

#[derive(Debug, Deserialize)]
struct DownloadInfo {
    url: String,
    sha1: Option<String>,
}

#[derive(Debug, Deserialize)]
struct Artifact {
    path: Option<String>,
    url: String,
    sha1: Option<String>,
}

#[derive(Debug, Deserialize)]
struct LibraryDownloads {
    artifact: Option<Artifact>,
    classifiers: Option<HashMap<String, Artifact>>,
}

#[derive(Debug, Deserialize)]
struct Library {
    name: String,
    #[serde(default)]
    url: Option<String>,
    #[serde(default)]
    rules: Option<Vec<Rule>>,
    #[serde(default)]
    downloads: Option<LibraryDownloads>,
    #[serde(default)]
    natives: Option<HashMap<String, String>>,
}

#[derive(Clone, Debug, Deserialize)]
pub(crate) struct Rule {
    pub(crate) action: String,
    #[serde(default)]
    pub(crate) os: Option<RuleOs>,
    #[serde(default)]
    pub(crate) features: Option<HashMap<String, bool>>,
}

#[derive(Clone, Debug, Deserialize)]
pub(crate) struct RuleOs {
    pub(crate) name: Option<String>,
    pub(crate) arch: Option<String>,
}

#[derive(Debug, Deserialize)]
struct AssetIndexInfo {
    id: String,
    url: String,
    sha1: Option<String>,
    #[serde(default)]
    virtual_assets: bool,
    #[serde(default)]
    map_to_resources: bool,
}

#[derive(Debug, Deserialize)]
struct VersionJson {
    #[serde(default)]
    downloads: HashMap<String, DownloadInfo>,
    #[serde(default)]
    libraries: Vec<Library>,
    #[serde(default)]
    asset_index: Option<AssetIndexInfo>,
}

#[derive(Debug, Deserialize)]
struct AssetIndex {
    objects: HashMap<String, AssetObject>,
}

#[derive(Debug, Deserialize)]
struct AssetObject {
    hash: String,
}

pub async fn list_versions() -> Result<Vec<VersionSummary>> {
    let manifest: VersionManifest = reqwest::get(MANIFEST_URL)
        .await
        .context("Could not reach the Minecraft version manifest")?
        .error_for_status()
        .context("Minecraft version manifest request failed")?
        .json()
        .await
        .context("Minecraft returned an invalid version manifest")?;
    Ok(manifest
        .versions
        .into_iter()
        .map(|version| VersionSummary {
            id: version.id,
            kind: version.kind,
            release_time: version.release_time,
        })
        .collect())
}

pub async fn install_version(root: &Path, version_id: &str) -> Result<()> {
    install_version_inner(None, root, version_id).await
}

pub async fn install_version_with_events(
    app: &tauri::AppHandle,
    root: &Path,
    version_id: &str,
) -> Result<()> {
    install_version_inner(Some(app.clone()), root, version_id).await
}

async fn install_version_inner(
    app: Option<tauri::AppHandle>,
    root: &Path,
    version_id: &str,
) -> Result<()> {
    emit_progress(&app, version_id, 4, "Fetching version metadata");
    validate_id(version_id)?;
    let client = reqwest::Client::new();
    let manifest: VersionManifest = client
        .get(MANIFEST_URL)
        .send()
        .await?
        .error_for_status()?
        .json()
        .await
        .context("Invalid version manifest")?;
    let selected = manifest
        .versions
        .iter()
        .find(|version| version.id == version_id)
        .context("Minecraft version is not listed in the current manifest")?;
    let raw: Value = client
        .get(&selected.url)
        .send()
        .await?
        .error_for_status()?
        .json()
        .await
        .context("Invalid Minecraft version metadata")?;
    let version_dir = root.join("versions").join(version_id);
    tokio::fs::create_dir_all(&version_dir).await?;
    let version_json_path = version_dir.join(format!("{version_id}.json"));
    let bytes = serde_json::to_vec_pretty(&raw)?;
    tokio::fs::write(&version_json_path, bytes).await?;
    emit_progress(&app, version_id, 16, "Downloading the game client");

    let document: VersionJson =
        serde_json::from_value(raw).context("Version metadata is missing required information")?;
    if let Some(client_jar) = document.downloads.get("client") {
        download_verified(
            &client,
            &client_jar.url,
            &version_dir.join(format!("{version_id}.jar")),
            client_jar.sha1.as_deref(),
        )
        .await?;
    }

    let mut library_downloads = Vec::new();
    let platform = current_os_name();
    for library in &document.libraries {
        if let Some(rules) = &library.rules {
            if !rules_allow(rules, platform, current_arch(), &HashMap::new()) {
                continue;
            }
        }
        let Some(downloads) = &library.downloads else {
            continue;
        };
        if let Some(artifact) = &downloads.artifact {
            let path = artifact
                .path
                .clone()
                .unwrap_or_else(|| maven_path(&library.name).unwrap_or_default());
            if !path.is_empty() {
                library_downloads.push((
                    artifact.url.clone(),
                    root.join("libraries").join(path),
                    artifact.sha1.clone(),
                ));
            }
        } else if let Some(base) = &library.url {
            if let Ok(path) = maven_path(&library.name) {
                library_downloads.push((
                    format!("{}{}", base.trim_end_matches('/'), format!("/{path}")),
                    root.join("libraries").join(path),
                    None,
                ));
            }
        }
        if let (Some(natives), Some(classifiers)) = (&library.natives, &downloads.classifiers) {
            if let Some(classifier_template) = natives.get(platform) {
                let classifier = classifier_template
                    .replace("${arch}", if current_arch() == "x86" { "32" } else { "64" });
                if let Some(native) = classifiers.get(&classifier) {
                    let path = native
                        .path
                        .clone()
                        .unwrap_or_else(|| maven_path(&library.name).unwrap_or_default());
                    if !path.is_empty() {
                        library_downloads.push((
                            native.url.clone(),
                            root.join("libraries").join(path),
                            native.sha1.clone(),
                        ));
                    }
                }
            }
        }
    }
    stream::iter(library_downloads)
        .map(|(url, path, sha1)| {
            let client = client.clone();
            async move { download_verified(&client, &url, &path, sha1.as_deref()).await }
        })
        .buffer_unordered(12)
        .try_collect::<Vec<_>>()
        .await?;
    emit_progress(&app, version_id, 48, "Downloading libraries");

    if let Some(asset_index) = &document.asset_index {
        install_assets(&client, root, asset_index).await?;
    }
    emit_progress(&app, version_id, 96, "Verifying assets");
    Ok(())
}

fn emit_progress(app: &Option<tauri::AppHandle>, version_id: &str, progress: u8, message: &str) {
    if let Some(app) = app {
        let _ = tauri::Emitter::emit(
            app,
            "install-progress",
            json!({
                "versionId": version_id,
                "progress": progress,
                "message": message
            }),
        );
    }
}

async fn install_assets(
    client: &reqwest::Client,
    root: &Path,
    asset_index_info: &AssetIndexInfo,
) -> Result<()> {
    validate_id(&asset_index_info.id)?;
    let index_path = root
        .join("assets")
        .join("indexes")
        .join(format!("{}.json", asset_index_info.id));
    download_verified(
        client,
        &asset_index_info.url,
        &index_path,
        asset_index_info.sha1.as_deref(),
    )
    .await?;
    let bytes = tokio::fs::read(&index_path).await?;
    let index: AssetIndex = serde_json::from_slice(&bytes).context("Asset index is invalid")?;
    let objects: Vec<(String, String)> = index
        .objects
        .iter()
        .map(|(name, asset)| (name.clone(), asset.hash.clone()))
        .collect();
    stream::iter(objects)
        .map(|(_name, hash)| {
            let client = client.clone();
            let root = root.to_path_buf();
            async move {
                if hash.len() < 2 || !hash.bytes().all(|byte| byte.is_ascii_hexdigit()) {
                    bail!("Minecraft asset index contains an invalid hash");
                }
                let path = root
                    .join("assets")
                    .join("objects")
                    .join(&hash[..2])
                    .join(&hash);
                let url = format!(
                    "https://resources.download.minecraft.net/{}/{hash}",
                    &hash[..2]
                );
                download_verified(&client, &url, &path, Some(&hash)).await?;
                Ok::<(), anyhow::Error>(())
            }
        })
        .buffer_unordered(16)
        .try_collect::<Vec<_>>()
        .await?;

    if asset_index_info.virtual_assets || asset_index_info.map_to_resources {
        let source_root = root.join("assets").join("objects");
        let virtual_root = if asset_index_info.map_to_resources {
            root.join("resources")
        } else {
            root.join("assets")
                .join("virtual")
                .join(&asset_index_info.id)
        };
        for (name, asset) in index.objects {
            let hash = asset.hash;
            if hash.len() < 2 {
                bail!("Minecraft asset index contains an invalid hash");
            }
            let source = source_root.join(&hash[..2]).join(&hash);
            let target = virtual_root.join(name);
            if let Some(parent) = target.parent() {
                tokio::fs::create_dir_all(parent).await?;
            }
            if !target.exists() {
                tokio::fs::copy(source, target).await?;
            }
        }
    }
    Ok(())
}

async fn download_verified(
    client: &reqwest::Client,
    url: &str,
    destination: &Path,
    expected_sha1: Option<&str>,
) -> Result<()> {
    if destination.is_file() {
        if let Some(expected) = expected_sha1 {
            let actual = sha1_file(destination).await?;
            if actual.eq_ignore_ascii_case(expected) {
                return Ok(());
            }
        } else {
            return Ok(());
        }
    }
    let parent = destination
        .parent()
        .context("Download target has no parent directory")?;
    tokio::fs::create_dir_all(parent).await?;
    let temporary = destination.with_extension("download");
    let mut last_error = None;
    for attempt in 0..3 {
        let result = async {
            let response = client
                .get(url)
                .send()
                .await?
                .error_for_status()
                .with_context(|| format!("Download failed: {url}"))?;
            let mut stream = response.bytes_stream();
            let mut file = tokio::fs::File::create(&temporary).await?;
            let mut digest = Sha1::new();
            while let Some(chunk) = stream.next().await {
                let chunk = chunk.context("Download was interrupted")?;
                digest.update(&chunk);
                file.write_all(&chunk).await?;
            }
            file.flush().await?;
            let actual = hex::encode(digest.finalize());
            if let Some(expected) = expected_sha1 {
                if !actual.eq_ignore_ascii_case(expected) {
                    bail!("SHA-1 mismatch for {url}: expected {expected}, received {actual}");
                }
            }
            tokio::fs::rename(&temporary, destination).await?;
            Ok::<(), anyhow::Error>(())
        }
        .await;
        match result {
            Ok(()) => return Ok(()),
            Err(error) => {
                last_error = Some(error);
                let _ = tokio::fs::remove_file(&temporary).await;
                if attempt < 2 {
                    tokio::time::sleep(std::time::Duration::from_millis(350 * (attempt + 1))).await;
                }
            }
        }
    }
    Err(last_error.unwrap_or_else(|| anyhow::anyhow!("Download failed")))
}

async fn sha1_file(path: &Path) -> Result<String> {
    let bytes = tokio::fs::read(path).await?;
    Ok(hex::encode(Sha1::digest(bytes)))
}

pub fn maven_path(coordinate: &str) -> Result<String> {
    let (coordinate, extension) = match coordinate.split_once('@') {
        Some((coordinate, extension)) => (coordinate, extension),
        None => (coordinate, "jar"),
    };
    let parts: Vec<&str> = coordinate.split(':').collect();
    if parts.len() < 3 || parts.len() > 4 || parts.iter().any(|part| part.is_empty()) {
        bail!("Invalid Maven coordinate: {coordinate}");
    }
    let group_path = parts[0].replace('.', "/");
    let artifact = parts[1];
    let version = parts[2];
    let classifier = parts
        .get(3)
        .map(|part| format!("-{part}"))
        .unwrap_or_default();
    Ok(format!(
        "{group_path}/{artifact}/{version}/{artifact}-{version}{classifier}.{extension}"
    ))
}

pub(crate) fn rules_allow(
    rules: &[Rule],
    os: &str,
    arch: &str,
    features: &HashMap<String, bool>,
) -> bool {
    if rules.is_empty() {
        return true;
    }
    let mut allowed = false;
    for rule in rules {
        let os_matches = rule
            .os
            .as_ref()
            .map(|rule_os| {
                rule_os.name.as_deref().is_none_or(|name| name == os)
                    && rule_os
                        .arch
                        .as_deref()
                        .is_none_or(|pattern| arch_matches(pattern, arch))
            })
            .unwrap_or(true);
        let features_match = rule
            .features
            .as_ref()
            .map(|expected| {
                expected
                    .iter()
                    .all(|(key, value)| features.get(key).copied().unwrap_or(false) == *value)
            })
            .unwrap_or(true);
        if os_matches && features_match {
            allowed = rule.action == "allow";
        }
    }
    allowed
}

fn arch_matches(pattern: &str, arch: &str) -> bool {
    match pattern {
        "x86" => arch == "x86" || arch == "i386" || arch == "i686",
        "x86_64" => arch == "x86_64" || arch == "amd64",
        other => other == arch,
    }
}

fn current_os_name() -> &'static str {
    if cfg!(target_os = "windows") {
        "windows"
    } else if cfg!(target_os = "macos") {
        "osx"
    } else {
        "linux"
    }
}

fn current_arch() -> &'static str {
    if cfg!(target_arch = "x86") {
        "x86"
    } else if cfg!(target_arch = "x86_64") {
        "x86_64"
    } else if cfg!(target_arch = "aarch64") {
        "aarch64"
    } else {
        std::env::consts::ARCH
    }
}

pub fn merge_version_json(parent: Value, child: Value) -> Value {
    let mut merged = parent.as_object().cloned().unwrap_or_default();
    let child_object = child.as_object().cloned().unwrap_or_default();
    for (key, child_value) in &child_object {
        if key == "libraries" {
            let mut libraries = Vec::new();
            let mut positions = HashMap::new();
            for library in merged
                .get(key)
                .and_then(Value::as_array)
                .into_iter()
                .flatten()
                .chain(child_value.as_array().into_iter().flatten())
            {
                let name = library
                    .get("name")
                    .and_then(Value::as_str)
                    .unwrap_or_default();
                if let Some(index) = positions.get(name).copied() {
                    libraries[index] = library.clone();
                } else {
                    positions.insert(name.to_string(), libraries.len());
                    libraries.push(library.clone());
                }
            }
            merged.insert(key.clone(), Value::Array(libraries));
        } else if key == "arguments" {
            let parent_args = merged.get(key).and_then(Value::as_object);
            let child_args = child_value.as_object();
            let mut args = Map::new();
            for argument_type in ["game", "jvm"] {
                let mut values = Vec::new();
                if let Some(parent_values) = parent_args
                    .and_then(|value| value.get(argument_type))
                    .and_then(Value::as_array)
                {
                    values.extend(parent_values.iter().cloned());
                }
                if let Some(child_values) = child_args
                    .and_then(|value| value.get(argument_type))
                    .and_then(Value::as_array)
                {
                    values.extend(child_values.iter().cloned());
                }
                if !values.is_empty() {
                    args.insert(argument_type.to_string(), Value::Array(values));
                }
            }
            if let Some(extra_args) = child_args {
                for (name, value) in extra_args {
                    if name != "game" && name != "jvm" {
                        args.insert(name.clone(), value.clone());
                    }
                }
            }
            merged.insert(key.clone(), Value::Object(args));
        } else {
            merged.insert(key.clone(), child_value.clone());
        }
    }
    merged.remove("inheritsFrom");
    Value::Object(merged)
}

pub fn merged_loader_version(parent: Value, child: Value) -> Value {
    merge_version_json(parent, child)
}

pub fn validate_id(id: &str) -> Result<()> {
    if id.is_empty()
        || id.len() > 128
        || !id
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'.' | b'_' | b'-'))
    {
        bail!("Invalid Minecraft version identifier");
    }
    Ok(())
}

pub async fn load_installed_version(root: &Path, id: &str) -> Result<Value> {
    validate_id(id)?;
    let path = root.join("versions").join(id).join(format!("{id}.json"));
    let bytes = tokio::fs::read(&path)
        .await
        .with_context(|| format!("Version {id} has not been installed"))?;
    Ok(serde_json::from_slice(&bytes)?)
}

pub async fn install_profile_json(root: &Path, profile: Value) -> Result<String> {
    let parent_id = profile
        .get("inheritsFrom")
        .and_then(Value::as_str)
        .map(str::to_owned);
    let profile_id = profile
        .get("id")
        .and_then(Value::as_str)
        .context("Loader profile has no version identifier")?
        .to_string();
    validate_id(&profile_id)?;
    let merged = if let Some(parent_id) = parent_id {
        let parent = load_installed_version(root, &parent_id).await?;
        let jar_id = parent
            .get("jar")
            .and_then(Value::as_str)
            .or_else(|| parent.get("id").and_then(Value::as_str))
            .unwrap_or(&parent_id)
            .to_string();
        let mut merged = merged_loader_version(parent, profile);
        if let Some(object) = merged.as_object_mut() {
            object
                .entry("jar".to_string())
                .or_insert(Value::String(jar_id));
        }
        merged
    } else {
        profile
    };
    let bytes = serde_json::to_vec_pretty(&merged)?;
    let version_dir = root.join("versions").join(&profile_id);
    tokio::fs::create_dir_all(&version_dir).await?;
    tokio::fs::write(version_dir.join(format!("{profile_id}.json")), bytes).await?;
    let libraries: Vec<Library> = serde_json::from_value(
        merged
            .get("libraries")
            .cloned()
            .unwrap_or_else(|| json!([])),
    )?;
    let client = reqwest::Client::new();
    let mut downloads = Vec::new();
    for library in libraries {
        if let Some(rules) = &library.rules {
            if !rules_allow(rules, current_os_name(), current_arch(), &HashMap::new()) {
                continue;
            }
        }
        if let Some(artifact) = library.downloads.and_then(|downloads| downloads.artifact) {
            let path = artifact.path.unwrap_or(maven_path(&library.name)?);
            downloads.push((
                artifact.url,
                root.join("libraries").join(path),
                artifact.sha1,
            ));
        } else if let Some(base) = library.url {
            let path = maven_path(&library.name)?;
            downloads.push((
                format!("{}/{path}", base.trim_end_matches('/')),
                root.join("libraries").join(path),
                None,
            ));
        }
    }
    stream::iter(downloads)
        .map(|(url, path, sha1)| {
            let client = client.clone();
            async move { download_verified(&client, &url, &path, sha1.as_deref()).await }
        })
        .buffer_unordered(12)
        .try_collect::<Vec<_>>()
        .await?;
    Ok(profile_id)
}

pub fn client_jar_path(root: &Path, version_id: &str, version: &Value) -> PathBuf {
    let jar_id = version
        .get("jar")
        .and_then(Value::as_str)
        .unwrap_or(version_id);
    root.join("versions")
        .join(jar_id)
        .join(format!("{jar_id}.jar"))
}

pub fn extract_natives(root: &Path, version: &Value, destination: &Path) -> Result<()> {
    use std::io::Write;
    use zip::ZipArchive;

    std::fs::create_dir_all(destination)?;
    let os = if cfg!(target_os = "windows") {
        "windows"
    } else if cfg!(target_os = "macos") {
        "osx"
    } else {
        "linux"
    };
    let arch = if cfg!(target_arch = "x86") {
        "x86"
    } else {
        current_arch()
    };
    let libraries = version
        .get("libraries")
        .and_then(Value::as_array)
        .context("Version metadata has no library list")?;
    for library in libraries {
        let Some(natives) = library.get("natives") else {
            continue;
        };
        let Some(template) = natives.get(os).and_then(Value::as_str) else {
            continue;
        };
        if let Some(rules) = library.get("rules").and_then(Value::as_array) {
            let rules: Vec<Rule> = serde_json::from_value(Value::Array(rules.clone()))?;
            if !rules_allow(&rules, os, arch, &HashMap::new()) {
                continue;
            }
        }
        let classifier = template.replace("${arch}", if arch == "x86" { "32" } else { "64" });
        let Some(path) = library
            .get("downloads")
            .and_then(|downloads| downloads.get("classifiers"))
            .and_then(|classifiers| classifiers.get(&classifier))
            .and_then(|artifact| artifact.get("path"))
            .and_then(Value::as_str)
        else {
            continue;
        };
        let archive_path = root.join("libraries").join(path);
        if !archive_path.is_file() {
            continue;
        }
        let excluded: Vec<String> = library
            .get("extract")
            .and_then(|extract| extract.get("exclude"))
            .and_then(Value::as_array)
            .into_iter()
            .flatten()
            .filter_map(Value::as_str)
            .map(str::to_owned)
            .collect();
        let file = std::fs::File::open(archive_path)?;
        let mut archive = ZipArchive::new(file).context("Native library archive is invalid")?;
        for index in 0..archive.len() {
            let mut entry = archive.by_index(index)?;
            let Some(relative) = entry.enclosed_name() else {
                continue;
            };
            let relative_name = relative.to_string_lossy().replace('\\', "/");
            if relative_name.starts_with("META-INF/")
                || excluded
                    .iter()
                    .any(|prefix| relative_name.starts_with(prefix))
            {
                continue;
            }
            let target = destination.join(relative);
            if let Some(parent) = target.parent() {
                std::fs::create_dir_all(parent)?;
            }
            let mut output = std::fs::File::create(target)?;
            std::io::copy(&mut entry, &mut output)?;
            output.flush()?;
        }
    }
    Ok(())
}

pub fn classpath_libraries(root: &Path, version: &Value) -> Result<Vec<PathBuf>> {
    let libraries = version
        .get("libraries")
        .and_then(Value::as_array)
        .context("Version metadata has no library list")?;
    let mut result = Vec::new();
    for library in libraries {
        let name = library
            .get("name")
            .and_then(Value::as_str)
            .unwrap_or_default();
        let rules = library.get("rules").and_then(Value::as_array);
        if let Some(rules) = rules {
            let rules: Vec<Rule> = serde_json::from_value(Value::Array(rules.clone()))?;
            if !rules_allow(&rules, current_os_name(), current_arch(), &HashMap::new()) {
                continue;
            }
        }
        let path = library
            .get("downloads")
            .and_then(|downloads| downloads.get("artifact"))
            .and_then(|artifact| artifact.get("path"))
            .and_then(Value::as_str)
            .map(str::to_owned)
            .or_else(|| maven_path(name).ok());
        if let Some(path) = path {
            result.push(root.join("libraries").join(path));
        }
    }
    Ok(result)
}

#[cfg(test)]
mod tests {
    use super::{maven_path, merge_version_json, rules_allow, Rule};
    use serde_json::json;
    use std::collections::HashMap;

    #[test]
    fn parses_maven_paths_with_classifier_and_extension() {
        assert_eq!(
            maven_path("org.lwjgl:lwjgl:3.3.3:natives-linux@jar").unwrap(),
            "org/lwjgl/lwjgl/3.3.3/lwjgl-3.3.3-natives-linux.jar"
        );
        assert_eq!(
            maven_path("com.example:library:1.2.0@pom").unwrap(),
            "com/example/library/1.2.0/library-1.2.0.pom"
        );
        assert!(maven_path("not-a-coordinate").is_err());
    }

    #[test]
    fn rules_follow_last_matching_action_and_platform() {
        let rules: Vec<Rule> = serde_json::from_value(json!([
            { "action": "allow" },
            { "action": "disallow", "os": { "name": "windows" } }
        ]))
        .unwrap();
        let features = HashMap::new();
        assert!(rules_allow(&rules, "linux", "x86_64", &features));
        assert!(!rules_allow(&rules, "windows", "x86_64", &features));
    }

    #[test]
    fn merges_parent_and_child_libraries_without_duplicates() {
        let merged = merge_version_json(
            json!({"id":"base","libraries":[{"name":"a"},{"name":"b"}],"arguments":{"game":["base"],"jvm":["-Xms1G"]}}),
            json!({"id":"loader","inheritsFrom":"base","libraries":[{"name":"b","url":"child"},{"name":"c"}],"arguments":{"game":["loader"]}}),
        );
        assert_eq!(merged["id"], "loader");
        assert!(merged.get("inheritsFrom").is_none());
        assert_eq!(merged["libraries"].as_array().unwrap().len(), 3);
        assert_eq!(merged["libraries"][1]["url"], "child");
        assert_eq!(merged["arguments"]["game"], json!(["base", "loader"]));
    }
}
