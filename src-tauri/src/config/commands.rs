use super::model::{AppConfig, Vault};
use crate::error::{PasseroError, Result};
use tauri_plugin_store::StoreExt;

fn load_config(app: &tauri::AppHandle) -> Result<(AppConfig, std::sync::Arc<tauri_plugin_store::Store<tauri::Wry>>)> {
    let store = app
        .store("config.json")
        .map_err(|e: tauri_plugin_store::Error| PasseroError::ConfigError(e.to_string()))?;

    let config = AppConfig {
        pass_binary: store
            .get("pass_binary")
            .and_then(|v: serde_json::Value| v.as_str().map(String::from)),
        gpg_binary: store
            .get("gpg_binary")
            .and_then(|v: serde_json::Value| v.as_str().map(String::from)),
        git_binary: store
            .get("git_binary")
            .and_then(|v: serde_json::Value| v.as_str().map(String::from)),
        password_store_dir: store
            .get("password_store_dir")
            .and_then(|v: serde_json::Value| v.as_str().map(String::from)),
        clipboard_timeout: store
            .get("clipboard_timeout")
            .and_then(|v: serde_json::Value| v.as_u64())
            .unwrap_or(45) as u32,
        vaults: store
            .get("vaults")
            .and_then(|v: serde_json::Value| serde_json::from_value(v).ok())
            .unwrap_or_default(),
        active_vault_id: store
            .get("active_vault_id")
            .and_then(|v: serde_json::Value| v.as_str().map(String::from)),
        device_key_fingerprint: store
            .get("device_key_fingerprint")
            .and_then(|v: serde_json::Value| v.as_str().map(String::from)),
        repo_url: store
            .get("repo_url")
            .and_then(|v: serde_json::Value| v.as_str().map(String::from)),
    };

    Ok((config, store))
}

fn save_config(
    store: &std::sync::Arc<tauri_plugin_store::Store<tauri::Wry>>,
    config: &AppConfig,
) -> Result<()> {
    if let Some(ref v) = config.pass_binary {
        store.set("pass_binary", serde_json::json!(v));
    } else {
        store.delete("pass_binary");
    }
    if let Some(ref v) = config.gpg_binary {
        store.set("gpg_binary", serde_json::json!(v));
    } else {
        store.delete("gpg_binary");
    }
    if let Some(ref v) = config.git_binary {
        store.set("git_binary", serde_json::json!(v));
    } else {
        store.delete("git_binary");
    }
    if let Some(ref v) = config.password_store_dir {
        store.set("password_store_dir", serde_json::json!(v));
    } else {
        store.delete("password_store_dir");
    }
    store.set("clipboard_timeout", serde_json::json!(config.clipboard_timeout));
    store.set("vaults", serde_json::json!(config.vaults));
    if let Some(ref v) = config.active_vault_id {
        store.set("active_vault_id", serde_json::json!(v));
    } else {
        store.delete("active_vault_id");
    }
    if let Some(ref v) = config.device_key_fingerprint {
        store.set("device_key_fingerprint", serde_json::json!(v));
    } else {
        store.delete("device_key_fingerprint");
    }
    store.delete("pat");
    if let Some(ref v) = config.repo_url {
        store.set("repo_url", serde_json::json!(v));
    } else {
        store.delete("repo_url");
    }

    store
        .save()
        .map_err(|e: tauri_plugin_store::Error| PasseroError::ConfigError(e.to_string()))?;

    Ok(())
}

/// Get the effective password store directory, considering active vault.
pub(crate) fn get_effective_store_dir(app: &tauri::AppHandle) -> Result<Option<String>> {
    let (config, _store) = load_config(app)?;
    Ok(config.effective_store_dir().map(String::from))
}

pub(crate) fn set_device_key_fingerprint(app: &tauri::AppHandle, fingerprint: &str) -> Result<()> {
    let (mut config, store) = load_config(app)?;
    config.device_key_fingerprint = Some(fingerprint.to_string());
    save_config(&store, &config)
}

pub(crate) fn get_device_key_fingerprint(app: &tauri::AppHandle) -> Result<Option<String>> {
    let (config, _store) = load_config(app)?;
    Ok(config.device_key_fingerprint)
}

pub(crate) fn get_pat(app: &tauri::AppHandle) -> Result<Option<String>> {
    let store = app
        .store("config.json")
        .map_err(|e: tauri_plugin_store::Error| PasseroError::ConfigError(e.to_string()))?;
    Ok(store
        .get("pat")
        .and_then(|v: serde_json::Value| v.as_str().map(String::from)))
}

