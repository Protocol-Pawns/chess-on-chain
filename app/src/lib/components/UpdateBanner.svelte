<script lang="ts">
  import { swUpdateAvailable, swVersion } from '$lib/pwa';
  import { get } from 'svelte/store';

  let dismissed = $state(false);

  function handleRefresh() {
    const version = get(swVersion);
    if (version) {
      localStorage.setItem('sw_acknowledged_version', version);
    }
    location.reload();
  }

  function handleDismiss() {
    dismissed = true;
    const version = get(swVersion);
    if (version) {
      localStorage.setItem('sw_acknowledged_version', version);
    }
  }

  $effect(() => {
    if ($swUpdateAvailable && $swVersion) {
      const acknowledged = localStorage.getItem('sw_acknowledged_version');
      if ($swVersion === acknowledged) {
        dismissed = true;
      }
    }
  });
</script>

{#if $swUpdateAvailable && !dismissed}
  <div
    class="fixed bottom-2 left-2 right-2 z-40 flex items-center gap-3 rounded border border-primary-info bg-bg px-3 py-2.5 shadow-lg"
  >
    <div class="flex-1 min-w-0">
      <span class="text-sm text-primary">A new version is available.</span>
    </div>
    <button class="btn-primary text-xs shrink-0" onclick={handleRefresh}>
      Refresh
    </button>
    <button
      class="text-white/30 hover:text-white/60 text-sm leading-none cursor-pointer"
      onclick={handleDismiss}
      aria-label="Dismiss"
    >
      ✕
    </button>
  </div>
{/if}
