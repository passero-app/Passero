<script lang="ts">
  import { invoke } from "@tauri-apps/api/core";
  import { openUrl } from "@tauri-apps/plugin-opener";

  let {
    onAuthorized,
    onCloned,
  }: { onAuthorized?: () => void; onCloned?: () => void } = $props();

  let busy = $state(false);
  let status = $state<string | null>(null);
  let ghState = $state<"idle" | "code" | "authorized" | "failed">("idle");
  let ghError = $state<string | null>(null);
  let userCode = $state("");
  let verificationUri = $state("");
  let codeCopied = $state(false);
  let repoUrl = $state("");
  let repos = $state<{ fullName: string; cloneUrl: string }[]>([]);
  let reposLoading = $state(false);
  let reposError = $state<string | null>(null);
  let showAdvanced = $state(false);
  let patInput = $state("");
  let pollTimer: ReturnType<typeof setInterval> | null = null;
  let pollMs = 0;

  $effect(() => {
    refreshSync();
    return () => stopPolling();
  });

  async function refreshSync() {
    try {
      const s = await invoke<{ repo_url?: string; has_pat: boolean }>(
        "get_sync_settings",
      );
      repoUrl = s.repo_url ?? "";
      if (s.has_pat) {
        ghState = "authorized";
        loadRepos();
      }
    } catch (e) {
      ghError = String(e);
    }
  }

  async function loadRepos() {
    reposLoading = true;
    reposError = null;
    try {
      repos = await invoke<{ fullName: string; cloneUrl: string }[]>(
        "github_list_repos",
      );
    } catch (e) {
      repos = [];
      reposError = String(e);
    } finally {
      reposLoading = false;
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
        onAuthorized?.();
        loadRepos();
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
      verificationUri = r.verificationUri;
      codeCopied = false;
      ghState = "code";
      pollMs = (r.interval + 1) * 1000;
      schedulePoll();
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
      repos = [];
      reposError = null;
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
      onCloned?.();
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
      onAuthorized?.();
      loadRepos();
    } catch (e) {
      ghError = String(e);
    } finally {
      busy = false;
    }
  }
</script>

<section class="space-y-3 border-t border-zinc-700 pt-4">
  <h2 class="text-sm font-medium text-zinc-300">GitHub sync</h2>
  {#if ghError}
    <div class="rounded bg-red-900/50 px-3 py-2 text-sm text-red-200">{ghError}</div>
  {/if}
  {#if status}
    <div class="rounded bg-green-900/50 px-3 py-2 text-sm text-green-200">{status}</div>
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
    <button
      class="w-full rounded bg-zinc-100 px-3 py-2 text-sm font-medium text-zinc-900 disabled:opacity-50"
      onclick={async () => {
        await navigator.clipboard.writeText(userCode);
        codeCopied = true;
      }}
    >
      {codeCopied ? "Copied ✓" : "Copy code"}
    </button>
    <button
      class="w-full rounded bg-emerald-600 px-3 py-2 text-sm font-medium disabled:opacity-50"
      onclick={() => openUrl(verificationUri)}
    >
      Open github.com to sign in
    </button>
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
    {#if reposLoading}
      <p class="text-sm text-zinc-400">Loading repositories…</p>
    {:else if repos.length > 0}
      <div class="space-y-2">
        {#each repos as repo (repo.fullName)}
          <button
            class="w-full rounded px-3 py-2 text-left text-sm font-medium disabled:opacity-50 {repoUrl ===
            repo.cloneUrl
              ? 'border border-emerald-500 bg-emerald-900/40'
              : 'bg-zinc-700'}"
            disabled={busy}
            onclick={() => (repoUrl = repo.cloneUrl)}
          >
            {repo.fullName}
          </button>
        {/each}
      </div>
      <p class="text-xs text-zinc-500">or enter a URL manually</p>
    {:else if reposError}
      <p class="text-xs text-zinc-500">
        Couldn't list repositories: {reposError}
      </p>
    {/if}
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
