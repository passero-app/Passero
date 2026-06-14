<script lang="ts">
  import type { PasswordEntry } from "$lib/types";
  import { passwords } from "$lib/stores/passwords.svelte";
  import PasswordTreeNode from "./PasswordTreeNode.svelte";

  let { entry, depth = 0 }: { entry: PasswordEntry; depth: number } = $props();
  let expanded = $state(false);

  function handleClick() {
    if (entry.is_dir) {
      expanded = !expanded;
    } else {
      passwords.select(entry.path);
    }
  }
</script>

<div>
  <button
    class="w-full text-left px-2 py-3 min-h-[44px] md:py-1 md:min-h-0 rounded text-base md:text-sm transition-colors flex items-center gap-1.5
      {!entry.is_dir && passwords.selectedPath === entry.path
        ? 'bg-zinc-700 text-white'
        : 'text-zinc-300 hover:bg-zinc-800 hover:text-white'}"
    style="padding-left: {depth * 16 + 8}px"
    onclick={handleClick}
  >
    {#if entry.is_dir}
      <span class="text-zinc-500 text-xs">{expanded ? "▼" : "▶"}</span>
      <span>{entry.name}/</span>
    {:else}
      <span class="text-zinc-500 text-xs">•</span>
      <span>{entry.name}</span>
    {/if}
  </button>

  {#if entry.is_dir && expanded}
    {#each entry.children as child (child.path)}
      <PasswordTreeNode entry={child} depth={depth + 1} />
    {/each}
  {/if}
</div>
