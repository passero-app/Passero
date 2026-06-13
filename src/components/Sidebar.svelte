<script lang="ts">
  import { ui } from "$lib/stores/ui.svelte";
  import { gitPull, gitPush, gitLog } from "$lib/commands";
  import { passwords } from "$lib/stores/passwords.svelte";
  import { settings } from "$lib/stores/settings.svelte";
  import type { GitLogEntry } from "$lib/types";
  import VaultSwitcher from "./VaultSwitcher.svelte";
  import StoreRecipients from "./StoreRecipients.svelte";

  let logEntries = $state<GitLogEntry[]>([]);
  let logExpanded = $state(false);

  let activeId = $derived(settings.config.active_vault_id);
  $effect(() => {
    void activeId;
    refreshLog();
  });

  async function refreshLog() {
    try {
      logEntries = await gitLog(10);
    } catch {
      logEntries = [];
    }
  }

  async function handleSync(action: "pull" | "push") {
    try {
      if (action === "pull") {
        await gitPull();
        await passwords.refresh();
        ui.notify("Pulled from remote");
      } else {
        await gitPush();
        ui.notify("Pushed to remote");
      }
      await refreshLog();
    } catch (e) {
      ui.notify(String(e), "error");
    }
  }
</script>

<aside class="w-56 bg-zinc-950 border-r border-zinc-800 flex flex-col">
  <div class="p-4 pt-10 border-b border-zinc-800" data-tauri-drag-region>
    <h1 class="text-lg font-semibold tracking-tight">Passero</h1>
    <a href="https://passero.app" target="_blank" class="text-xs text-zinc-500 hover:text-zinc-300 transition-colors">passero.app</a>
  </div>

  <nav class="p-2 space-y-1">
    <button
      class="w-full text-left px-3 py-2 rounded text-sm transition-colors {ui.currentView ===
      'main'
        ? 'bg-zinc-800 text-white'
        : 'text-zinc-400 hover:text-white hover:bg-zinc-800/50'}"
      onclick={() => ui.navigate("main")}
    >
      Passwords
    </button>
  </nav>

  <div class="p-2 border-t border-zinc-800">
    <VaultSwitcher />
  </div>

  <div class="px-2 pb-2">
    <StoreRecipients />
  </div>

  <div class="px-2 pb-2 flex gap-1">
    <button
      class="flex-1 text-center px-3 py-1.5 rounded text-sm text-zinc-400 hover:text-white hover:bg-zinc-800/50 transition-colors"
      onclick={() => handleSync("pull")}
    >
      Pull
    </button>
    <button
      class="flex-1 text-center px-3 py-1.5 rounded text-sm text-zinc-400 hover:text-white hover:bg-zinc-800/50 transition-colors"
      onclick={() => handleSync("push")}
    >
      Push
    </button>
  </div>

  {#if logEntries.length > 0}
    <div class="px-2 pb-2">
      <button
        class="w-full px-3 py-1 text-xs text-zinc-500 uppercase tracking-wide flex items-center justify-between hover:text-zinc-300 transition-colors"
        onclick={() => (logExpanded = !logExpanded)}
      >
        <span>History</span>
        <span class="text-[10px]">{logExpanded ? "▲" : "▼"}</span>
      </button>
      {#if logExpanded}
        <div class="mt-1 space-y-0.5 max-h-60 overflow-y-auto">
          {#each logEntries as entry}
            <div class="px-2 py-1 rounded hover:bg-zinc-800/50" title="{entry.author} — {entry.date}">
              <div class="text-[11px] text-zinc-300 truncate">{entry.message}</div>
              <div class="text-[10px] text-zinc-600">{entry.date.slice(0, 10)}</div>
            </div>
          {/each}
        </div>
      {/if}
    </div>
  {/if}

  <div class="flex-1"></div>

  <nav class="p-2 border-t border-zinc-800 space-y-1">
    <button
      class="w-full text-left px-3 py-2 rounded text-sm transition-colors {ui.currentView ===
      'gpg'
        ? 'bg-zinc-800 text-white'
        : 'text-zinc-400 hover:text-white hover:bg-zinc-800/50'}"
      onclick={() => ui.navigate("gpg")}
    >
      GPG Keys
    </button>
    <button
      class="w-full text-left px-3 py-2 rounded text-sm transition-colors {ui.currentView ===
      'settings'
        ? 'bg-zinc-800 text-white'
        : 'text-zinc-400 hover:text-white hover:bg-zinc-800/50'}"
      onclick={() => ui.navigate("settings")}
    >
      Settings
    </button>
  </nav>
</aside>
