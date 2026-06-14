#![cfg(target_os = "ios")]

use std::path::PathBuf;
use std::sync::Mutex;
use std::time::SystemTime;

use serde::{Deserialize, Serialize};
use tauri::State;
use totp_rs::TOTP;
use url::Url;

use crate::config::commands as config;
use crate::error::{PasseroError, Result};
use passero_core::crypto::{cert_from_bytes, generate_key, Cert};
use passero_core::{store, sync};

pub struct IosState {
    pub store_dir: Mutex<PathBuf>,
    pub key_armored: Mutex<Option<Vec<u8>>>,
    pub token: Mutex<Option<String>>,
}

impl Default for IosState {
    fn default() -> Self {
        Self {
            store_dir: Mutex::new(PathBuf::new()),
            key_armored: Mutex::new(None),
            token: Mutex::new(None),
        }
    }
}

#[derive(Debug, Clone, Serialize)]
pub struct PasswordEntry {
    pub path: String,
    pub name: String,
    pub is_dir: bool,
    pub children: Vec<PasswordEntry>,
}

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
    pub key_type: Option<String>,
    pub key_length: Option<u32>,
}

#[derive(Debug, Clone, Serialize)]
pub struct GitLogEntry {
    pub hash: String,
    pub message: String,
    pub author: String,
    pub date: String,
}

#[derive(Debug, Clone, Serialize)]
pub struct TotpCode {
    pub code: String,
    pub remaining_seconds: u64,
    pub period: u64,
}

#[derive(Debug, Clone, Serialize)]
pub struct GeneratedKey {
    pub fingerprint: String,
    pub armored: String,
}

