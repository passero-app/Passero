<script lang="ts">
  import SearchBar from "../components/SearchBar.svelte";
  import PasswordTree from "../components/PasswordTree.svelte";
  import PasswordViewer from "../components/PasswordViewer.svelte";
  import PasswordEditor from "../components/PasswordEditor.svelte";
  import SettingsView from "./SettingsView.svelte";
  import MobileSettings from "./MobileSettings.svelte";
  import GpgView from "./GpgView.svelte";
  import VaultSwitcher from "../components/VaultSwitcher.svelte";
  import { ui } from "$lib/stores/ui.svelte";
  import { passwords } from "$lib/stores/passwords.svelte";
  import { gitPull, gitPush } from "$lib/commands";

  let showNewEditor = $state(false);

  let showDetail = $derived(
    ui.currentView === "main" && passwords.selectedPath !== null,
  );

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
    } catch (e) {
      ui.notify(String(e), "error");
    }
  }
</script>

<div class="flex flex-col h-full">
  {#if ui.currentView === "settings"}
    <div class="flex items-center gap-1 px-2 pt-safe border-b border-zinc-800 bg-zinc-950">
      <button
        class="flex items-center gap-1 px-2 min-h-[44px] text-blue-400 active:opacity-60"
        onclick={() => ui.navigate("main")}
      >
        <span class="text-xl leading-none">‹</span>
        <span class="text-base">Passwords</span>
      </button>
      <h1 class="flex-1 text-center text-base font-semibold pr-16">Settings</h1>
    </div>
    <div class="flex-1 momentum-scroll">
      <SettingsView />
    </div>
  {:else if ui.currentView === "sync"}
    <div class="flex items-center gap-1 px-2 pt-safe border-b border-zinc-800 bg-zinc-950">
      <button
        class="flex items-center gap-1 px-2 min-h-[44px] text-blue-400 active:opacity-60"
        onclick={() => ui.navigate("main")}
      >
        <span class="text-xl leading-none">‹</span>
        <span class="text-base">Passwords</span>
      </button>
      <h1 class="flex-1 text-center text-base font-semibold pr-16">Sync &amp; Device</h1>
    </div>
    <div class="flex-1 momentum-scroll">
      <MobileSettings />
    </div>
  {:else if ui.currentView === "gpg"}
    <div class="flex items-center gap-1 px-2 pt-safe border-b border-zinc-800 bg-zinc-950">
      <button
        class="flex items-center gap-1 px-2 min-h-[44px] text-blue-400 active:opacity-60"
        onclick={() => ui.navigate("main")}
      >
        <span class="text-xl leading-none">‹</span>
        <span class="text-base">Passwords</span>
      </button>
      <h1 class="flex-1 text-center text-base font-semibold pr-16">GPG Keys</h1>
    </div>
    <div class="flex-1 momentum-scroll">
      <GpgView />
    </div>
  {:else if showDetail}
    <div class="flex items-center px-2 pt-safe border-b border-zinc-800 bg-zinc-950">
      <button
        class="flex items-center gap-1 px-2 min-h-[44px] text-blue-400 active:opacity-60"
        onclick={() => passwords.deselect()}
      >
        <span class="text-xl leading-none">‹</span>
        <span class="text-base">Passwords</span>
      </button>
    </div>
    <div class="flex-1 flex flex-col momentum-scroll pb-safe">
      <PasswordViewer />
    </div>
  {:else}
    <div class="px-3 pt-safe border-b border-zinc-800 bg-zinc-950">
      <div class="flex items-center justify-between min-h-[52px]">
        <h1 class="text-2xl font-bold tracking-tight">Passwords</h1>
        <div class="flex items-center gap-1">
          <button
            class="flex items-center justify-center w-11 h-11 text-blue-400 active:opacity-60"
            onclick={() => handleSync("pull")}
            title="Pull"
            aria-label="Pull from remote"
          >
            <span class="text-xl">↓</span>
          </button>
          <button
            class="flex items-center justify-center w-11 h-11 text-blue-400 active:opacity-60"
            onclick={() => handleSync("push")}
            title="Push"
            aria-label="Push to remote"
          >
            <span class="text-xl">↑</span>
          </button>
          <button
            class="flex items-center justify-center w-11 h-11 text-blue-400 active:opacity-60"
            onclick={() => (showNewEditor = true)}
            title="New entry"
            aria-label="New entry"
          >
            <span class="text-2xl leading-none">+</span>
          </button>
          <button
            class="flex items-center justify-center w-11 h-11 text-blue-400 active:opacity-60"
            onclick={() => (ui.mobileMenuOpen = !ui.mobileMenuOpen)}
            title="Menu"
            aria-label="Menu"
          >
            <span class="text-xl leading-none">⋯</span>
          </button>
        </div>
      </div>
    </div>

    {#if ui.mobileMenuOpen}
      <div class="border-b border-zinc-800 bg-zinc-950 px-2 py-2 space-y-1">
        <VaultSwitcher />
        <button
          class="w-full text-left px-3 min-h-[44px] rounded text-sm text-zinc-300 hover:bg-zinc-800 active:bg-zinc-800"
          onclick={() => ui.navigate("gpg")}
        >
          GPG Keys
        </button>
        <button
          class="w-full text-left px-3 min-h-[44px] rounded text-sm text-zinc-300 hover:bg-zinc-800 active:bg-zinc-800"
          onclick={() => ui.navigate("settings")}
        >
          Settings
        </button>
        <button
          class="w-full text-left px-3 min-h-[44px] rounded text-sm text-zinc-300 hover:bg-zinc-800 active:bg-zinc-800"
          onclick={() => ui.navigate("sync")}
        >
          Sync &amp; Device
        </button>
      </div>
    {/if}

    <SearchBar />
    <div class="flex-1 momentum-scroll pb-safe">
      <PasswordTree />
    </div>
  {/if}
</div>

{#if showNewEditor}
  <PasswordEditor editPath={null} onclose={() => (showNewEditor = false)} />
{/if}

{#if ui.showEditor && passwords.selectedPath}
  <PasswordEditor
    editPath={passwords.selectedPath}
    onclose={() => (ui.showEditor = false)}
  />
{/if}
