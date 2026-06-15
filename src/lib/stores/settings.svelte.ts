import { getConfig, setConfig } from "$lib/commands";
import type { AppConfig } from "$lib/types";

class SettingsStore {
  config = $state<AppConfig>({
    pass_binary: null,
    gpg_binary: null,
    git_binary: null,
    password_store_dir: null,
    clipboard_timeout: 45,
    vaults: [],
    active_vault_id: null,
    repo_url: null,
  });
  loading = $state(false);
  error = $state<string | null>(null);

  async load() {
    this.loading = true;
    this.error = null;
    try {
      this.config = await getConfig();
    } catch (e) {
      this.error = String(e);
    } finally {
      this.loading = false;
    }
  }

  async save() {
    this.error = null;
    try {
      await setConfig(this.config);
    } catch (e) {
      this.error = String(e);
    }
  }
}

export const settings = new SettingsStore();
