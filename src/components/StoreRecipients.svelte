<script lang="ts">
  import {
    listRecipients,
    addRecipient,
    removeRecipient,
    resolveGpgKeys,
    listGpgSecretKeys,
    listGpgKeys,
    searchGpgKeyserver,
    importGpgKeyFromKeyserver,
  } from "$lib/commands";
  import { passwords } from "$lib/stores/passwords.svelte";
  import { settings } from "$lib/stores/settings.svelte";
  import { ui } from "$lib/stores/ui.svelte";
  import type { GpgKey } from "$lib/types";

  interface ResolvedRecipient {
    rawId: string;
    key: GpgKey | null;
    hasSecret: boolean;
  }

  let recipients = $state<ResolvedRecipient[]>([]);
  let expanded = $state(false);
  let busy = $state(false);

  // Add key dialog
  let showAddDialog = $state(false);
  let searchQuery = $state("");
  let searching = $state(false);
  let localResults = $state<GpgKey[]>([]);
  let keyserverResults = $state<GpgKey[]>([]);
  let importing = $state<string | null>(null);

  // Dynamic label: "Keys" if all recipients have secret keys, "Sharing" if any don't
  let label = $derived(() => {
    if (recipients.length === 0) return "Keys";
    const allOwned = recipients.every((r) => r.hasSecret);
    return allOwned ? "Keys" : "Sharing";
  });

  let activeId = $derived(settings.config.active_vault_id);
  $effect(() => {
    void activeId;
    refresh();
  });

  async function refresh() {
    try {
      const rawIds = await listRecipients();
      if (rawIds.length === 0) {
        recipients = [];
        return;
      }
      const [resolved, secretKeys] = await Promise.all([
        resolveGpgKeys(rawIds),
        listGpgSecretKeys(),
      ]);
      const secretIds = new Set(secretKeys.map((k) => k.id));
      recipients = rawIds.map((rawId, i) => ({
        rawId,
        key: resolved[i] ?? null,
        hasSecret: resolved[i] ? secretIds.has(resolved[i]!.id) : false,
      }));
    } catch {
      recipients = [];
    }
  }

  async function handleRemove(rawId: string) {
    if (!confirm(`Remove this key? The store will be re-encrypted.`)) return;
    busy = true;
    try {
      await removeRecipient(rawId);
      await refresh();
      ui.notify("Key removed, store re-encrypted");
      await passwords.refresh();
    } catch (e) {
      ui.notify(String(e), "error");
    } finally {
      busy = false;
    }
  }

  function openAddDialog() {
    showAddDialog = true;
    searchQuery = "";
    localResults = [];
    keyserverResults = [];
  }

  async function handleSearch() {
    if (!searchQuery.trim()) return;
    searching = true;
    keyserverResults = [];
    try {
      // Search local keyring first
      const allKeys = await listGpgKeys();
      const q = searchQuery.trim().toLowerCase();
      localResults = allKeys.filter(
        (k) =>
          k.uid.toLowerCase().includes(q) ||
          k.id.toLowerCase().includes(q) ||
          k.fingerprint.toLowerCase().includes(q),
      );
      // Also search keyserver
      try {
        keyserverResults = await searchGpgKeyserver(searchQuery.trim());
        // Filter out keys already in local results
        const localIds = new Set(localResults.map((k) => k.id));
        keyserverResults = keyserverResults.filter((k) => !localIds.has(k.id));
      } catch {
        // Keyserver search may fail, that's OK
      }
    } catch (e) {
      ui.notify(String(e), "error");
    } finally {
      searching = false;
    }
  }

  async function addLocalKey(key: GpgKey) {
    busy = true;
    try {
      await addRecipient(key.id);
      await refresh();
      showAddDialog = false;
      ui.notify("Key added, store re-encrypted");
      await passwords.refresh();
    } catch (e) {
      ui.notify(String(e), "error");
    } finally {
      busy = false;
    }
  }

  async function importAndAdd(key: GpgKey) {
    importing = key.id;
    try {
      await importGpgKeyFromKeyserver(key.id);
      await addRecipient(key.id);
      await refresh();
      showAddDialog = false;
      ui.notify("Key imported and added, store re-encrypted");
      await passwords.refresh();
    } catch (e) {
      ui.notify(String(e), "error");
    } finally {
      importing = null;
    }
  }
</script>

