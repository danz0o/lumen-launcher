use crate::{java, models::InstanceRecord, versions};
use anyhow::{bail, Context, Result};
use std::path::Path;
use tokio::io::AsyncWriteExt;

pub async fn install(
    root: &Path,
    instance: &mut InstanceRecord,
    loader: &str,
    loader_version: Option<&str>,
) -> Result<String> {
    let normalized = loader.to_ascii_lowercase();
    if !matches!(
        normalized.as_str(),
        "fabric" | "quilt" | "forge" | "neoforge"
    ) {
        bail!("Unsupported mod loader: {loader}");
    }
    if !root
        .join("versions")
        .join(&instance.game_version_id)
        .join(format!("{}.json", instance.game_version_id))
        .is_file()
    {
        versions::install_version(root, &instance.game_version_id).await?;
    }
    let installed_id = match normalized.as_str() {
        "fabric" => install_fabric(root, &instance.game_version_id, loader_version).await?,
        "quilt" => install_quilt(root, &instance.game_version_id, loader_version).await?,
        "forge" | "neoforge" => {
            install_forge_family(root, instance, &normalized, loader_version).await?
        }
        _ => unreachable!(),
    };
    instance.loader = Some(normalized);
    instance.loader_version = loader_version.map(str::to_owned);
    instance.version_id = installed_id.clone();
    Ok(installed_id)
}

async fn install_fabric(
    root: &Path,
    game_version: &str,
    requested: Option<&str>,
) -> Result<String> {
    let client = reqwest::Client::new();
    let versions: Vec<serde_json::Value> = client
        .get(format!(
            "https://meta.fabricmc.net/v2/versions/loader/{game_version}"
        ))
        .send()
        .await
        .context("Could not reach Fabric metadata")?
        .error_for_status()
        .context("Fabric has no compatible loader for this Minecraft version")?
        .json()
        .await
        .context("Fabric returned invalid loader metadata")?;
    let loader_version = versions
        .iter()
        .filter_map(|entry| {
            entry
                .get("loader")
                .and_then(|loader| loader.get("version"))
                .and_then(serde_json::Value::as_str)
        })
        .find(|version| requested.is_none_or(|selected| *version == selected))
        .context("Requested Fabric loader version is unavailable")?;
    let profile: serde_json::Value = client
        .get(format!("https://meta.fabricmc.net/v2/versions/loader/{game_version}/{loader_version}/profile/json"))
        .send().await?.error_for_status()?.json().await.context("Fabric returned an invalid launch profile")?;
    versions::install_profile_json(root, profile).await
}

async fn install_quilt(root: &Path, game_version: &str, requested: Option<&str>) -> Result<String> {
    let client = reqwest::Client::new();
    let versions: Vec<serde_json::Value> = client
        .get(format!(
            "https://meta.quiltmc.org/v3/versions/loader/{game_version}"
        ))
        .send()
        .await
        .context("Could not reach Quilt metadata")?
        .error_for_status()
        .context("Quilt has no compatible loader for this Minecraft version")?
        .json()
        .await
        .context("Quilt returned invalid loader metadata")?;
    let loader_version = versions
        .iter()
        .filter_map(|entry| {
            entry
                .get("loader")
                .and_then(|loader| loader.get("version"))
                .and_then(serde_json::Value::as_str)
        })
        .find(|version| requested.is_none_or(|selected| *version == selected))
        .context("Requested Quilt loader version is unavailable")?;
    let profile: serde_json::Value = client
        .get(format!("https://meta.quiltmc.org/v3/versions/loader/{game_version}/{loader_version}/profile/json"))
        .send().await?.error_for_status()?.json().await.context("Quilt returned an invalid launch profile")?;
    versions::install_profile_json(root, profile).await
}

async fn install_forge_family(
    root: &Path,
    instance: &InstanceRecord,
    loader: &str,
    requested: Option<&str>,
) -> Result<String> {
    let installer_url = if loader == "forge" {
        let forge_version = match requested {
            Some(version) => version.to_string(),
            None => {
                let promotions: serde_json::Value = reqwest::get("https://files.minecraftforge.net/net/minecraftforge/forge/promotions_slim.json")
                    .await.context("Could not read Forge version metadata")?.error_for_status()?.json().await?;
                let key = format!("{}-recommended", instance.game_version_id);
                let build = promotions
                    .get("promos")
                    .and_then(|promos| promos.get(&key))
                    .and_then(serde_json::Value::as_str)
                    .or_else(|| {
                        promotions
                            .get("promos")
                            .and_then(|promos| {
                                promos.get(format!("{}-latest", instance.game_version_id))
                            })
                            .and_then(serde_json::Value::as_str)
                    })
                    .context(
                        "Forge has no recommended or latest build for this Minecraft version",
                    )?;
                build.to_string()
            }
        };
        let full_version = if forge_version.starts_with(&format!("{}-", instance.game_version_id)) {
            forge_version
        } else {
            format!("{}-{forge_version}", instance.game_version_id)
        };
        let url = format!("https://maven.minecraftforge.net/net/minecraftforge/forge/{full_version}/forge-{full_version}-installer.jar");
        url
    } else {
        let neoforge_version = requested.context("Choose a NeoForge version to install")?;
        let url = format!("https://maven.neoforged.net/releases/net/neoforged/neoforge/{neoforge_version}/neoforge-{neoforge_version}-installer.jar");
        url
    };
    let installer = root
        .join("installers")
        .join(format!("{loader}-installer.jar"));
    tokio::fs::create_dir_all(installer.parent().context("Installer folder is missing")?).await?;
    let bytes = reqwest::get(&installer_url)
        .await
        .context("Could not download the mod-loader installer")?
        .error_for_status()
        .context("Mod-loader installer download failed")?
        .bytes()
        .await?;
    let mut file = tokio::fs::File::create(&installer).await?;
    file.write_all(&bytes).await?;
    file.flush().await?;

    let java_major = if instance.game_version_id.starts_with("1.16")
        || instance.game_version_id.starts_with("1.15")
    {
        8
    } else if instance.game_version_id.starts_with("1.17") {
        17
    } else {
        21
    };
    let java_path = java::ensure_runtime(root, java_major).await?;
    tokio::fs::create_dir_all(root).await?;
    let output = tokio::process::Command::new(java_path)
        .arg("-jar")
        .arg(&installer)
        .arg("--installClient")
        .arg(root)
        .output()
        .await
        .context("Could not start the mod-loader installer")?;
    if !output.status.success() {
        bail!(
            "Mod-loader installer failed: {}",
            String::from_utf8_lossy(&output.stderr)
        );
    }
    let versions_dir = root.join("versions");
    let mut candidates = Vec::new();
    for entry in
        std::fs::read_dir(&versions_dir).context("Mod-loader did not create a versions folder")?
    {
        let entry = entry?;
        let path = entry
            .path()
            .join(format!("{}.json", entry.file_name().to_string_lossy()));
        if !path.is_file() {
            continue;
        }
        let bytes = std::fs::read(&path)?;
        let Ok(profile) = serde_json::from_slice::<serde_json::Value>(&bytes) else {
            continue;
        };
        let inherits = profile
            .get("inheritsFrom")
            .and_then(serde_json::Value::as_str);
        let id = profile
            .get("id")
            .and_then(serde_json::Value::as_str)
            .unwrap_or_default();
        if inherits == Some(instance.game_version_id.as_str())
            && id.to_ascii_lowercase().contains(loader)
        {
            candidates.push(id.to_string());
        }
    }
    candidates.sort();
    candidates
        .pop()
        .context("Installer finished, but did not create a compatible launch profile")
}
