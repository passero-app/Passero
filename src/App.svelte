<script lang="ts">
  import Sidebar from "./components/Sidebar.svelte";
  import MainView from "./views/MainView.svelte";
  import SettingsView from "./views/SettingsView.svelte";

  import GpgView from "./views/GpgView.svelte";
  import MobileView from "./views/MobileView.svelte";
  import M0Setup from "./views/M0Setup.svelte";
  import { ui } from "$lib/stores/ui.svelte";
  import { passwords } from "$lib/stores/passwords.svelte";
  import { settings } from "$lib/stores/settings.svelte";
  import { onMount } from "svelte";

  const isIos = /iPad|iPhone|iPod/.test(navigator.userAgent);
  let unlocked = $state(!isIos);

  function loadStores() {
    passwords.refresh();
    settings.load();
  }

  function handleUnlocked() {
    unlocked = true;
    loadStores();
  }

  onMount(() => {
    if (!isIos) {
      loadStores();
    }
  });
</script>

{#if !unlocked}
  <M0Setup onUnlocked={handleUnlocked} />
{:else}
<div class="hidden md:flex h-screen bg-zinc-900 text-zinc-100">
  <Sidebar />
  <main class="flex-1 overflow-hidden flex flex-col">
    {#if ui.notification}
      <div
        class="px-4 py-2 text-sm {ui.notification.type === 'error'
          ? 'bg-red-900/50 text-red-200'
          : 'bg-green-900/50 text-green-200'}"
      >
        {ui.notification.message}
      </div>
    {/if}
    {#if ui.currentView === "main"}
      <MainView />
    {:else if ui.currentView === "settings"}
      <SettingsView />
    {:else if ui.currentView === "gpg"}
      <GpgView />
    {/if}
  </main>
</div>

<div class="flex md:hidden flex-col h-screen bg-zinc-900 text-zinc-100">
  {#if ui.notification}
    <div
      class="px-4 py-2 text-sm pt-safe {ui.notification.type === 'error'
        ? 'bg-red-900/50 text-red-200'
        : 'bg-green-900/50 text-green-200'}"
    >
      {ui.notification.message}
    </div>
  {/if}
  <div class="flex-1 overflow-hidden">
    <MobileView />
  </div>
</div>
{/if}
