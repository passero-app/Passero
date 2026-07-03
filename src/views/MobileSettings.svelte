<script lang="ts">
  import { invoke } from "@tauri-apps/api/core";
  import GithubSync from "../components/GithubSync.svelte";
  import ResetDevice from "../components/ResetDevice.svelte";

  let fingerprint = $state<string | null>(null);

  $effect(() => {
    invoke<{ has_key: boolean; fingerprint: string | null }>(
      "device_key_status",
    )
      .then((s) => {
        fingerprint = s.fingerprint;
      })
      .catch(() => {
        fingerprint = null;
      });
  });
</script>

<div class="px-4 py-4 pb-safe">
  <div class="mx-auto w-full max-w-sm space-y-6">
    <section class="space-y-3">
      <h2 class="text-sm font-medium text-zinc-300">Device key</h2>
      {#if fingerprint}
        <p class="break-all text-xs text-zinc-400">Device key: {fingerprint}</p>
      {:else}
        <p class="text-xs text-zinc-500">No device key found.</p>
      {/if}
    </section>

    <GithubSync />

    <ResetDevice onReset={() => window.location.reload()} />
  </div>
</div>
