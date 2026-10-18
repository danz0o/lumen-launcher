use crate::models::InstanceRecord;
use crate::storage;
use anyhow::{bail, Context, Result};
use std::fs::{self, File};
use std::io::{Read, Write};
use std::path::{Path, PathBuf};
use uuid::Uuid;
use walkdir::WalkDir;
use zip::{write::SimpleFileOptions, ZipArchive, ZipWriter};

pub fn instance_path(root: &Path, instance_id: &str) -> Result<PathBuf> {
    if instance_id.is_empty()
        || !instance_id
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || byte == b'-')
    {
        bail!("Invalid instance identifier");
    }
    Ok(root.join("instances").join(instance_id))
}

pub fn create(root: &Path, name: &str, version_id: &str) -> Result<InstanceRecord> {
    let clean_name = name.trim();
    if clean_name.is_empty() || clean_name.chars().count() > 48 {
        bail!("Instance name must be between 1 and 48 characters");
    }
    crate::versions::validate_id(version_id)?;
    let now = now_seconds();
    let instance = InstanceRecord {
        id: Uuid::new_v4().simple().to_string(),
        name: clean_name.to_string(),
        version_id: version_id.to_string(),
        game_version_id: version_id.to_string(),
        loader: None,
        loader_version: None,
        ram_mb: 4096,
        java_path: None,
        jvm_args: Vec::new(),
        created_at: now,
        last_played_at: None,
    };
    let folder = instance_path(root, &instance.id)?;
    fs::create_dir_all(folder.join("game"))?;
    storage::save_instance(root, &instance)?;
    Ok(instance)
}

pub fn clone_instance(root: &Path, id: &str, name: &str) -> Result<InstanceRecord> {
    let source = instance_path(root, id)?;
    let original = load_instance(root, id)?;
    let mut cloned = original.clone();
    let clean_name = name.trim();
    if clean_name.is_empty() || clean_name.chars().count() > 48 {
        bail!("Instance name must be between 1 and 48 characters");
    }
    cloned.id = Uuid::new_v4().simple().to_string();
    cloned.name = clean_name.to_string();
    cloned.created_at = now_seconds();
    cloned.last_played_at = None;
    let destination = instance_path(root, &cloned.id)?;
    copy_tree(&source, &destination)?;
    storage::save_instance(root, &cloned)?;
    Ok(cloned)
}

pub fn load_instance(root: &Path, id: &str) -> Result<InstanceRecord> {
    storage::load_instances(root)?
        .into_iter()
        .find(|instance| instance.id == id)
        .context("Instance not found")
}

pub fn delete(root: &Path, id: &str) -> Result<()> {
    storage::delete_instance(root, id)
}

pub fn export_zip(root: &Path, id: &str, destination: &Path) -> Result<()> {
    let source = instance_path(root, id)?;
    if !source.is_dir() {
        bail!("Instance folder does not exist");
    }
    if let Some(parent) = destination.parent() {
        fs::create_dir_all(parent)?;
    }
    let file = File::create(destination)
        .with_context(|| format!("Could not create {}", destination.display()))?;
    let mut archive = ZipWriter::new(file);
    let options = SimpleFileOptions::default().compression_method(zip::CompressionMethod::Deflated);
    for entry in WalkDir::new(&source).follow_links(false) {
        let entry = entry?;
        let path = entry.path();
        let relative = path.strip_prefix(&source)?;
        if relative.as_os_str().is_empty() {
            continue;
        }
        let archive_name = relative.to_string_lossy().replace('\\', "/");
        if entry.file_type().is_dir() {
            archive.add_directory(format!("{archive_name}/"), options)?;
        } else if entry.file_type().is_file() {
            archive.start_file(archive_name, options)?;
            let mut input = File::open(path)?;
            std::io::copy(&mut input, &mut archive)?;
        }
    }
    archive.finish()?;
    Ok(())
}

pub fn import_zip(
    root: &Path,
    source: &Path,
    name: &str,
    fallback_version: &str,
) -> Result<InstanceRecord> {
    let clean_name = name.trim();
    if clean_name.is_empty() || clean_name.chars().count() > 48 {
        bail!("Instance name must be between 1 and 48 characters");
    }
    crate::versions::validate_id(fallback_version)?;
    let id = Uuid::new_v4().simple().to_string();
    let destination = instance_path(root, &id)?;
    fs::create_dir_all(&destination)?;
    let file =
        File::open(source).with_context(|| format!("Could not open {}", source.display()))?;
    let mut archive =
        ZipArchive::new(file).context("The selected file is not a valid ZIP archive")?;
    let mut imported_manifest = None;
    for index in 0..archive.len() {
        let mut entry = archive.by_index(index)?;
        let Some(relative) = entry.enclosed_name() else {
            bail!("The archive contains an unsafe path");
        };
        let target = destination.join(relative);
        if entry.is_dir() {
            fs::create_dir_all(&target)?;
            continue;
        }
        let target = if target.starts_with(&destination) {
            target
        } else {
            bail!("The archive contains an unsafe path");
        };
        if let Some(parent) = target.parent() {
            fs::create_dir_all(parent)?;
        }
        let mut output = File::create(&target)?;
        if target
            .file_name()
            .is_some_and(|name| name == "lumen-instance.json")
        {
            let mut bytes = Vec::new();
            entry.read_to_end(&mut bytes)?;
            output.write_all(&bytes)?;
            imported_manifest = serde_json::from_slice::<InstanceRecord>(&bytes).ok();
        } else {
            std::io::copy(&mut entry, &mut output)?;
        }
    }
    let mut record = imported_manifest.unwrap_or(InstanceRecord {
        id: id.clone(),
        name: clean_name.to_string(),
        version_id: fallback_version.to_string(),
        game_version_id: fallback_version.to_string(),
        loader: None,
        loader_version: None,
        ram_mb: 4096,
        java_path: None,
        jvm_args: Vec::new(),
        created_at: now_seconds(),
        last_played_at: None,
    });
    record.id = id;
    record.name = clean_name.to_string();
    record.created_at = now_seconds();
    storage::save_instance(root, &record)?;
    Ok(record)
}

fn copy_tree(source: &Path, destination: &Path) -> Result<()> {
    fs::create_dir_all(destination)?;
    for entry in WalkDir::new(source).follow_links(false) {
        let entry = entry?;
        let relative = entry.path().strip_prefix(source)?;
        if relative.as_os_str().is_empty() {
            continue;
        }
        let target = destination.join(relative);
        if entry.file_type().is_dir() {
            fs::create_dir_all(&target)?;
        } else if entry.file_type().is_file() {
            if let Some(parent) = target.parent() {
                fs::create_dir_all(parent)?;
            }
            fs::copy(entry.path(), target)?;
        }
    }
    Ok(())
}

fn now_seconds() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs()
}
