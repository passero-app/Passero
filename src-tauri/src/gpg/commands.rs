use crate::error::{PasseroError, Result};
use serde::{Deserialize, Serialize};
use tauri_plugin_store::StoreExt;
use tokio::process::Command;

#[derive(Debug, Clone, Serialize)]
pub struct GpgKey {
    pub id: String,
    pub fingerprint: String,
    pub uid: String,
    pub trust: String,
}

#[derive(Debug, Clone, Deserialize)]
pub struct GenerateKeyParams {
    pub name: String,
    pub email: String,
    pub passphrase: Option<String>,
    pub key_type: Option<String>, // e.g., "RSA", "ECC"
    pub key_length: Option<u32>,  // e.g., 4096
}

fn gpg_binary(app: &tauri::AppHandle) -> Result<String> {
    let store = app
        .store("config.json")
        .map_err(|e: tauri_plugin_store::Error| PasseroError::ConfigError(e.to_string()))?;

    Ok(store
        .get("gpg_binary")
        .and_then(|v: serde_json::Value| v.as_str().map(String::from))
        .unwrap_or_else(|| "gpg".to_string()))
}

fn gpg_command(binary: &str) -> Command {
    let mut cmd = Command::new(binary);
    cmd.env("PATH", crate::path::augmented_path());
    cmd
}

#[tauri::command]
pub async fn list_gpg_keys(app: tauri::AppHandle) -> Result<Vec<GpgKey>> {
    let binary = gpg_binary(&app)?;

    let output = gpg_command(&binary)
        .args(["--list-keys", "--with-colons", "--batch", "--no-tty"])
        .output()
        .await?;

    if !output.status.success() {
        return Err(PasseroError::GpgError(
            String::from_utf8_lossy(&output.stderr).to_string(),
        ));
    }

    Ok(parse_key_listing(&String::from_utf8_lossy(&output.stdout)))
}

#[tauri::command]
pub async fn list_gpg_secret_keys(app: tauri::AppHandle) -> Result<Vec<GpgKey>> {
    let binary = gpg_binary(&app)?;

    let output = gpg_command(&binary)
        .args(["--list-secret-keys", "--with-colons", "--batch", "--no-tty"])
        .output()
        .await?;

    if !output.status.success() {
        return Err(PasseroError::GpgError(
            String::from_utf8_lossy(&output.stderr).to_string(),
        ));
    }

    Ok(parse_key_listing(&String::from_utf8_lossy(&output.stdout)))
}

fn parse_key_listing(stdout: &str) -> Vec<GpgKey> {
    let mut keys = Vec::new();
    let mut current_fpr = String::new();
    let mut current_id = String::new();
    let mut current_trust = String::new();

    for line in stdout.lines() {
        let fields: Vec<&str> = line.split(':').collect();
        if fields.len() < 2 {
            continue;
        }

        match fields[0] {
            "pub" | "sec" => {
                current_id = fields.get(4).unwrap_or(&"").to_string();
                current_trust = fields.get(1).unwrap_or(&"").to_string();
                current_fpr.clear();
            }
            "fpr" => {
                current_fpr = fields.get(9).unwrap_or(&"").to_string();
            }
            "uid" => {
                let uid = fields.get(9).unwrap_or(&"").to_string();
                if !current_id.is_empty() {
                    keys.push(GpgKey {
                        id: current_id.clone(),
                        fingerprint: current_fpr.clone(),
                        uid,
                        trust: current_trust.clone(),
                    });
                }
            }
            _ => {}
        }
    }

    keys
}

#[tauri::command]
pub async fn get_store_gpg_id(app: tauri::AppHandle) -> Result<String> {
    let store_dir = crate::config::commands::get_effective_store_dir(&app)?
        .unwrap_or_else(|| {
            dirs::home_dir()
                .unwrap_or_default()
                .join(".password-store")
                .to_string_lossy()
                .to_string()
        });

    let gpg_id_path = std::path::Path::new(&store_dir).join(".gpg-id");
    let content = tokio::fs::read_to_string(&gpg_id_path)
        .await
        .map_err(|e| PasseroError::GpgError(format!("Failed to read .gpg-id: {}", e)))?;

    Ok(content.trim().to_string())
}

