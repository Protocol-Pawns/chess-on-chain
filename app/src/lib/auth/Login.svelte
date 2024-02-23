<script context="module" lang="ts">
  export async function showWalletSelector() {
    modalSize$.set(ModalSize.Medium);
    modal$.set(bind(WalletSelector, {}));
  }
</script>

<script lang="ts">
  import type {
    Account,
    WalletSelector as NearWalletSelector,
  } from "@near-wallet-selector/core";
  import Button from "@smui/button";
  // import { onDestroy } from "svelte";
  import { bind } from "svelte-simple-modal";

  import { account$, WalletSelector } from ".";

  import { browser } from "$app/environment";
  import { modal$, modalSize$, ModalSize } from "$lib/layout";
  import { selector$ } from "$lib/near";
  import { showSnackbar } from "$lib/snackbar";

  $: if (browser) setupWallet($selector$);

  async function signOut(account?: Account | null) {
    if (!account) return;
    const selector = await $selector$;
    const wallet = await selector.wallet();
    await wallet.signOut();
    showSnackbar(`Disconnected Near account ${account.accountId}`);
    $account$ = null;
  }

  async function setupWallet(s: Promise<NearWalletSelector>) {
    const selector = await s;
    const isSignedInWithNear = selector.isSignedIn();
    $account$ = null;
    if (isSignedInWithNear) {
      const account = selector.store
        .getState()
        .accounts.find(({ active }) => active);
      if (!account) return;
      $account$ = account;
      return;
    }
  }
</script>

<div class="login">
  {#if selector$}
    {#if $account$}
      <Button variant="outlined">
        {$account$.accountId}
      </Button>
      <Button
        color="secondary"
        variant="outlined"
        on:click={() => signOut($account$)}>Logout</Button
      >
    {:else}
      <Button variant="outlined" on:click={showWalletSelector}>Login</Button>
    {/if}
  {/if}
</div>

<style>
  .login {
    display: flex;
    align-items: center;
    gap: 0.2rem;
    flex-wrap: wrap;
    justify-content: end;
  }
</style>
