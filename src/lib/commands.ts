import { invoke } from "@tauri-apps/api/core";
import type {
  PasswordEntry,
  AppConfig,
  GpgKey,
  GitLogEntry,
  TotpCode,
  TotpInfo,
  Vault,
} from "./types";

export async function listPasswords(): Promise<PasswordEntry[]> {
  return invoke("list_passwords");
}

export async function showPassword(path: string): Promise<string> {
  return invoke("show_password", { path });
}

export async function insertPassword(
  path: string,
  content: string,
): Promise<void> {
  return invoke("insert_password", { path, content });
}

export async function editPassword(
  path: string,
  content: string,
): Promise<void> {
  return invoke("edit_password", { path, content });
}

export async function deletePassword(path: string): Promise<void> {
  return invoke("delete_password", { path });
}

export async function generatePassword(
  path: string,
  length: number,
  symbols: boolean,
): Promise<string> {
  return invoke("generate_password", { path, length, symbols });
}

export async function copyPassword(path: string): Promise<void> {
  return invoke("copy_password", { path });
}

export async function listGpgKeys(): Promise<GpgKey[]> {
  return invoke("list_gpg_keys");
}

export async function getStoreGpgId(): Promise<string> {
  return invoke("get_store_gpg_id");
}

export async function gitPull(): Promise<string> {
  return invoke("git_pull");
}

export async function gitPush(): Promise<string> {
  return invoke("git_push");
}

export async function gitLog(count?: number): Promise<GitLogEntry[]> {
  return invoke("git_log", { count });
}

export async function gitClone(
  url: string,
  path?: string,
): Promise<void> {
  return invoke("git_clone", { url, path });
}

export async function getConfig(): Promise<AppConfig> {
  return invoke("get_config");
}

export async function setConfig(config: AppConfig): Promise<void> {
  return invoke("set_config", { config });
}

export async function getPasswordStorePath(): Promise<string> {
  return invoke("get_password_store_path");
}

export async function listRecipients(): Promise<string[]> {
  return invoke("list_recipients");
}

export async function addRecipient(gpgId: string): Promise<string[]> {
  return invoke("add_recipient", { gpgId });
}

export async function removeRecipient(gpgId: string): Promise<string[]> {
  return invoke("remove_recipient", { gpgId });
}

export async function initPasswordStore(gpgIds: string[]): Promise<void> {
  return invoke("init_password_store", { gpgIds });
}

export async function listGpgSecretKeys(): Promise<GpgKey[]> {
  return invoke("list_gpg_secret_keys");
}

export async function generateGpgKey(params: {
  name: string;
  email: string;
  passphrase?: string;
  key_type?: string;
  key_length?: number;
}): Promise<string> {
  return invoke("generate_gpg_key", { params });
}

export async function importGpgKey(keyData: string): Promise<string> {
  return invoke("import_gpg_key", { keyData });
}

export async function importGpgKeyFromKeyserver(
  keyId: string,
  keyserver?: string,
): Promise<string> {
  return invoke("import_gpg_key_from_keyserver", { keyId, keyserver });
}

export async function exportGpgKey(
  keyId: string,
  secret: boolean = false,
): Promise<string> {
  return invoke("export_gpg_key", { keyId, secret });
}

export async function publishGpgKey(
  keyId: string,
  keyserver?: string,
): Promise<string> {
  return invoke("publish_gpg_key", { keyId, keyserver });
}

export async function setGpgKeyTrust(
  fingerprint: string,
  trustLevel: number,
): Promise<void> {
  return invoke("set_gpg_key_trust", { fingerprint, trustLevel });
}

export async function deleteGpgKey(
  fingerprint: string,
  secret: boolean = false,
): Promise<void> {
  return invoke("delete_gpg_key", { fingerprint, secret });
}

export async function resolveGpgKeys(
  keyIds: string[],
): Promise<(GpgKey | null)[]> {
  return invoke("resolve_gpg_keys", { keyIds });
}

export async function searchGpgKeyserver(
  query: string,
  keyserver?: string,
): Promise<GpgKey[]> {
  return invoke("search_gpg_keyserver", { query, keyserver });
}

export async function getTotp(path: string): Promise<TotpCode> {
  return invoke("get_totp", { path });
}

export async function getTotpInfo(path: string): Promise<TotpInfo | null> {
  return invoke("get_totp_info", { path });
}

export async function listVaults(): Promise<Vault[]> {
  return invoke("list_vaults");
}

export async function addVault(
  name: string,
  path: string,
): Promise<Vault> {
  return invoke("add_vault", { name, path });
}

export async function removeVault(id: string): Promise<void> {
  return invoke("remove_vault", { id });
}

export async function setActiveVault(id: string | null): Promise<void> {
  return invoke("set_active_vault", { id });
}

export async function decodeQrImage(imagePath: string): Promise<string> {
  return invoke("decode_qr_image", { imagePath });
}

export async function importTotpFromQr(
  path: string,
  imagePath: string,
): Promise<TotpCode> {
  return invoke("import_totp_from_qr", { path, imagePath });
}

export async function insertTotp(
  path: string,
  secret: string,
  issuer?: string,
  account?: string,
): Promise<void> {
  return invoke("insert_totp", { path, secret, issuer, account });
}