#[tauri::command]
pub async fn generate_gpg_key(app: tauri::AppHandle, params: GenerateKeyParams) -> Result<String> {
    let binary = gpg_binary(&app)?;

    let key_type = params.key_type.as_deref().unwrap_or("RSA");
    let key_length = params.key_length.unwrap_or(4096);

    // Build batch generation script
    let mut batch = format!(
        "Key-Type: {key_type}\n\
         Key-Length: {key_length}\n\
         Subkey-Type: {key_type}\n\
         Subkey-Length: {key_length}\n\
         Name-Real: {}\n\
         Name-Email: {}\n",
        params.name, params.email,
    );

    if let Some(ref passphrase) = params.passphrase {
        batch.push_str(&format!("Passphrase: {passphrase}\n"));
    } else {
        batch.push_str("%no-protection\n");
    }
    batch.push_str("%commit\n");

    let mut child = gpg_command(&binary)
        .args(["--batch", "--gen-key", "--status-fd", "1"])
        .stdin(std::process::Stdio::piped())
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::piped())
        .spawn()?;

    if let Some(mut stdin) = child.stdin.take() {
        use tokio::io::AsyncWriteExt;
        stdin.write_all(batch.as_bytes()).await?;
    }

    let output = child.wait_with_output().await?;
    if !output.status.success() {
        return Err(PasseroError::GpgError(
            String::from_utf8_lossy(&output.stderr).to_string(),
        ));
    }

    // Extract the key fingerprint from the status output
    let stdout = String::from_utf8_lossy(&output.stdout);
    let fingerprint = stdout
        .lines()
        .find(|line| line.contains("KEY_CREATED"))
        .and_then(|line| line.split_whitespace().last())
        .unwrap_or("unknown")
        .to_string();

    Ok(fingerprint)
}

#[tauri::command]
pub async fn import_gpg_key(app: tauri::AppHandle, key_data: String) -> Result<String> {
    let binary = gpg_binary(&app)?;

    let mut child = gpg_command(&binary)
        .args(["--batch", "--import"])
        .stdin(std::process::Stdio::piped())
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::piped())
        .spawn()?;

    if let Some(mut stdin) = child.stdin.take() {
        use tokio::io::AsyncWriteExt;
        stdin.write_all(key_data.as_bytes()).await?;
    }

    let output = child.wait_with_output().await?;
    let stderr = String::from_utf8_lossy(&output.stderr).to_string();

    if !output.status.success() {
        return Err(PasseroError::GpgError(stderr));
    }

    Ok(stderr) // GPG outputs import results to stderr
}

#[tauri::command]
pub async fn import_gpg_key_from_keyserver(
    app: tauri::AppHandle,
    key_id: String,
    keyserver: Option<String>,
) -> Result<String> {
    let binary = gpg_binary(&app)?;

    let server = keyserver.unwrap_or_else(|| "hkps://keys.openpgp.org".to_string());

    let output = gpg_command(&binary)
        .args([
            "--batch",
            "--keyserver",
            &server,
            "--recv-keys",
            &key_id,
        ])
        .output()
        .await?;

    let stderr = String::from_utf8_lossy(&output.stderr).to_string();
    if !output.status.success() {
        return Err(PasseroError::GpgError(stderr));
    }

    Ok(stderr)
}

#[tauri::command]
pub async fn export_gpg_key(app: tauri::AppHandle, key_id: String, secret: bool) -> Result<String> {
    let binary = gpg_binary(&app)?;

    let export_flag = if secret { "--export-secret-keys" } else { "--export" };

    let output = gpg_command(&binary)
        .args(["--batch", "--armor", export_flag, &key_id])
        .output()
        .await?;

    if !output.status.success() {
        return Err(PasseroError::GpgError(
            String::from_utf8_lossy(&output.stderr).to_string(),
        ));
    }

    Ok(String::from_utf8_lossy(&output.stdout).to_string())
}

#[tauri::command]
pub async fn publish_gpg_key(
    app: tauri::AppHandle,
    key_id: String,
    keyserver: Option<String>,
) -> Result<String> {
    let binary = gpg_binary(&app)?;

    let server = keyserver.unwrap_or_else(|| "hkps://keys.openpgp.org".to_string());

    let output = gpg_command(&binary)
        .args([
            "--batch",
            "--keyserver",
            &server,
            "--send-keys",
            &key_id,
        ])
        .output()
        .await?;

    let stderr = String::from_utf8_lossy(&output.stderr).to_string();
    if !output.status.success() {
        return Err(PasseroError::GpgError(stderr));
    }

    Ok(stderr)
}

