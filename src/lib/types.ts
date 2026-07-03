export interface PasswordEntry {
  path: string;
  name: string;
  is_dir: boolean;
  children: PasswordEntry[];
}

export interface AppConfig {
  pass_binary: string | null;
  gpg_binary: string | null;
  git_binary: string | null;
  password_store_dir: string | null;
  clipboard_timeout: number;
  vaults: Vault[];
  active_vault_id: string | null;
  repo_url?: string | null;
  device_key_fingerprint?: string | null;
}

export interface SyncSettings {
  repo_url: string | null;
  has_pat: boolean;
}

export interface GpgKey {
  id: string;
  fingerprint: string;
  uid: string;
  trust: string;
}

export interface GitLogEntry {
  hash: string;
  message: string;
  author: string;
  date: string;
}

export interface TotpCode {
  code: string;
  remaining_seconds: number;
  period: number;
}

export interface TotpInfo {
  issuer: string | null;
  account: string | null;
  uri: string;
}

export interface Vault {
  id: string;
  name: string;
  path: string;
}

export type View = "main" | "settings" | "gpg" | "sync";
