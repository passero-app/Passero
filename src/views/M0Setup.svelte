<script lang="ts">
  import { invoke } from "@tauri-apps/api/core";
  import { store, retrieve, remove } from "@impierce/tauri-plugin-keystore";
  import { checkStatus } from "@tauri-apps/plugin-biometric";

  const KEYSTORE_SERVICE = "app.passero";
  const KEYSTORE_USER = "passero-pgp-key";

  let { onUnlocked }: { onUnlocked: () => void } = $props();

  let busy = $state(false);
  let error = $state<string | null>(null);
  let status = $state<string | null>(null);
  let biometricAvailable = $state(false);

  let ready = $state(false);
  let hasKey = $state(false);
  let confirmingReset = $state(false);

  let name = $state("Passero User");
  let email = $state("");
  let fingerprint = $state<string | null>(null);

  let vaultUrl = $state("");
  let pat = $state("");

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
      await store(generated.armored);
      hasKey = true;
      status = "Key generated and saved to the Keychain.";
    } catch (e) {
      error = String(e);
    } finally {
      busy = false;
    }
  }

  async function cloneAndInit() {
    if (!fingerprint) {
      error = "Generate a key first.";
      return;
    }
    busy = true;
    error = null;
    status = null;
    try {
      await invoke("clone_store", { url: vaultUrl, token: pat });
      await invoke("init_store", { fingerprints: [fingerprint] });
      status = "Vault cloned and initialized.";
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

  async function resetDevice() {
    busy = true;
    error = null;
    status = null;
    try {
      try {
        await remove(KEYSTORE_SERVICE, KEYSTORE_USER);
      } catch (e) {
        console.warn("keystore remove failed", e);
      }
      await invoke("reset_device");
      hasKey = false;
      fingerprint = null;
      vaultUrl = "";
      pat = "";
      confirmingReset = false;
      status = "Device reset. Start fresh below.";
    } catch (e) {
      error = String(e);
    } finally {
      busy = false;
    }
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

    {#if !ready}
      <p class="text-center text-sm text-zinc-400">Loading…</p>
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

      <section class="space-y-3 border-t border-zinc-700 pt-4">
        {#if confirmingReset}
          <p class="text-sm text-red-300">
            Resetting deletes the device key. Any vault encrypted to this key
            becomes undecryptable. This cannot be undone.
          </p>
          <div class="flex gap-2">
            <button
              class="flex-1 rounded bg-red-700 px-3 py-2 text-sm font-medium disabled:opacity-50"
              disabled={busy}
              onclick={resetDevice}
            >
              Reset device
            </button>
            <button
              class="flex-1 rounded bg-zinc-700 px-3 py-2 text-sm font-medium disabled:opacity-50"
              disabled={busy}
              onclick={() => (confirmingReset = false)}
            >
              Cancel
            </button>
          </div>
        {:else}
          <button
            class="w-full rounded border border-red-800 px-3 py-2 text-sm font-medium text-red-300 disabled:opacity-50"
            disabled={busy}
            onclick={() => (confirmingReset = true)}
          >
            Reset device
          </button>
        {/if}
      </section>
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
        {#if fingerprint}
          <p class="break-all text-xs text-zinc-400">Fingerprint: {fingerprint}</p>
        {/if}
      </section>

      <section class="space-y-3">
        <h2 class="text-sm font-medium text-zinc-300">Test vault (optional)</h2>
        <input
          class="w-full rounded bg-zinc-800 px-3 py-2 text-sm"
          placeholder="HTTPS clone URL"
          bind:value={vaultUrl}
        />
        <input
          class="w-full rounded bg-zinc-800 px-3 py-2 text-sm"
          placeholder="GitHub PAT"
          type="password"
          bind:value={pat}
        />
        <button
          class="w-full rounded bg-zinc-700 px-3 py-2 text-sm font-medium disabled:opacity-50"
          disabled={busy || !vaultUrl || !pat}
          onclick={cloneAndInit}
        >
          Clone and initialize
        </button>
      </section>
    {/if}
  </div>
</div>
