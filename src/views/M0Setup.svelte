<script lang="ts">
  import { invoke } from "@tauri-apps/api/core";
  import { store, retrieve } from "../lib/keystore";
  import { checkStatus } from "@tauri-apps/plugin-biometric";
  import GithubSync from "../components/GithubSync.svelte";
  import ResetDevice from "../components/ResetDevice.svelte";

  const KEYSTORE_SERVICE = "app.passero";
  const KEYSTORE_USER = "passero-pgp-key";

  let { onUnlocked }: { onUnlocked: () => void } = $props();

  let busy = $state(false);
  let error = $state<string | null>(null);
  let status = $state<string | null>(null);
  let biometricAvailable = $state(false);

  let ready = $state(false);
  let hasKey = $state(false);
  let syncReady = $state(false);
  let ghAuthorized = $state(false);
  let vaultCloned = $state(false);

  let name = $state("Passero User");
  let email = $state("");
  let fingerprint = $state<string | null>(null);
  let showImport = $state(false);
  let importArmored = $state("");

  let fullySetUp = $derived(hasKey && ghAuthorized && vaultCloned);

  $effect(() => {
    checkStatus()
      .then((s) => {
        biometricAvailable = s.isAvailable;
      })
      .catch(() => {
        biometricAvailable = false;
      });
  });

  $effect(() => {
    refreshStatus();
  });

  $effect(() => {
    checkSyncState();
  });

  async function refreshStatus() {
    try {
      const s = await invoke<{ has_key: boolean; fingerprint: string | null }>(
        "device_key_status",
      );
      hasKey = s.has_key;
      fingerprint = s.fingerprint;
    } catch (e) {
      error = String(e);
    } finally {
      ready = true;
    }
  }

  async function checkSyncState() {
    try {
      const [s, isRepo] = await Promise.all([
        invoke<{ repo_url?: string; has_pat: boolean }>("get_sync_settings"),
        invoke<boolean>("store_is_repo"),
      ]);
      ghAuthorized = s.has_pat;
      vaultCloned = isRepo;
    } catch (e) {
      error = String(e);
    } finally {
      syncReady = true;
    }
  }

  async function generate() {
    busy = true;
    error = null;
    status = null;
    try {
      const userId = email ? `${name} <${email}>` : name;
      const generated = await invoke<{ fingerprint: string; armored: string }>(
        "generate_in_app_key",
        { userId },
      );
      fingerprint = generated.fingerprint;
      await store(KEYSTORE_SERVICE, KEYSTORE_USER, generated.armored);
      hasKey = true;
      status = "Key generated and saved to the Keychain.";
    } catch (e) {
      error = String(e);
    } finally {
      busy = false;
    }
  }

  async function importKey() {
    busy = true;
    error = null;
    status = null;
    try {
      const armored = importArmored;
      const r = await invoke<{ fingerprint: string }>("validate_secret_key", {
        armored,
      });
      await store(KEYSTORE_SERVICE, KEYSTORE_USER, armored);
      await invoke("load_key", { armored });
      importArmored = "";
      showImport = false;
      fingerprint = r.fingerprint;
      hasKey = true;
      status = "Key imported and saved to the Keychain.";
    } catch (e) {
      error = String(e);
    } finally {
      busy = false;
    }
  }

  async function unlock() {
    busy = true;
    error = null;
    status = null;
    try {
      const armored = await retrieve(KEYSTORE_SERVICE, KEYSTORE_USER);
      if (!armored) {
        error = "No key found in the Keychain. Reset the device to start over.";
        return;
      }
      await invoke("load_key", { armored });
      onUnlocked();
    } catch (e) {
      error = String(e);
    } finally {
      busy = false;
    }
  }

  function handleReset() {
    hasKey = false;
    fingerprint = null;
    vaultCloned = false;
    ghAuthorized = false;
    status = "Device reset. Start fresh below.";
  }
</script>

<div class="flex min-h-screen flex-col items-center justify-center gap-6 bg-zinc-900 p-6 text-zinc-100">
  <div class="w-full max-w-sm space-y-6">
    <header class="space-y-1 text-center">
      <h1 class="text-xl font-semibold">Passero (M0)</h1>
      <p class="text-sm text-zinc-400">
        Biometric availability: {biometricAvailable ? "yes" : "no"}
      </p>
    </header>

    {#if error}
      <div class="rounded bg-red-900/50 px-3 py-2 text-sm text-red-200">{error}</div>
    {/if}
    {#if status}
      <div class="rounded bg-green-900/50 px-3 py-2 text-sm text-green-200">{status}</div>
    {/if}

    {#if !ready || !syncReady}
      <p class="text-center text-sm text-zinc-400">Loading…</p>
    {:else if fullySetUp}
      <section class="space-y-3">
        <button
          class="w-full rounded bg-emerald-600 px-3 py-2 text-sm font-medium disabled:opacity-50"
          disabled={busy}
          onclick={unlock}
        >
          Unlock with FaceID
        </button>
      </section>
    {:else if hasKey}
      <section class="space-y-3">
        <h2 class="text-sm font-medium text-zinc-300">Unlock</h2>
        <button
          class="w-full rounded bg-emerald-600 px-3 py-2 text-sm font-medium disabled:opacity-50"
          disabled={busy}
          onclick={unlock}
        >
          Unlock with FaceID
        </button>
        {#if fingerprint}
          <p class="break-all text-xs text-zinc-400">Device key: {fingerprint}</p>
        {/if}
      </section>

      <GithubSync
        onAuthorized={() => (ghAuthorized = true)}
        onCloned={() => (vaultCloned = true)}
      />

      <ResetDevice onReset={handleReset} />
    {:else}
      <section class="space-y-3">
        <h2 class="text-sm font-medium text-zinc-300">First run</h2>
        <input
          class="w-full rounded bg-zinc-800 px-3 py-2 text-sm"
          placeholder="Name"
          bind:value={name}
        />
        <input
          class="w-full rounded bg-zinc-800 px-3 py-2 text-sm"
          placeholder="Email (optional)"
          bind:value={email}
        />
        <button
          class="w-full rounded bg-blue-600 px-3 py-2 text-sm font-medium disabled:opacity-50"
          disabled={busy}
          onclick={generate}
        >
          Generate key
        </button>
        <button
          class="text-xs text-zinc-500 underline"
          onclick={() => (showImport = !showImport)}
        >
          Import existing key
        </button>
        {#if showImport}
          <textarea
            class="w-full rounded bg-zinc-800 px-3 py-2 font-mono text-xs"
            rows="6"
            placeholder="-----BEGIN PGP PRIVATE KEY BLOCK-----"
            bind:value={importArmored}
          ></textarea>
          <button
            class="w-full rounded bg-zinc-700 px-3 py-2 text-sm font-medium disabled:opacity-50"
            disabled={busy || !importArmored.trim()}
            onclick={importKey}
          >
            Import
          </button>
        {/if}
        {#if fingerprint}
          <p class="break-all text-xs text-zinc-400">Fingerprint: {fingerprint}</p>
        {/if}
      </section>
    {/if}
  </div>
</div>