#[derive(Debug, Clone, Serialize)]
pub struct DeviceKeyStatus {
    pub has_key: bool,
    pub fingerprint: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
pub struct TotpInfo {
    pub issuer: Option<String>,
    pub account: Option<String>,
    pub uri: String,
}

fn store_dir(state: &State<'_, IosState>) -> PathBuf {
    state.store_dir.lock().unwrap().clone()
}

fn token(state: &State<'_, IosState>) -> Option<String> {
    state.token.lock().unwrap().clone()
}

fn loaded_cert(state: &State<'_, IosState>) -> Result<Cert> {
    let guard = state.key_armored.lock().unwrap();
    let bytes = guard
        .as_ref()
        .ok_or_else(|| PasseroError::GpgError("store is locked (no key loaded)".into()))?;
    cert_from_bytes(bytes).map_err(|e| PasseroError::GpgError(e.to_string()))
}

fn tree_from_paths(paths: &[String]) -> Vec<PasswordEntry> {
    let mut roots: Vec<PasswordEntry> = Vec::new();

    for full in paths {
        let segments: Vec<&str> = full.split('/').collect();
        insert_segments(&mut roots, &segments, String::new());
    }

    sort_tree(&mut roots);
    roots
}

fn insert_segments(level: &mut Vec<PasswordEntry>, segments: &[&str], prefix: String) {
    if segments.is_empty() {
        return;
    }
    let name = segments[0].to_string();
    let path = if prefix.is_empty() {
        name.clone()
    } else {
        format!("{prefix}/{name}")
    };
    let is_dir = segments.len() > 1;

    let idx = match level.iter().position(|e| e.name == name && e.is_dir == is_dir) {
        Some(i) => i,
        None => {
            level.push(PasswordEntry {
                path: path.clone(),
                name,
                is_dir,
                children: Vec::new(),
            });
            level.len() - 1
        }
    };

    if is_dir {
        insert_segments(&mut level[idx].children, &segments[1..], path);
    }
}

fn sort_tree(level: &mut [PasswordEntry]) {
    level.sort_by(|a, b| b.is_dir.cmp(&a.is_dir).then(a.name.cmp(&b.name)));
    for entry in level.iter_mut() {
        sort_tree(&mut entry.children);
    }
}

#[tauri::command]
pub async fn list_passwords(state: State<'_, IosState>) -> Result<Vec<PasswordEntry>> {
    let dir = store_dir(&state);
    if !dir.exists() {
        return Ok(vec![]);
    }
    let paths = store::list(&dir).map_err(|e| PasseroError::PassError(e.to_string()))?;
    Ok(tree_from_paths(&paths))
}

#[tauri::command]
pub async fn show_password(state: State<'_, IosState>, path: String) -> Result<String> {
    let dir = store_dir(&state);
    let key = loaded_cert(&state)?;
    let bytes = store::show(&dir, &path, &key).map_err(|e| PasseroError::PassError(e.to_string()))?;
    Ok(String::from_utf8_lossy(&bytes).to_string())
}

#[tauri::command]
pub async fn insert_password(
    state: State<'_, IosState>,
    path: String,
    content: String,
) -> Result<()> {
    let dir = store_dir(&state);
    let key = loaded_cert(&state)?;
    store::insert(&dir, &path, content.as_bytes(), std::slice::from_ref(&key))
        .map_err(|e| PasseroError::PassError(e.to_string()))
}

#[tauri::command]
pub async fn edit_password(
    state: State<'_, IosState>,
    path: String,
    content: String,
) -> Result<()> {
    insert_password(state, path, content).await
}

#[tauri::command]
pub async fn delete_password(state: State<'_, IosState>, path: String) -> Result<()> {
    let dir = store_dir(&state);
    let file = dir.join(&path).with_extension("gpg");
    std::fs::remove_file(file)?;
    Ok(())
}

fn generate_random_password(length: u32, symbols: bool) -> String {
    const ALNUM: &[u8] = b"abcdefghijklmnopqrstuvwxyzABCDEFGHIJKLMNOPQRSTUVWXYZ0123456789";
    const SYMS: &[u8] = b"!@#$%^&*()-_=+[]{};:,.<>?";

    let mut pool = ALNUM.to_vec();
    if symbols {
        pool.extend_from_slice(SYMS);
    }

    let mut out = String::with_capacity(length as usize);
    let mut seed = SystemTime::now()
        .duration_since(SystemTime::UNIX_EPOCH)
        .map(|d| d.as_nanos() as u64)
        .unwrap_or(0x9e3779b97f4a7c15);

    for _ in 0..length {
        seed ^= seed << 13;
        seed ^= seed >> 7;
        seed ^= seed << 17;
        let idx = (seed % pool.len() as u64) as usize;
        out.push(pool[idx] as char);
    }
    out
}

#[tauri::command]
pub async fn generate_password(
    state: State<'_, IosState>,
    path: String,
    length: u32,
    symbols: bool,
) -> Result<String> {
    let password = generate_random_password(length, symbols);
    insert_password(state, path, password.clone()).await?;
    Ok(password)
}

#[tauri::command]
pub async fn copy_password(_state: State<'_, IosState>, _path: String) -> Result<()> {
    Ok(())
}

#[tauri::command]
pub async fn list_recipients(state: State<'_, IosState>) -> Result<Vec<String>> {
    let dir = store_dir(&state);
    match store::recipients_for(&dir, "") {
        Ok(r) => Ok(r),
        Err(_) => Ok(vec![]),
    }
}

#[tauri::command]
pub async fn add_recipient(_state: State<'_, IosState>, _gpg_id: String) -> Result<Vec<String>> {
    Err(PasseroError::GpgError("not supported on iOS (M0)".into()))
}

#[tauri::command]
pub async fn remove_recipient(_state: State<'_, IosState>, _gpg_id: String) -> Result<Vec<String>> {
    Err(PasseroError::GpgError("not supported on iOS (M0)".into()))
}

#[tauri::command]
pub async fn init_password_store(state: State<'_, IosState>, gpg_ids: Vec<String>) -> Result<()> {
    let dir = store_dir(&state);
    store::init(&dir, &gpg_ids).map_err(|e| PasseroError::PassError(e.to_string()))
}

#[tauri::command]
pub async fn generate_in_app_key(
    app: tauri::AppHandle,
    state: State<'_, IosState>,
    user_id: String,
) -> Result<GeneratedKey> {
    let (cert, armored_bytes) = generate_key(&user_id).map_err(|e| PasseroError::GpgError(e.to_string()))?;
    let fingerprint = cert.fingerprint().to_hex();
    *state.key_armored.lock().unwrap() = Some(armored_bytes.clone());
    config::set_device_key_fingerprint(&app, &fingerprint)?;
    let armored = String::from_utf8(armored_bytes)
        .map_err(|e| PasseroError::GpgError(format!("armored key is not valid UTF-8: {e}")))?;
    Ok(GeneratedKey {
        fingerprint,
        armored,
    })
}

#[tauri::command]
pub async fn device_key_status(app: tauri::AppHandle) -> Result<DeviceKeyStatus> {
    let fingerprint = config::get_device_key_fingerprint(&app)?;
    Ok(DeviceKeyStatus {
        has_key: fingerprint.is_some(),
        fingerprint,
    })
}

#[tauri::command]
pub async fn reset_device(app: tauri::AppHandle, state: State<'_, IosState>) -> Result<()> {
    config::reset_device_config(&app)?;
    let dir = store_dir(&state);
    if dir.exists() {
        for entry in std::fs::read_dir(&dir)? {
            let path = entry?.path();
            if path.is_dir() {
                std::fs::remove_dir_all(&path)?;
            } else {
                std::fs::remove_file(&path)?;
            }
        }
    }
    *state.key_armored.lock().unwrap() = None;
    *state.token.lock().unwrap() = None;
    Ok(())
}

#[tauri::command]
pub async fn load_key(
    app: tauri::AppHandle,
    state: State<'_, IosState>,
    armored: String,
) -> Result<String> {
    let bytes = armored.into_bytes();
    let cert = cert_from_bytes(&bytes).map_err(|e| PasseroError::GpgError(e.to_string()))?;
    let fingerprint = cert.fingerprint().to_hex();
    *state.key_armored.lock().unwrap() = Some(bytes);
    if let Some(pat) = config::get_pat(&app)? {
        *state.token.lock().unwrap() = Some(pat);
    }
    Ok(fingerprint)
}

#[tauri::command]
pub async fn clone_store(
    app: tauri::AppHandle,
    state: State<'_, IosState>,
    url: String,
    token: String,
) -> Result<()> {
    let dir = store_dir(&state);
    *state.token.lock().unwrap() = Some(token.clone());
    sync::clone(&url, &dir, Some(&token)).map_err(|e| PasseroError::GitError(e.to_string()))?;
    let name = vault_name_from_url(&url);
    config::register_cloned_vault(&app, &name, &dir.to_string_lossy(), &token)?;
    Ok(())
}

fn vault_name_from_url(url: &str) -> String {
    url.trim_end_matches('/')
        .rsplit('/')
        .next()
        .map(|s| s.trim_end_matches(".git"))
        .filter(|s| !s.is_empty())
        .unwrap_or("Vault")
        .to_string()
}

#[tauri::command]
pub async fn init_store(state: State<'_, IosState>, fingerprints: Vec<String>) -> Result<()> {
    let dir = store_dir(&state);
    store::init(&dir, &fingerprints).map_err(|e| PasseroError::PassError(e.to_string()))
}

#[tauri::command]
pub async fn list_gpg_keys(_state: State<'_, IosState>) -> Result<Vec<GpgKey>> {
    Ok(vec![])
}

#[tauri::command]
pub async fn list_gpg_secret_keys(_state: State<'_, IosState>) -> Result<Vec<GpgKey>> {
    Ok(vec![])
}

#[tauri::command]
pub async fn get_store_gpg_id(state: State<'_, IosState>) -> Result<String> {
    let dir = store_dir(&state);
    let recipients = store::recipients_for(&dir, "")
        .map_err(|e| PasseroError::GpgError(e.to_string()))?;
    Ok(recipients.join("\n"))
}

#[tauri::command]
pub async fn generate_gpg_key(
    app: tauri::AppHandle,
    state: State<'_, IosState>,
    params: GenerateKeyParams,
) -> Result<String> {
    let user_id = if params.email.is_empty() {
        params.name.clone()
    } else {
        format!("{} <{}>", params.name, params.email)
    };
    let generated = generate_in_app_key(app, state, user_id).await?;
    Ok(generated.fingerprint)
}

#[tauri::command]
pub async fn import_gpg_key(_state: State<'_, IosState>, _key_data: String) -> Result<String> {
    Err(PasseroError::GpgError("not supported on iOS (M0)".into()))
}

#[tauri::command]
pub async fn import_gpg_key_from_keyserver(
    _state: State<'_, IosState>,
    _key_id: String,
    _keyserver: Option<String>,
) -> Result<String> {
    Err(PasseroError::GpgError("not supported on iOS (M0)".into()))
}

#[tauri::command]
pub async fn export_gpg_key(
    _state: State<'_, IosState>,
    _key_id: String,
    _secret: bool,
) -> Result<String> {
    Err(PasseroError::GpgError("not supported on iOS (M0)".into()))
}

#[tauri::command]
pub async fn publish_gpg_key(
    _state: State<'_, IosState>,
    _key_id: String,
    _keyserver: Option<String>,
) -> Result<String> {
    Err(PasseroError::GpgError("not supported on iOS (M0)".into()))
}

#[tauri::command]
pub async fn set_gpg_key_trust(
    _state: State<'_, IosState>,
    _fingerprint: String,
    _trust_level: u32,
) -> Result<()> {
    Err(PasseroError::GpgError("not supported on iOS (M0)".into()))
}

#[tauri::command]
pub async fn delete_gpg_key(
    _state: State<'_, IosState>,
    _fingerprint: String,
    _secret: bool,
) -> Result<()> {
    Err(PasseroError::GpgError("not supported on iOS (M0)".into()))
}

#[tauri::command]
pub async fn resolve_gpg_keys(
    _state: State<'_, IosState>,
    key_ids: Vec<String>,
) -> Result<Vec<Option<GpgKey>>> {
    Ok(key_ids.iter().map(|_| None).collect())
}

#[tauri::command]
pub async fn search_gpg_keyserver(
    _state: State<'_, IosState>,
    _query: String,
    _keyserver: Option<String>,
) -> Result<Vec<GpgKey>> {
    Err(PasseroError::GpgError("not supported on iOS (M0)".into()))
}

fn open_repo(dir: &std::path::Path) -> Result<git2::Repository> {
    git2::Repository::open(dir).map_err(|e| PasseroError::GitError(e.to_string()))
}

#[tauri::command]
pub async fn git_pull(state: State<'_, IosState>) -> Result<String> {
    let dir = store_dir(&state);
    let tok = token(&state);
    let repo = open_repo(&dir)?;
    sync::pull(&repo, tok.as_deref()).map_err(|e| PasseroError::GitError(e.to_string()))?;
    Ok(String::new())
}

#[tauri::command]
pub async fn git_push(state: State<'_, IosState>) -> Result<String> {
    let dir = store_dir(&state);
    let tok = token(&state);
    let repo = open_repo(&dir)?;
    sync::commit_all(&repo, "passero: sync").map_err(|e| PasseroError::GitError(e.to_string()))?;
    sync::push(&repo, tok.as_deref()).map_err(|e| PasseroError::GitError(e.to_string()))?;
    Ok(String::new())
}

#[tauri::command]
pub async fn git_log(_state: State<'_, IosState>, _count: Option<u32>) -> Result<Vec<GitLogEntry>> {
    Ok(vec![])
}

#[tauri::command]
pub async fn git_clone(
    state: State<'_, IosState>,
    url: String,
    path: Option<String>,
) -> Result<()> {
    let into = match path {
        Some(p) => PathBuf::from(p),
        None => store_dir(&state),
    };
    let tok = token(&state);
    sync::clone(&url, &into, tok.as_deref())
        .map_err(|e| PasseroError::GitError(e.to_string()))?;
    Ok(())
}

fn read_entry(state: &State<'_, IosState>, path: &str) -> Result<String> {
    let dir = store_dir(state);
    let key = loaded_cert(state)?;
    let bytes = store::show(&dir, path, &key).map_err(|e| PasseroError::PassError(e.to_string()))?;
    Ok(String::from_utf8_lossy(&bytes).to_string())
}

fn write_entry(state: &State<'_, IosState>, path: &str, content: &str) -> Result<()> {
    let dir = store_dir(state);
    let key = loaded_cert(state)?;
    store::insert(&dir, path, content.as_bytes(), std::slice::from_ref(&key))
        .map_err(|e| PasseroError::PassError(e.to_string()))
}

fn find_otp_uri(content: &str) -> Option<String> {
    content
        .lines()
        .find(|line| line.trim().starts_with("otpauth://"))
        .map(|line| line.trim().to_string())
}

fn generate_from_uri(uri: &str) -> Result<(TotpCode, TotpInfo)> {
    let totp = TOTP::from_url(uri)
        .map_err(|e| PasseroError::TotpError(format!("Invalid otpauth URI: {e}")))?;

    let now = SystemTime::now()
        .duration_since(SystemTime::UNIX_EPOCH)
        .map_err(|e| PasseroError::TotpError(e.to_string()))?
        .as_secs();

    let code = totp.generate(now).to_string();
    let period = totp.step;
    let remaining = period - (now % period);

    let parsed = Url::parse(uri).ok();
    let account = parsed.as_ref().and_then(|u| {
        let path = u.path().trim_start_matches('/');
        if path.is_empty() {
            None
        } else {
            let account = if let Some((_issuer, acct)) = path.split_once(':') {
                acct
            } else {
                path
            };
            let decoded = urlencoding::decode(account).unwrap_or(account.into());
            Some(decoded.into_owned())
        }
    });

    let info = TotpInfo {
        issuer: totp.issuer.clone(),
        account,
        uri: uri.to_string(),
    };

    Ok((
        TotpCode {
            code,
            remaining_seconds: remaining,
            period,
        },
        info,
    ))
}

#[tauri::command]
pub async fn get_totp(state: State<'_, IosState>, path: String) -> Result<TotpCode> {
    let content = read_entry(&state, &path)?;
    let uri = find_otp_uri(&content)
        .ok_or_else(|| PasseroError::TotpError("No otpauth:// URI found in entry".into()))?;
    let (code, _info) = generate_from_uri(&uri)?;
    Ok(code)
}

#[tauri::command]
pub async fn get_totp_info(state: State<'_, IosState>, path: String) -> Result<Option<TotpInfo>> {
    let content = read_entry(&state, &path)?;
    let Some(uri) = find_otp_uri(&content) else {
        return Ok(None);
    };
    let (_code, info) = generate_from_uri(&uri)?;
    Ok(Some(info))
}

#[tauri::command]
pub async fn insert_totp(
    state: State<'_, IosState>,
    path: String,
    secret: String,
    issuer: Option<String>,
    account: Option<String>,
) -> Result<()> {
    let account_name = account.as_deref().unwrap_or("unknown");
    let label = match &issuer {
        Some(iss) => format!("{iss}:{account_name}"),
        None => account_name.to_string(),
    };
    let mut uri = format!(
        "otpauth://totp/{}?secret={}",
        urlencoding::encode(&label),
        secret.replace(' ', "").to_uppercase()
    );
    if let Some(iss) = &issuer {
        uri.push_str(&format!("&issuer={}", urlencoding::encode(iss)));
    }

    TOTP::from_url(&uri)
        .map_err(|e| PasseroError::TotpError(format!("Invalid TOTP parameters: {e}")))?;

    let existing = read_entry(&state, &path).unwrap_or_default();
    let new_content = merge_otp(&existing, &uri);
    write_entry(&state, &path, &new_content)
}

fn merge_otp(existing: &str, uri: &str) -> String {
    if find_otp_uri(existing).is_some() {
        existing
            .lines()
            .map(|line| {
                if line.trim().starts_with("otpauth://") {
                    uri
                } else {
                    line
                }
            })
            .collect::<Vec<_>>()
            .join("\n")
    } else {
        let trimmed = existing.trim_end();
        if trimmed.is_empty() {
            uri.to_string()
        } else {
            format!("{trimmed}\n{uri}")
        }
    }
}

#[tauri::command]
pub async fn decode_qr_image(image_path: String) -> Result<String> {
    let img = image::open(&image_path)
        .map_err(|e| PasseroError::TotpError(format!("Failed to open image: {e}")))?;
    let gray = img.to_luma8();
    let mut prepared = rqrr::PreparedImage::prepare(gray);
    let grids = prepared.detect_grids();
    if grids.is_empty() {
        return Err(PasseroError::TotpError("No QR code found in image".into()));
    }
    let (_meta, content) = grids[0]
        .decode()
        .map_err(|e| PasseroError::TotpError(format!("Failed to decode QR: {e}")))?;
    if !content.starts_with("otpauth://") {
        return Err(PasseroError::TotpError(
            "QR code does not contain an otpauth:// URI".into(),
        ));
    }
    TOTP::from_url(&content)
        .map_err(|e| PasseroError::TotpError(format!("Invalid otpauth URI in QR: {e}")))?;
    Ok(content)
}

#[tauri::command]
pub async fn import_totp_from_qr(
    state: State<'_, IosState>,
    path: String,
    image_path: String,
) -> Result<TotpCode> {
    let uri = decode_qr_image(image_path).await?;
    let existing = read_entry(&state, &path).unwrap_or_default();
    let new_content = merge_otp(&existing, &uri);
    write_entry(&state, &path, &new_content)?;
    let (code, _info) = generate_from_uri(&uri)?;
    Ok(code)
}
