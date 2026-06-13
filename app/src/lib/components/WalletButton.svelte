<script lang="ts">
  import {
    accountStore,
    connect,
    disconnect,
    isLoggedIn,
    isRegistered,
    isCheckingRegistration,
    register
  } from '$lib/near/account';
  import { showTxToast } from '$lib/toast';

  let showMenu = $state(false);
</script>

<div class="relative">
  {#if $isLoggedIn}
    <button
      class="btn-secondary text-sm"
      onclick={() => (showMenu = !showMenu)}
    >
      <span class="truncate max-w-24 sm:max-w-32">{$accountStore}</span>
    </button>
    {#if showMenu}
      <div class="fixed inset-0 z-40" onclick={() => (showMenu = false)}></div>
      <div class="dropdown right-0 top-full mt-1 min-w-40 space-y-0.5">
        <a
          href="/profile/{$accountStore}"
          class="block btn-secondary w-full text-left text-sm flex items-center gap-2"
          onclick={() => (showMenu = false)}
        >
          <svg
            xmlns="http://www.w3.org/2000/svg"
            width="16"
            height="16"
            viewBox="0 0 24 24"
            fill="none"
            stroke="currentColor"
            stroke-width="2"
            stroke-linecap="round"
            stroke-linejoin="round"
            ><circle cx="12" cy="8" r="4" /><path
              d="M4 20c0-4 4-6 8-6s8 2 8 6"
            /></svg
          >
          Profile
        </a>
        {#if !$isRegistered && !$isCheckingRegistration}
          <button
            class="btn-primary w-full text-left text-sm flex items-center gap-2"
            onclick={() => {
              showTxToast(register());
              showMenu = false;
            }}
          >
            <svg
              xmlns="http://www.w3.org/2000/svg"
              width="16"
              height="16"
              viewBox="0 0 24 24"
              fill="none"
              stroke="currentColor"
              stroke-width="2"
              stroke-linecap="round"
              stroke-linejoin="round"
              ><circle cx="8" cy="15" r="4" /><line
                x1="10.85"
                y1="12.15"
                x2="19"
                y2="4"
              /><line x1="18" y1="5" x2="20" y2="7" /><line
                x1="15"
                y1="8"
                x2="17"
                y2="10"
              /></svg
            >
            Register (0.05 N)
          </button>
        {/if}
        <button
          class="btn-secondary w-full text-left text-sm flex items-center gap-2"
          onclick={() => {
            disconnect();
            showMenu = false;
          }}
        >
          <svg
            xmlns="http://www.w3.org/2000/svg"
            width="16"
            height="16"
            viewBox="0 0 24 24"
            fill="none"
            stroke="currentColor"
            stroke-width="2"
            stroke-linecap="round"
            stroke-linejoin="round"
            ><path d="M9 21H5a2 2 0 0 1-2-2V5a2 2 0 0 1 2-2h4" /><polyline
              points="16 17 21 12 16 7"
            /><line x1="21" y1="12" x2="9" y2="12" /></svg
          >
          Disconnect
        </button>
      </div>
    {/if}
  {:else}
    <button class="btn-primary text-sm" onclick={connect}> Login </button>
  {/if}
</div>
