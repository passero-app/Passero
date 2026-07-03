<script lang="ts">
  import { invoke } from "@tauri-apps/api/core";
  import { remove } from "../lib/keystore";

  const KEYSTORE_SERVICE = "app.passero";
  const KEYSTORE_USER = "passero-pgp-key";

  let { onReset }: { onReset: () => void } = $props();

  let busy = $state(false);
  let error = $state<string | null>(null);
  let confirmingReset = $state(false);

  async function resetDevice() {
    busy = true;
    error = null;
    try {
      try {
        await remove(KEYSTORE_SERVICE, KEYSTORE_USER);
      } catch (e) {
        console.warn("keystore remove failed", e);
      }
      await invoke("reset_device");
      confirmingReset = false;
      onReset();
    } catch (e) {
      error = String(e);
    } finally {
      busy = false;
    }
  }
</script>

<section class="space-y-3 border-t border-zinc-700 pt-4">
  {#if error}
    <div class="rounded bg-red-900/50 px-3 py-2 text-sm text-red-200">{error}</div>
  {/if}
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
