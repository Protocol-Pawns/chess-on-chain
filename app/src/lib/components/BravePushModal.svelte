<script lang="ts">
  import Modal from './Modal.svelte';
  import {
    bravePushModalOpen,
    closeBravePushModal
  } from '$lib/push/brave-modal';
  import { showToast } from '$lib/toast';

  const BRAVE_SETTINGS_URL = 'brave://settings/privacy';

  function copySettingsLink() {
    navigator.clipboard
      .writeText(BRAVE_SETTINGS_URL)
      .then(() =>
        showToast('success', 'Copied', 'Paste it into your Brave address bar.')
      )
      .catch(() =>
        showToast(
          'error',
          'Copy failed',
          `Paste this into your address bar: ${BRAVE_SETTINGS_URL}`
        )
      );
  }
</script>

<Modal open={$bravePushModalOpen} onclose={closeBravePushModal}>
  <div class="card max-w-sm w-full bg-surface">
    <div class="flex items-start gap-3 mb-4">
      <div class="text-2xl">🦁</div>
      <div>
        <h3 class="text-base font-semibold mb-1">Brave push setup</h3>
        <p class="text-sm text-white/70">
          Brave blocks Google's push service by default, so notifications can't
          register until you enable it.
        </p>
      </div>
    </div>

    <ol class="text-sm text-white/80 space-y-2 mb-4 list-decimal pl-4">
      <li>
        Open <span class="font-mono text-primary-light"
          >{BRAVE_SETTINGS_URL}</span
        >
      </li>
      <li>
        Enable <strong>"Use Google services for push messaging"</strong>
      </li>
      <li>Return here and try enabling notifications again</li>
    </ol>

    <div class="flex gap-2 justify-end">
      <button class="btn-secondary text-sm" onclick={closeBravePushModal}>
        Close
      </button>
      <button class="btn-primary text-sm" onclick={copySettingsLink}>
        Copy settings link
      </button>
    </div>
  </div>
</Modal>
