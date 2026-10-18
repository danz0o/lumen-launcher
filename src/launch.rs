use crate::{
    auth, instances, java,
    models::{AccountType, InstanceRecord},
    storage, versions,
};
use anyhow::{bail, Context, Result};
use serde_json::Value;
use std::path::{Path, PathBuf};
use tauri::{AppHandle, Emitter};
use tokio::io::{AsyncBufReadExt, BufReader};

pub async fn launch(app: &AppHandle, root: &Path, instance: &mut InstanceRecord) -> Result<u32> {
    let accounts = storage::load_accounts(root)?;
    let account = accounts
        .iter()
        .find(|account| account.active)
        .context("Choose an account before launching the game")?
        .clone();
    let mut tokens = storage::read_tokens(&account.id)?;
    if matches!(account.account_type, AccountType::Microsoft)
        && tokens.expires_at <= now_seconds() + 120
    {
        tokens = auth::refresh_account(&tokens).await?;
        storage::store_tokens(&account.id, &tokens)?;
    }
    let version = versions::load_installed_version(root, &instance.version_id).await?;
    let game_dir = instances::instance_path(root, &instance.id)?.join("game");
    tokio::fs::create_dir_all(&game_dir).await?;

    let major = java::required_major(&version);
    let java_path = if let Some(custom) = instance
        .java_path
        .as_ref()
        .filter(|value| !value.trim().is_empty())
    {
        PathBuf::from(custom)
    } else {
        let settings = storage::load_settings(root)?;
        if let Some(custom) = settings.java_path.filter(|path| !path.trim().is_empty()) {
            PathBuf::from(custom)
        } else {
            java::ensure_runtime(root, major).await?
        }
    };
    if !java_path.is_file() {
        bail!("Java executable does not exist: {}", java_path.display());
    }

    let natives = instances::instance_path(root, &instance.id)?.join("natives");
    tokio::fs::create_dir_all(&natives).await?;
    versions::extract_natives(root, &version, &natives)
        .context("Could not extract native game libraries")?;
    let client_jar = versions::client_jar_path(root, &instance.version_id, &version);
    if !client_jar.is_file() {
        bail!(
            "Game version {} is not installed yet. Install it before launching.",
            instance.version_id
        );
    }
    let mut classpath_entries = versions::classpath_libraries(root, &version)?;
    classpath_entries.push(client_jar);
    let classpath = classpath_entries
        .iter()
        .map(|path| path.to_string_lossy())
        .collect::<Vec<_>>()
        .join(if cfg!(target_os = "windows") {
            ";"
        } else {
            ":"
        });
    let asset_index = version
        .get("assetIndex")
        .and_then(|value| value.get("id"))
        .and_then(Value::as_str)
        .or_else(|| version.get("assets").and_then(Value::as_str))
        .unwrap_or("legacy");
    let uuid = account.uuid.replace('-', "");
    let game_dir_text = game_dir
        .to_str()
        .context("Game directory path is not valid UTF-8")?
        .to_string();
    let assets_root_text = root
        .join("assets")
        .to_str()
        .context("Assets path is not valid UTF-8")?
        .to_string();
    let natives_text = natives
        .to_str()
        .context("Native library path is not valid UTF-8")?
        .to_string();
    let libraries_text = root
        .join("libraries")
        .to_str()
        .context("Library path is not valid UTF-8")?
        .to_string();
    let vars = [
        ("auth_player_name", account.username.as_str()),
        ("version_name", instance.version_id.as_str()),
        ("game_directory", game_dir_text.as_str()),
        ("assets_root", assets_root_text.as_str()),
        ("assets_index_name", asset_index),
        ("auth_uuid", uuid.as_str()),
        ("auth_access_token", tokens.access_token.as_str()),
        ("user_type", tokens.user_type.as_str()),
        ("version_type", "release"),
        ("natives_directory", natives_text.as_str()),
        ("launcher_name", "LumenLauncher"),
        ("launcher_version", env!("CARGO_PKG_VERSION")),
        ("classpath", classpath.as_str()),
        ("library_directory", libraries_text.as_str()),
        (
            "classpath_separator",
            if cfg!(target_os = "windows") {
                ";"
            } else {
                ":"
            },
        ),
    ];

    let mut args = vec![
        "-Xms512M".to_string(),
        format!("-Xmx{}M", instance.ram_mb.clamp(1024, 65536)),
    ];
    args.extend(instance.jvm_args.iter().cloned());
    let metadata_jvm = version
        .get("arguments")
        .and_then(|arguments| arguments.get("jvm"));
    if let Some(values) = metadata_jvm {
        args.extend(resolve_argument_values(values, &vars)?);
    } else {
        args.push(format!("-Djava.library.path={}", natives.to_string_lossy()));
        args.push("-cp".to_string());
        args.push(classpath.clone());
    }
    let main_class = version
        .get("mainClass")
        .and_then(Value::as_str)
        .context("Version metadata does not contain a main class")?;
    args.push(main_class.to_string());

    let game_args = if let Some(arguments) = version
        .get("arguments")
        .and_then(|arguments| arguments.get("game"))
    {
        resolve_argument_values(arguments, &vars)?
    } else if let Some(legacy) = version.get("minecraftArguments").and_then(Value::as_str) {
        legacy.split_whitespace().map(str::to_owned).collect()
    } else {
        Vec::new()
    };
    args.extend(game_args);
    if let AccountType::Offline = account.account_type {
        if !args
            .iter()
            .any(|arg| arg == "--userType" || arg == "--user-type")
        {
            args.extend(["--userType".to_string(), "legacy".to_string()]);
        }
    }
    let settings = storage::load_settings(root)?;
    if !settings.server_address.trim().is_empty() {
        let (host, port) = parse_server_address(&settings.server_address)?;
        args.extend(["--server".to_string(), host]);
        if let Some(port) = port {
            args.extend(["--port".to_string(), port.to_string()]);
        }
    }

    let mut child = tokio::process::Command::new(&java_path)
        .args(args)
        .current_dir(&game_dir)
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::piped())
        .spawn()
        .with_context(|| format!("Could not start Java at {}", java_path.display()))?;
    let pid = child.id().unwrap_or_default();
    instance.last_played_at = Some(now_seconds());
    storage::save_instance(root, instance)?;

    if let Some(stdout) = child.stdout.take() {
        let app_handle = app.clone();
        let id = instance.id.clone();
        tokio::spawn(async move {
            emit_lines(app_handle, id, "stdout", stdout).await;
        });
    }
    if let Some(stderr) = child.stderr.take() {
        let app_handle = app.clone();
        let id = instance.id.clone();
        tokio::spawn(async move {
            emit_lines(app_handle, id, "stderr", stderr).await;
        });
    }
    let app_handle = app.clone();
    let id = instance.id.clone();
    tokio::spawn(async move {
        let status = child
            .wait()
            .await
            .map(|status| status.to_string())
            .unwrap_or_else(|error| error.to_string());
        let _ = app_handle.emit(
            "game-exit",
            serde_json::json!({"instanceId": id, "status": status}),
        );
    });
    Ok(pid)
}

