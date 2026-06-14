<script lang="ts">
  import { passwords } from "$lib/stores/passwords.svelte";
  import { ui } from "$lib/stores/ui.svelte";
  import { filterTree } from "$lib/utils";
  import PasswordTreeNode from "./PasswordTreeNode.svelte";
  import EmptyStoreSetup from "./EmptyStoreSetup.svelte";

  const filteredTree = $derived(filterTree(passwords.tree, ui.searchQuery));
</script>

<div class="flex-1 overflow-y-auto p-2">
  {#if passwords.loading}
    <div class="text-zinc-500 text-sm p-4">Loading...</div>
  {:else if passwords.error}
    <div class="text-red-400 text-sm p-4">{passwords.error}</div>
  {:else if filteredTree.length === 0}
    {#if ui.searchQuery}
      <div class="text-zinc-500 text-sm p-4">
        No matches for "{ui.searchQuery}"
      </div>
    {:else if passwords.initialized}
      <div class="text-zinc-500 text-sm p-4">No entries yet.</div>
    {:else}
      <EmptyStoreSetup />
    {/if}
  {:else}
    {#each filteredTree as entry (entry.path)}
      <PasswordTreeNode {entry} depth={0} />
    {/each}
  {/if}
</div>