#[tauri::command]
pub async fn set_gpg_key_trust(
    app: tauri::AppHandle,
    fingerprint: String,
    trust_level: u32, // 1-5 (undefined, never, marginal, full, ultimate)
) -> Result<()> {
    let binary = gpg_binary(&app)?;

    if !(1..=5).contains(&trust_level) {
        return Err(PasseroError::GpgError("Trust level must be 1-5".into()));
    }

    let trust_input = format!("{trust_level}\ny\n");

    let mut child = gpg_command(&binary)
        .args(["--batch", "--command-fd", "0", "--edit-key", &fingerprint, "trust"])
        .stdin(std::process::Stdio::piped())
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::piped())
        .spawn()?;

    if let Some(mut stdin) = child.stdin.take() {
        use tokio::io::AsyncWriteExt;
        stdin.write_all(trust_input.as_bytes()).await?;
    }

    let output = child.wait_with_output().await?;
    if !output.status.success() {
        return Err(PasseroError::GpgError(
            String::from_utf8_lossy(&output.stderr).to_string(),
        ));
    }

    Ok(())
}

/// Resolve a list of key IDs/fingerprints to full GpgKey info from the local keyring.
#[tauri::command]
pub async fn resolve_gpg_keys(app: tauri::AppHandle, key_ids: Vec<String>) -> Result<Vec<Option<GpgKey>>> {
    let all_public = list_gpg_keys(app.clone()).await?;
    let all_secret = list_gpg_secret_keys(app).await?;

    Ok(key_ids
        .iter()
        .map(|id| {
            all_public
                .iter()
                .find(|k| {
                    k.id == *id
                        || k.fingerprint == *id
                        || k.id.ends_with(id)
                        || id.ends_with(&k.id)
                        || k.fingerprint.ends_with(id)
                        || id.ends_with(&k.fingerprint)
                })
                .map(|k| {
                    let mut key = k.clone();
                    // Mark whether we have the secret key
                    if all_secret.iter().any(|sk| sk.id == key.id) {
                        key.trust = format!("{}+secret", key.trust);
                    }
                    key
                })
        })
        .collect())
}

/// Search a keyserver for keys matching a query (email, name, or key ID).
#[tauri::command]
pub async fn search_gpg_keyserver(
    app: tauri::AppHandle,
    query: String,
    keyserver: Option<String>,
) -> Result<Vec<GpgKey>> {
    let binary = gpg_binary(&app)?;
    let server = keyserver.unwrap_or_else(|| "hkps://keys.openpgp.org".to_string());

    let output = gpg_command(&binary)
        .args([
            "--batch",
            "--keyserver",
            &server,
            "--search-keys",
            "--with-colons",
            &query,
        ])
        .stdin(std::process::Stdio::null())
        .output()
        .await?;

    // --search-keys outputs to stdout in colon format when --with-colons is used
    let stdout = String::from_utf8_lossy(&output.stdout);
    let mut keys = Vec::new();
    let mut current_id = String::new();

    for line in stdout.lines() {
        let fields: Vec<&str> = line.split(':').collect();
        if fields.len() < 2 {
            continue;
        }
        match fields[0] {
            "pub" => {
                current_id = fields.get(4).unwrap_or(&"").to_string();
            }
            "uid" => {
                let uid = fields.get(9).unwrap_or(&"").to_string();
                if !current_id.is_empty() && !uid.is_empty() {
                    keys.push(GpgKey {
                        id: current_id.clone(),
                        fingerprint: String::new(),
                        uid,
                        trust: String::new(),
                    });
                }
            }
            _ => {}
        }
    }

    Ok(keys)
}

#[tauri::command]
pub async fn delete_gpg_key(app: tauri::AppHandle, fingerprint: String, secret: bool) -> Result<()> {
    let binary = gpg_binary(&app)?;

    // Must delete secret key first if it exists
    if secret {
        let output = gpg_command(&binary)
            .args(["--batch", "--yes", "--delete-secret-and-public-key", &fingerprint])
            .output()
            .await?;

        if !output.status.success() {
            return Err(PasseroError::GpgError(
                String::from_utf8_lossy(&output.stderr).to_string(),
            ));
        }
    } else {
        let output = gpg_command(&binary)
            .args(["--batch", "--yes", "--delete-keys", &fingerprint])
            .output()
            .await?;

        if !output.status.success() {
            return Err(PasseroError::GpgError(
                String::from_utf8_lossy(&output.stderr).to_string(),
            ));
        }
    }

    Ok(())
}