fn resolve_argument_values(arguments: &Value, vars: &[(&str, &str)]) -> Result<Vec<String>> {
    let mut result = Vec::new();
    let items = arguments
        .as_array()
        .context("Minecraft argument list is invalid")?;
    for item in items {
        let (rules, value) = if item.is_object() {
            (
                item.get("rules"),
                item.get("value")
                    .context("Minecraft argument entry is missing its value")?,
            )
        } else {
            (None, item)
        };
        if let Some(rules) = rules.and_then(Value::as_array) {
            if !rules_allow_json(rules)? {
                continue;
            }
        }
        if let Some(value) = value.as_str() {
            result.push(replace_placeholders(value, vars));
        } else if let Some(values) = value.as_array() {
            for value in values {
                if let Some(value) = value.as_str() {
                    result.push(replace_placeholders(value, vars));
                }
            }
        }
    }
    Ok(result)
}

fn rules_allow_json(rules: &[Value]) -> Result<bool> {
    let rules: Vec<crate::versions::Rule> = serde_json::from_value(Value::Array(rules.to_vec()))?;
    // Argument and library rules use the same ordered allow/disallow semantics.
    Ok(crate::versions::rules_allow(
        &rules,
        current_os(),
        std::env::consts::ARCH,
        &std::collections::HashMap::new(),
    ))
}

fn current_os() -> &'static str {
    if cfg!(target_os = "windows") {
        "windows"
    } else if cfg!(target_os = "macos") {
        "osx"
    } else {
        "linux"
    }
}

