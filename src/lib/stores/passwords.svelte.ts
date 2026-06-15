import { listPasswords, showPassword, getStoreGpgId } from "$lib/commands";
import type { PasswordEntry } from "$lib/types";
import { parsePasswordContent } from "$lib/utils";

class PasswordStore {
  tree = $state<PasswordEntry[]>([]);
  selectedPath = $state<string | null>(null);
  selectedContent = $state<string | null>(null);
  loading = $state(false);
  error = $state<string | null>(null);
  initialized = $state(false);

  parsedContent = $derived.by(() => {
    if (!this.selectedContent) return null;
    return parsePasswordContent(this.selectedContent);
  });

  async refresh() {
    this.loading = true;
    this.error = null;
    try {
      this.tree = await listPasswords();
    } catch (e) {
      this.error = String(e);
    } finally {
      this.loading = false;
    }
    try {
      const gpgId = await getStoreGpgId();
      this.initialized = gpgId.trim().length > 0;
    } catch {
      this.initialized = false;
    }
  }

  async select(path: string) {
    this.selectedPath = path;
    this.selectedContent = null;
    this.error = null;
    try {
      this.selectedContent = await showPassword(path);
    } catch (e) {
      this.error = String(e);
    }
  }

  deselect() {
    this.selectedPath = null;
    this.selectedContent = null;
  }
}

export const passwords = new PasswordStore();
