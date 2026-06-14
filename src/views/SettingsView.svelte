<script lang="ts">
  import { onMount } from "svelte";
  import { settings } from "$lib/stores/settings.svelte";
  import { ui } from "$lib/stores/ui.svelte";
  import { passwords } from "$lib/stores/passwords.svelte";
  import {
    getSyncSettings,
    setSyncSettings,
    cloneStore,
    getPasswordStorePath,
    getConfig,
    gitPull,
    gitPush,
  } from "$lib/commands";

  let repoUrl = $state("");
  let patInput = $state("");
  let hasPat = $state(false);
  let storePath = $state("");
  let fingerprint = $state<string | null>(null);
  let syncBusy = $state(false);
  let syncError = $state<string | null>(null);

  async function handleSave() {
    await settings.save();
    ui.notify("Settings saved");
  }

  async function loadSync() {
    syncError = null;
    try {
      const s = await getSyncSettings();
      repoUrl = s.repo_url ?? "";
      hasPat = s.has_pat;
      storePath = await getPasswordStorePath();
      const cfg = await getConfig();
      fingerprint = cfg.device_key_fingerprint ?? null;
    } catch (e) {
      syncError = String(e);
    }
  }

  async function saveSync() {
    syncBusy = true;
    syncError = null;
    try {
      await setSyncSettings(repoUrl || null, patInput || null);
      patInput = "";
      await loadSync();
      ui.notify("Sync settings saved");
    } catch (e) {
      syncError = String(e);
    } finally {
      syncBusy = false;
    }
  }

  async function cloneOrSync() {
    syncBusy = true;
    syncError = null;
    try {
      await setSyncSettings(repoUrl || null, patInput || null);
      patInput = "";
      if (passwords.initialized) {
        await gitPull();
        await gitPush();
        ui.notify("Synced with remote");
      } else {
        const s = await getSyncSettings();
        if (!s.repo_url) {
          throw new Error("Set a repository URL first.");
        }
        await cloneStore(s.repo_url, "");
        ui.notify("Repository cloned");
      }
      await passwords.refresh();
      await loadSync();
    } catch (e) {
      syncError = String(e);
    } finally {
      syncBusy = false;
    }
  }

  onMount(loadSync);
</script>

<div class="h-8 w-full shrink-0" data-tauri-drag-region></div>
<div class="p-6 max-w-2xl overflow-y-auto">
  <h2 class="text-lg font-medium mb-6">Settings</h2>

  {#if settings.error}
    <div class="text-red-400 text-sm mb-4">{settings.error}</div>
  {/if}

  <div class="space-y-6">
    <section class="space-y-3">
      <h3 class="text-sm font-medium text-zinc-400 uppercase tracking-wide">Sync / Vault</h3>
      {#if syncError}
        <div class="text-red-400 text-sm">{syncError}</div>
      {/if}
      <div>
        <label class="text-xs text-zinc-500" for="repo-url">Repository URL</label>
        <input
          id="repo-url"
          type="text"
          bind:value={repoUrl}
          placeholder="https://github.com/you/store.git"
          class="mt-1 w-full bg-zinc-800 border border-zinc-700 rounded px-3 py-2 text-sm text-zinc-100 placeholder:text-zinc-500 focus:outline-none focus:border-zinc-500"
        />
      </div>
      <div>
        <label class="text-xs text-zinc-500" for="pat">GitHub PAT</label>
        <input
          id="pat"
          type="password"
          bind:value={patInput}
          placeholder={hasPat ? "(set)" : "ghp_…"}
          class="mt-1 w-full bg-zinc-800 border border-zinc-700 rounded px-3 py-2 text-sm text-zinc-100 placeholder:text-zinc-500 focus:outline-none focus:border-zinc-500"
        />
      </div>
      <div class="flex gap-2">
        <button
          class="px-4 py-2 text-sm bg-zinc-700 text-zinc-100 rounded hover:bg-zinc-600 disabled:opacity-50 transition-colors"
          disabled={syncBusy}
          onclick={saveSync}
        >
          Save
        </button>
        <button
          class="px-4 py-2 text-sm bg-blue-600 text-white rounded hover:bg-blue-500 disabled:opacity-50 transition-colors"
          disabled={syncBusy || !repoUrl}
          onclick={cloneOrSync}
        >
          {passwords.initialized ? "Pull / Push" : "Clone"}
        </button>
      </div>
      <div class="text-xs text-zinc-500 space-y-1 break-all">
        {#if fingerprint}
          <div>Device key: {fingerprint}</div>
        {/if}
        {#if storePath}
          <div>Store path: {storePath}</div>
        {/if}
      </div>
    </section>

    <section class="space-y-3">
      <h3 class="text-sm font-medium text-zinc-400 uppercase tracking-wide">Tool Paths</h3>
      <div>
        <label class="text-xs text-zinc-500" for="pass-binary">pass binary</label>
        <input
          id="pass-binary"
          type="text"
          bind:value={settings.config.pass_binary}
          placeholder="pass (default)"
          class="mt-1 w-full bg-zinc-800 border border-zinc-700 rounded px-3 py-2 text-sm text-zinc-100 placeholder:text-zinc-500 focus:outline-none focus:border-zinc-500"
        />
      </div>
      <div>
        <label class="text-xs text-zinc-500" for="gpg-binary">gpg binary</label>
        <input
          id="gpg-binary"
          type="text"
          bind:value={settings.config.gpg_binary}
          placeholder="gpg (default)"
          class="mt-1 w-full bg-zinc-800 border border-zinc-700 rounded px-3 py-2 text-sm text-zinc-100 placeholder:text-zinc-500 focus:outline-none focus:border-zinc-500"
        />
      </div>
      <div>
        <label class="text-xs text-zinc-500" for="git-binary">git binary</label>
        <input
          id="git-binary"
          type="text"
          bind:value={settings.config.git_binary}
          placeholder="git (default)"
          class="mt-1 w-full bg-zinc-800 border border-zinc-700 rounded px-3 py-2 text-sm text-zinc-100 placeholder:text-zinc-500 focus:outline-none focus:border-zinc-500"
        />
      </div>
    </section>

    <section class="space-y-3">
      <h3 class="text-sm font-medium text-zinc-400 uppercase tracking-wide">Password Store</h3>
      <div>
        <label class="text-xs text-zinc-500" for="store-dir">Default store directory</label>
        <input
          id="store-dir"
          type="text"
          bind:value={settings.config.password_store_dir}
          placeholder="~/.password-store (default)"
          class="mt-1 w-full bg-zinc-800 border border-zinc-700 rounded px-3 py-2 text-sm text-zinc-100 placeholder:text-zinc-500 focus:outline-none focus:border-zinc-500"
        />
      </div>
      <div>
        <label class="text-xs text-zinc-500" for="clipboard-timeout">Clipboard timeout (seconds)</label>
        <input
          id="clipboard-timeout"
          type="number"
          bind:value={settings.config.clipboard_timeout}
          min={0}
          max={300}
          class="mt-1 w-full bg-zinc-800 border border-zinc-700 rounded px-3 py-2 text-sm text-zinc-100 focus:outline-none focus:border-zinc-500"
        />
      </div>
    </section>

    <button
      class="px-4 py-2 text-sm bg-zinc-100 text-zinc-900 rounded hover:bg-white transition-colors"
      onclick={handleSave}
    >
      Save Settings
    </button>
  </div>
</div>
