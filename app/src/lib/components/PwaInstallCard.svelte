<script lang="ts">
  import { pwaInstallAvailable, promptInstall } from '$lib/pwa';

  let loading = $state(false);
  let dismissed = $state(
    typeof localStorage !== 'undefined' &&
      localStorage.getItem('pwa-install-dismissed') === 'true'
  );
  let justInstalled = $state(false);

  async function handleInstall() {
    loading = true;
    try {
      var accepted = await promptInstall();
      if (accepted) {
        justInstalled = true;
        setTimeout(function () {
          dismissed = true;
          justInstalled = false;
        }, 3000);
      }
    } catch (e) {
      console.error('Install failed:', e);
    } finally {
      loading = false;
    }
  }

  function handleDismiss() {
    dismissed = true;
    localStorage.setItem('pwa-install-dismissed', 'true');
  }
</script>

{#if $pwaInstallAvailable && !dismissed}
  <div class="card border-primary-info relative">
    <button
      class="absolute top-2 right-2 text-white/30 hover:text-white/60 transition-colors text-sm leading-none cursor-pointer"
      onclick={handleDismiss}
      aria-label="Dismiss"
    >
      ✕
    </button>
    <div class="flex items-start gap-3">
      <div class="shrink-0">
        <img
          src="/icons/icon-192.png"
          alt="Protocol Pawns"
          class="w-11 h-11 rounded-lg"
        />
      </div>
      <div class="flex-1 min-w-0">
        {#if justInstalled}
          <h3 class="text-sm font-semibold text-primary-green mb-1">
            Installed!
          </h3>
          <p class="text-xs text-white/60">
            Protocol Pawns is now installed on your device. You can launch it
            from your home screen.
          </p>
        {:else}
          <h3 class="text-sm font-semibold mb-1">Install Protocol Pawns</h3>
          <p class="text-xs text-white/60 mb-2">
            Get the full app experience — play chess on-chain right from your
            home screen.
          </p>
          <ul class="text-xs text-white/50 mb-2.5 space-y-0.5">
            <li>Quick access from your home screen</li>
            <li>Native app-like experience</li>
          </ul>
          <button
            class="btn-primary text-xs"
            onclick={handleInstall}
            disabled={loading}
          >
            {loading ? 'Installing...' : 'Install App'}
          </button>
        {/if}
      </div>
    </div>
  </div>
{/if}
