<script lang="ts">
  import ConfirmModal from './ConfirmModal.svelte';
  import {
    reloginPromptOpen,
    confirmRelogin,
    cancelRelogin
  } from '$lib/near/relogin';

  let loading = $state(false);

  async function handleConfirm() {
    loading = true;
    try {
      await confirmRelogin();
    } finally {
      loading = false;
    }
  }
</script>

<ConfirmModal
  open={$reloginPromptOpen}
  title="Access key used up"
  message="Your limited access key has run out of allowance or is no longer valid. Reconnect your wallet to create a new access key and continue."
  confirmLabel="Reconnect wallet"
  confirmClass="btn-primary text-sm"
  confirmDisabled={loading}
  onconfirm={handleConfirm}
  onclose={cancelRelogin}
/>