fn replace_placeholders(value: &str, vars: &[(&str, &str)]) -> String {
    vars.iter()
        .fold(value.to_string(), |value, (key, replacement)| {
            value.replace(&format!("${{{key}}}"), replacement)
        })
}

fn parse_server_address(address: &str) -> Result<(String, Option<u16>)> {
    let address = address.trim();
    if address.is_empty() || address.contains(char::is_whitespace) {
        bail!("Enter a valid server address");
    }
    if let Some((host, port)) = address.rsplit_once(':') {
        if let Ok(port) = port.parse::<u16>() {
            if host.is_empty()
                || (host.contains(':') && !(host.starts_with('[') && host.ends_with(']')))
            {
                bail!("Enter a valid server address");
            }
            return Ok((host.to_string(), Some(port)));
        }
    }
    if address.contains(':') && !(address.starts_with('[') && address.ends_with(']')) {
        bail!("IPv6 server addresses must use brackets, for example [2001:db8::1]:25565");
    }
    Ok((address.to_string(), None))
}

async fn emit_lines<R>(app: AppHandle, instance_id: String, stream: &str, reader: R)
where
    R: tokio::io::AsyncRead + Unpin,
{
    let mut lines = BufReader::new(reader).lines();
    while let Ok(Some(line)) = lines.next_line().await {
        let _ = app.emit(
            "game-log",
            serde_json::json!({
                "instanceId": instance_id,
                "stream": stream,
                "line": line
            }),
        );
    }
}

pub fn crash_reports(root: &Path) -> Result<Vec<crate::models::CrashReport>> {
    let mut reports = Vec::new();
    for instance in storage::load_instances(root)? {
        let folder = instances::instance_path(root, &instance.id)?
            .join("game")
            .join("crash-reports");
        if !folder.exists() {
            continue;
        }
        for entry in std::fs::read_dir(folder)? {
            let entry = entry?;
            if !entry.file_type()?.is_file()
                || entry.path().extension().is_none_or(|ext| ext != "txt")
            {
                continue;
            }
            let metadata = entry.metadata()?;
            let bytes = std::fs::read(entry.path())?;
            let preview = String::from_utf8_lossy(&bytes[..bytes.len().min(220)]).to_string();
            reports.push(crate::models::CrashReport {
                instance_id: instance.id.clone(),
                filename: entry.file_name().to_string_lossy().to_string(),
                modified_at: metadata
                    .modified()
                    .ok()
                    .and_then(|time| time.duration_since(std::time::UNIX_EPOCH).ok())
                    .map(|duration| duration.as_secs())
                    .unwrap_or_default(),
                preview,
            });
        }
    }
    reports.sort_by(|left, right| right.modified_at.cmp(&left.modified_at));
    Ok(reports)
}

pub fn read_crash_report(root: &Path, instance_id: &str, filename: &str) -> Result<String> {
    if filename.is_empty()
        || filename.contains('/')
        || filename.contains('\\')
        || filename == "."
        || filename == ".."
    {
        bail!("Invalid crash report filename");
    }
    let path = instances::instance_path(root, instance_id)?
        .join("game")
        .join("crash-reports")
        .join(filename);
    let metadata = std::fs::metadata(&path).context("Crash report not found")?;
    if metadata.len() > 2 * 1024 * 1024 {
        bail!("Crash report is larger than 2 MB");
    }
    Ok(std::fs::read_to_string(path)?)
}

fn now_seconds() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs()
}

#[cfg(test)]
mod tests {
    use super::{parse_server_address, replace_placeholders};

    #[test]
    fn parses_server_address_and_port() {
        assert_eq!(
            parse_server_address("play.example.org:25566").unwrap(),
            ("play.example.org".to_string(), Some(25566))
        );
        assert_eq!(
            parse_server_address("play.example.org").unwrap(),
            ("play.example.org".to_string(), None)
        );
        assert_eq!(
            parse_server_address("[2001:db8::1]:25565").unwrap(),
            ("[2001:db8::1]".to_string(), Some(25565))
        );
        assert!(parse_server_address("2001:db8::1").is_err());
        assert!(parse_server_address("no server").is_err());
    }

    #[test]
    fn replaces_version_arguments() {
        assert_eq!(
            replace_placeholders(
                "--user ${auth_player_name}",
                &[("auth_player_name", "Lumen")]
            ),
            "--user Lumen"
        );
    }
}