pub(crate) fn clear_pat(app: &tauri::AppHandle) -> Result<()> {
    let store = app
        .store("config.json")
        .map_err(|e: tauri_plugin_store::Error| PasseroError::ConfigError(e.to_string()))?;
    if store.delete("pat") {
        store
            .save()
            .map_err(|e: tauri_plugin_store::Error| PasseroError::ConfigError(e.to_string()))?;
    }
    Ok(())
}

pub(crate) fn get_repo_url(app: &tauri::AppHandle) -> Result<Option<String>> {
    let (config, _store) = load_config(app)?;
    Ok(config.repo_url)
}

pub(crate) fn set_sync_settings(
    app: &tauri::AppHandle,
    repo_url: Option<String>,
) -> Result<()> {
    let (mut config, store) = load_config(app)?;
    config.repo_url = repo_url.filter(|s| !s.is_empty());
    save_config(&store, &config)
}

pub(crate) fn set_store_dir(app: &tauri::AppHandle, path: &str) -> Result<()> {
    let (mut config, store) = load_config(app)?;
    if config.password_store_dir.as_deref() != Some(path) {
        config.password_store_dir = Some(path.to_string());
        save_config(&store, &config)?;
    }
    Ok(())
}

pub(crate) fn register_cloned_vault(
    app: &tauri::AppHandle,
    name: &str,
    path: &str,
    url: &str,
) -> Result<()> {
    let (mut config, store) = load_config(app)?;

    config.repo_url = Some(url.to_string());
    config.password_store_dir = Some(path.to_string());

    let id = match config.vaults.iter().find(|v| v.path == path) {
        Some(existing) => existing.id.clone(),
        None => {
            let id = format!(
                "vault_{}",
                std::time::SystemTime::now()
                    .duration_since(std::time::UNIX_EPOCH)
                    .unwrap_or_default()
                    .as_millis()
            );
            config.vaults.push(Vault {
                id: id.clone(),
                name: name.to_string(),
                path: path.to_string(),
            });
            id
        }
    };
    config.active_vault_id = Some(id);

    save_config(&store, &config)
}

pub(crate) fn reset_device_config(app: &tauri::AppHandle) -> Result<()> {
    let (mut config, store) = load_config(app)?;
    config.device_key_fingerprint = None;
    config.repo_url = None;
    config.vaults.clear();
    config.active_vault_id = None;
    config.password_store_dir = None;
    save_config(&store, &config)
}

#[tauri::command]
pub async fn get_config(app: tauri::AppHandle) -> Result<AppConfig> {
    let (config, _store) = load_config(&app)?;
    Ok(config)
}

#[tauri::command]
pub async fn set_config(app: tauri::AppHandle, config: AppConfig) -> Result<()> {
    let (_old, store) = load_config(&app)?;
    save_config(&store, &config)
}

#[tauri::command]
pub async fn get_password_store_path(app: tauri::AppHandle) -> Result<String> {
    let path = get_effective_store_dir(&app)?.unwrap_or_else(|| {
        dirs::home_dir()
            .unwrap_or_default()
            .join(".password-store")
            .to_string_lossy()
            .to_string()
    });
    Ok(path)
}

#[tauri::command]
pub async fn list_vaults(app: tauri::AppHandle) -> Result<Vec<Vault>> {
    let (config, _store) = load_config(&app)?;
    Ok(config.vaults)
}

#[tauri::command]
pub async fn add_vault(app: tauri::AppHandle, name: String, path: String) -> Result<Vault> {
    let (mut config, store) = load_config(&app)?;

    let id = format!("vault_{}", std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis());

    let vault = Vault {
        id: id.clone(),
        name,
        path,
    };

    config.vaults.push(vault.clone());

    // If this is the first vault, make it active
    if config.vaults.len() == 1 {
        config.active_vault_id = Some(id);
    }

    save_config(&store, &config)?;
    Ok(vault)
}

#[tauri::command]
pub async fn remove_vault(app: tauri::AppHandle, id: String) -> Result<()> {
    let (mut config, store) = load_config(&app)?;

    config.vaults.retain(|v| v.id != id);

    if config.active_vault_id.as_deref() == Some(&id) {
        config.active_vault_id = config.vaults.first().map(|v| v.id.clone());
    }

    save_config(&store, &config)
}

#[tauri::command]
pub async fn set_active_vault(app: tauri::AppHandle, id: Option<String>) -> Result<()> {
    let (mut config, store) = load_config(&app)?;

    if let Some(ref vault_id) = id {
        if !config.vaults.iter().any(|v| &v.id == vault_id) {
            return Err(PasseroError::ConfigError("Vault not found".into()));
        }
    }

    config.active_vault_id = id;
    save_config(&store, &config)
}
