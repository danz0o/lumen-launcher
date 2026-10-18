use anyhow::{bail, Context, Result};
use flate2::read::GzDecoder;
use serde::Deserialize;
use std::path::{Path, PathBuf};
use tokio::io::AsyncWriteExt;
use zip::ZipArchive;

#[derive(Debug, Deserialize)]
struct AdoptiumBinary {
    binary: AdoptiumPackage,
}

#[derive(Debug, Deserialize)]
struct AdoptiumPackage {
    package: PackageLink,
}

#[derive(Debug, Deserialize)]
struct PackageLink {
    link: String,
}

pub fn required_major(version_json: &serde_json::Value) -> u8 {
    if let Some(major) = version_json
        .get("javaVersion")
        .and_then(|value| value.get("majorVersion"))
        .and_then(serde_json::Value::as_u64)
    {
        return major.min(255) as u8;
    }
    match version_json
        .get("id")
        .and_then(serde_json::Value::as_str)
        .unwrap_or_default()
    {
        id if id.starts_with("1.16")
            || id.starts_with("1.15")
            || id.starts_with("1.14")
            || id.starts_with("1.13")
            || id.starts_with("1.12") =>
        {
            8
        }
        id if id.starts_with("1.17") => 16,
        id if id.starts_with("1.18") || id.starts_with("1.19") || id.starts_with("1.20") => 17,
        _ => 21,
    }
}

pub async fn ensure_runtime(root: &Path, major: u8) -> Result<PathBuf> {
    let folder = root.join("runtime").join(format!("temurin-{major}"));
    let executable = java_executable(&folder);
    if executable.is_file() {
        return Ok(executable);
    }
    let os = if cfg!(target_os = "windows") {
        "windows"
    } else if cfg!(target_os = "macos") {
        "mac"
    } else {
        "linux"
    };
    let arch = if cfg!(target_arch = "aarch64") {
        "aarch64"
    } else if cfg!(target_arch = "x86") {
        "x32"
    } else {
        "x64"
    };
    let api = format!("https://api.adoptium.net/v3/assets/latest/{major}/hotspot?architecture={arch}&image_type=jdk&os={os}&vendor=eclipse");
    let response: Vec<AdoptiumBinary> = reqwest::get(api)
        .await
        .context("Could not find the Temurin Java runtime")?
        .error_for_status()?
        .json()
        .await
        .context("Temurin returned invalid runtime metadata")?;
    let link = response
        .first()
        .context("Temurin has no compatible runtime for this computer")?
        .binary
        .package
        .link
        .clone();
    let archive_path = root
        .join("runtime")
        .join(format!("temurin-{major}-download"));
    tokio::fs::create_dir_all(
        archive_path
            .parent()
            .context("Runtime directory is missing")?,
    )
    .await?;
    let response = reqwest::get(&link)
        .await
        .context("Could not download the Temurin runtime")?
        .error_for_status()
        .context("Temurin runtime download failed")?;
    let mut file = tokio::fs::File::create(&archive_path).await?;
    let bytes = response.bytes().await?;
    file.write_all(&bytes).await?;
    file.flush().await?;

    let extract_folder = folder.with_extension("extracting");
    tokio::fs::create_dir_all(&extract_folder).await?;
    let extension = if os == "windows" { "zip" } else { "tar.gz" };
    let extraction = extract_archive(&archive_path, &extract_folder, extension);
    let _ = tokio::fs::remove_file(&archive_path).await;
    extraction?;
    let extracted_root = find_java_home(&extract_folder)?;
    if folder.exists() {
        tokio::fs::remove_dir_all(&folder).await?;
    }
    tokio::fs::rename(&extracted_root, &folder)
        .await
        .with_context(|| format!("Could not place Java {major} in the runtime folder"))?;
    if extract_folder.exists() {
        let _ = tokio::fs::remove_dir_all(extract_folder).await;
    }
    let executable = java_executable(&folder);
    if !executable.is_file() {
        bail!("Temurin archive did not contain a Java executable");
    }
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let mut permissions = std::fs::metadata(&executable)?.permissions();
        permissions.set_mode(0o755);
        std::fs::set_permissions(&executable, permissions)?;
    }
    Ok(executable)
}

fn extract_archive(archive_path: &Path, destination: &Path, extension: &str) -> Result<()> {
    if extension == "zip" {
        let file = std::fs::File::open(archive_path)?;
        let mut archive =
            ZipArchive::new(file).context("Temurin archive is not a valid ZIP file")?;
        for index in 0..archive.len() {
            let mut entry = archive.by_index(index)?;
            let Some(relative) = entry.enclosed_name() else {
                bail!("Temurin archive contains an unsafe path");
            };
            let target = destination.join(relative);
            if entry.is_dir() {
                std::fs::create_dir_all(&target)?;
            } else {
                if let Some(parent) = target.parent() {
                    std::fs::create_dir_all(parent)?;
                }
                let mut output = std::fs::File::create(&target)?;
                std::io::copy(&mut entry, &mut output)?;
            }
        }
        return Ok(());
    }
    let file = std::fs::File::open(archive_path)?;
    let decoder = GzDecoder::new(file);
    let mut archive = tar::Archive::new(decoder);
    archive
        .unpack(destination)
        .context("Could not extract the Temurin runtime archive")?;
    Ok(())
}

fn find_java_home(root: &Path) -> Result<PathBuf> {
    let entries = std::fs::read_dir(root)?;
    for entry in entries {
        let path = entry?.path();
        if path.is_dir() && java_executable(&path).is_file() {
            return Ok(path);
        }
    }
    if java_executable(root).is_file() {
        return Ok(root.to_path_buf());
    }
    bail!("Could not locate the Java executable in the extracted runtime")
}

fn java_executable(folder: &Path) -> PathBuf {
    if cfg!(target_os = "windows") {
        folder.join("bin").join("java.exe")
    } else {
        folder.join("bin").join("java")
    }
}