{#if recipients.length > 0}
  <div class="space-y-1">
    <button
      class="w-full px-3 py-1 text-xs text-zinc-500 uppercase tracking-wide flex items-center justify-between hover:text-zinc-300 transition-colors"
      onclick={() => (expanded = !expanded)}
    >
      <span>{label()} ({recipients.length})</span>
      <span class="text-[10px]">{expanded ? "▲" : "▼"}</span>
    </button>

    {#if expanded}
      <div class="space-y-1">
        {#each recipients as r}
          <div class="flex items-start group px-1">
            <div class="flex-1 min-w-0">
              {#if r.key}
                <div class="text-[11px] text-zinc-300 truncate" title={r.key.uid}>{r.key.uid}</div>
                <div class="text-[10px] text-zinc-500 font-mono truncate" title={r.rawId}>{r.key.id}</div>
              {:else}
                <div class="text-[11px] text-zinc-400 font-mono truncate" title={r.rawId}>{r.rawId}</div>
                <div class="text-[10px] text-zinc-600">Unknown key</div>
              {/if}
            </div>
            <button
              class="px-1 mt-0.5 text-[10px] text-zinc-600 hover:text-red-400 opacity-0 group-hover:opacity-100 transition-opacity shrink-0"
              onclick={() => handleRemove(r.rawId)}
              disabled={recipients.length <= 1 || busy}
              title={recipients.length <= 1 ? "Cannot remove the last key" : "Remove"}
            >
              ×
            </button>
          </div>
        {/each}
        <button
          class="w-full px-1 py-1 text-[11px] text-zinc-500 hover:text-zinc-300 transition-colors text-left"
          onclick={openAddDialog}
        >
          + Add key...
        </button>
      </div>
    {/if}
  </div>
{/if}

{#if showAddDialog}
  <div class="fixed inset-0 bg-black/50 flex items-center justify-center z-50">
    <div class="bg-zinc-900 border border-zinc-700 rounded-lg w-full max-w-sm p-4 space-y-3">
      <div class="flex items-center justify-between">
        <h3 class="text-sm font-medium">Add Key to Store</h3>
        <button
          class="text-zinc-500 hover:text-zinc-300 transition-colors text-sm"
          onclick={() => (showAddDialog = false)}
        >
          ×
        </button>
      </div>

      <form onsubmit={(e) => { e.preventDefault(); handleSearch(); }} class="flex gap-2">
        <input
          type="text"
          bind:value={searchQuery}
          placeholder="Search by email, name, or key ID..."
          class="flex-1 bg-zinc-800 border border-zinc-700 rounded px-3 py-1.5 text-sm focus:outline-none focus:border-zinc-500"
        />
        <button
          type="submit"
          disabled={!searchQuery.trim() || searching}
          class="px-3 py-1.5 text-sm bg-zinc-700 hover:bg-zinc-600 disabled:opacity-50 rounded transition-colors"
        >
          {searching ? "..." : "Search"}
        </button>
      </form>

      <div class="max-h-64 overflow-y-auto space-y-1">
        {#if localResults.length > 0}
          <div class="text-[10px] text-zinc-500 uppercase tracking-wide px-1 pt-1">Local keyring</div>
          {#each localResults as key}
            <button
              class="w-full text-left px-3 py-2 bg-zinc-800/50 border border-zinc-700 hover:border-zinc-500 rounded transition-colors"
              onclick={() => addLocalKey(key)}
              disabled={busy}
            >
              <div class="text-sm truncate">{key.uid}</div>
              <div class="text-xs text-zinc-500 font-mono">{key.id}</div>
            </button>
          {/each}
        {/if}

        {#if keyserverResults.length > 0}
          <div class="text-[10px] text-zinc-500 uppercase tracking-wide px-1 pt-1">Keyserver</div>
          {#each keyserverResults as key}
            <button
              class="w-full text-left px-3 py-2 bg-zinc-800/50 border border-zinc-700 hover:border-zinc-500 rounded transition-colors"
              onclick={() => importAndAdd(key)}
              disabled={importing !== null}
            >
              <div class="text-sm truncate">{key.uid}</div>
              <div class="text-xs text-zinc-500 font-mono">{key.id}</div>
              {#if importing === key.id}
                <div class="text-[10px] text-blue-400 mt-0.5">Importing...</div>
              {:else}
                <div class="text-[10px] text-zinc-600 mt-0.5">Import from keyserver</div>
              {/if}
            </button>
          {/each}
        {/if}

        {#if !searching && searchQuery.trim() && localResults.length === 0 && keyserverResults.length === 0}
          <div class="text-zinc-500 text-sm p-2">No keys found for "{searchQuery}"</div>
        {/if}
      </div>
    </div>
  </div>
{/if}
