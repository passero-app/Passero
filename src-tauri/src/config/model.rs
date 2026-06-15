use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Vault {
    pub id: String,
    pub name: String,
    pub path: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AppConfig {
    pub pass_binary: Option<String>,
    pub gpg_binary: Option<String>,
    pub git_binary: Option<String>,
    pub password_store_dir: Option<String>,
    pub clipboard_timeout: u32,
    #[serde(default)]
    pub vaults: Vec<Vault>,
    pub active_vault_id: Option<String>,
    #[serde(default)]
    pub device_key_fingerprint: Option<String>,
    #[serde(default)]
    pub pat: Option<String>,
    #[serde(default)]
    pub repo_url: Option<String>,
}

impl Default for AppConfig {
    fn default() -> Self {
        Self {
            pass_binary: None,
            gpg_binary: None,
            git_binary: None,
            password_store_dir: None,
            clipboard_timeout: 45,
            vaults: Vec::new(),
            active_vault_id: None,
            device_key_fingerprint: None,
            pat: None,
            repo_url: None,
        }
    }
}

impl AppConfig {
    /// Resolve the effective store directory: active vault path takes priority,
    /// then `password_store_dir`, then `None` (caller provides the default).
    pub fn effective_store_dir(&self) -> Option<&str> {
        if let Some(ref active_id) = self.active_vault_id {
            if let Some(vault) = self.vaults.iter().find(|v| v.id == *active_id) {
                return Some(&vault.path);
            }
        }
        self.password_store_dir.as_deref()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn effective_store_dir_no_vaults_no_default() {
        let config = AppConfig::default();
        assert_eq!(config.effective_store_dir(), None);
    }

    #[test]
    fn effective_store_dir_falls_back_to_password_store_dir() {
        let config = AppConfig {
            password_store_dir: Some("/home/user/.password-store".into()),
            ..Default::default()
        };
        assert_eq!(config.effective_store_dir(), Some("/home/user/.password-store"));
    }

    #[test]
    fn effective_store_dir_active_vault_takes_priority() {
        let config = AppConfig {
            password_store_dir: Some("/home/user/.password-store".into()),
            vaults: vec![
                Vault { id: "v1".into(), name: "Work".into(), path: "/work/pass".into() },
                Vault { id: "v2".into(), name: "Personal".into(), path: "/personal/pass".into() },
            ],
            active_vault_id: Some("v2".into()),
            ..Default::default()
        };
        assert_eq!(config.effective_store_dir(), Some("/personal/pass"));
    }

    #[test]
    fn effective_store_dir_invalid_active_id_falls_back() {
        let config = AppConfig {
            password_store_dir: Some("/default".into()),
            vaults: vec![
                Vault { id: "v1".into(), name: "Work".into(), path: "/work/pass".into() },
            ],
            active_vault_id: Some("nonexistent".into()),
            ..Default::default()
        };
        // Active vault ID doesn't match any vault, falls back to password_store_dir
        assert_eq!(config.effective_store_dir(), Some("/default"));
    }

    #[test]
    fn effective_store_dir_invalid_active_id_no_fallback() {
        let config = AppConfig {
            vaults: vec![
                Vault { id: "v1".into(), name: "Work".into(), path: "/work/pass".into() },
            ],
            active_vault_id: Some("nonexistent".into()),
            ..Default::default()
        };
        assert_eq!(config.effective_store_dir(), None);
    }

    #[test]
    fn effective_store_dir_no_active_id_with_vaults() {
        let config = AppConfig {
            password_store_dir: Some("/default".into()),
            vaults: vec![
                Vault { id: "v1".into(), name: "Work".into(), path: "/work/pass".into() },
            ],
            active_vault_id: None,
            ..Default::default()
        };
        // No active vault selected, falls back to password_store_dir
        assert_eq!(config.effective_store_dir(), Some("/default"));
    }
}
