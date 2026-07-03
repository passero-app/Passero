<script lang="ts">
  import { invoke } from "@tauri-apps/api/core";
  import { open } from "@tauri-apps/plugin-shell";
  import { store, retrieve, remove } from "../lib/keystore";
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

  let ghState = $state<"idle" | "code" | "authorized" | "failed">("idle");
  let ghError = $state<string | null>(null);
  let userCode = $state("");
  let repoUrl = $state("");
  let showAdvanced = $state(false);
  let patInput = $state("");
  let pollTimer: ReturnType<typeof setInterval> | null = null;
  let pollMs = 0;

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
    refreshSync();
    return () => stopPolling();
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
      await store(KEYSTORE_SERVICE, KEYSTORE_USER, generated.armored);
      hasKey = true;
      status = "Key generated and saved to the Keychain.";
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

  async function refreshSync() {
    try {
      const s = await invoke<{ repo_url?: string; has_pat: boolean }>(
        "get_sync_settings",
      );
      repoUrl = s.repo_url ?? "";
      if (s.has_pat) {
        ghState = "authorized";
      }
    } catch (e) {
      ghError = String(e);
    }
  }

  function stopPolling() {
    if (pollTimer) {
      clearInterval(pollTimer);
      pollTimer = null;
    }
  }

  function schedulePoll() {
    stopPolling();
    pollTimer = setInterval(pollLogin, pollMs);
  }

  async function pollLogin() {
    try {
      const r = await invoke<string>("github_login_poll");
      if (r === "slow_down") {
        pollMs += 5000;
        schedulePoll();
      } else if (r === "authorized") {
        stopPolling();
        ghState = "authorized";
        status = "GitHub connected.";
      }
    } catch (e) {
      stopPolling();
      ghState = "failed";
      ghError = String(e);
    }
  }

  async function githubSignIn() {
    busy = true;
    ghError = null;
    status = null;
    try {
      const r = await invoke<{
        userCode: string;
        verificationUri: string;
        interval: number;
      }>("github_login_start");
      userCode = r.userCode;
      ghState = "code";
      pollMs = (r.interval + 1) * 1000;
      schedulePoll();
      await open(r.verificationUri);
    } catch (e) {
      stopPolling();
      ghState = "failed";
      ghError = String(e);
    } finally {
      busy = false;
    }
  }

  async function githubSignOut() {
    busy = true;
    ghError = null;
    status = null;
    try {
      stopPolling();
      await invoke("github_logout");
      ghState = "idle";
      userCode = "";
      status = "Signed out of GitHub.";
    } catch (e) {
      ghError = String(e);
    } finally {
      busy = false;
    }
  }

  async function cloneVault() {
    busy = true;
    ghError = null;
    status = null;
    try {
      await invoke("set_sync_settings", { repoUrl: repoUrl.trim(), pat: null });
      await invoke("clone_store", { url: repoUrl.trim(), token: "" });
      status = "Vault cloned.";
    } catch (e) {
      ghError = String(e);
    } finally {
      busy = false;
    }
  }

  async function saveToken() {
    busy = true;
    ghError = null;
    status = null;
    try {
      stopPolling();
      await invoke("set_sync_settings", {
        repoUrl: repoUrl.trim() || null,
        pat: patInput,
      });
      patInput = "";
      showAdvanced = false;
      ghState = "authorized";
      status = "Token saved.";
    } catch (e) {
      ghError = String(e);
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
        <h2 class="text-sm font-medium text-zinc-300">GitHub sync</h2>
        {#if ghError}
          <div class="rounded bg-red-900/50 px-3 py-2 text-sm text-red-200">{ghError}</div>
        {/if}
        {#if ghState === "idle"}
          <button
            class="w-full rounded bg-zinc-100 px-3 py-2 text-sm font-medium text-zinc-900 disabled:opacity-50"
            disabled={busy}
            onclick={githubSignIn}
          >
            Sign in with GitHub
          </button>
        {:else if ghState === "code"}
          <p class="text-sm text-zinc-400">
            Enter this code on github.com/login/device:
          </p>
          <p class="text-center font-mono text-3xl font-bold tracking-widest">
            {userCode}
          </p>
          <p class="text-center text-sm text-zinc-400">Waiting for authorization…</p>
        {:else if ghState === "failed"}
          <button
            class="w-full rounded bg-zinc-100 px-3 py-2 text-sm font-medium text-zinc-900 disabled:opacity-50"
            disabled={busy}
            onclick={githubSignIn}
          >
            Retry GitHub sign-in
          </button>
        {:else}
          <div class="flex items-center justify-between">
            <p class="text-sm text-green-300">Signed in to GitHub ✓</p>
            <button
              class="text-xs text-zinc-500 underline disabled:opacity-50"
              disabled={busy}
              onclick={githubSignOut}
            >
              Sign out
            </button>
          </div>
          <input
            class="w-full rounded bg-zinc-800 px-3 py-2 text-sm"
            placeholder="https://github.com/you/store.git"
            bind:value={repoUrl}
          />
          <button
            class="w-full rounded bg-blue-600 px-3 py-2 text-sm font-medium disabled:opacity-50"
            disabled={busy || !repoUrl.trim()}
            onclick={cloneVault}
          >
            Clone vault
          </button>
        {/if}
        {#if ghState !== "authorized"}
          <button
            class="text-xs text-zinc-500 underline"
            onclick={() => (showAdvanced = !showAdvanced)}
          >
            Advanced: use a token instead
          </button>
          {#if showAdvanced}
            <input
              class="w-full rounded bg-zinc-800 px-3 py-2 text-sm"
              type="password"
              placeholder="ghp_…"
              bind:value={patInput}
            />
            <button
              class="w-full rounded bg-zinc-700 px-3 py-2 text-sm font-medium disabled:opacity-50"
              disabled={busy || !patInput.trim()}
              onclick={saveToken}
            >
              Save token
            </button>
          {/if}
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
    {/if}
  </div>
</div>
