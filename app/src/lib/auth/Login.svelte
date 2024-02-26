<script context="module" lang="ts">
  export async function showWalletSelector() {
    modalSize$.set(ModalSize.Medium);
    modal$.set(bind(WalletSelector, {}));
  }
</script>

<script lang="ts">
  import type { WalletSelector as NearWalletSelector } from "@near-wallet-selector/core";
  import Button from "@smui/button";
  import type { Writable } from "svelte/store";
  import { bind } from "svelte-simple-modal";

  import { account$, WalletSelector } from ".";

  import { browser } from "$app/environment";
  import { Near } from "$lib/assets";
  import { modal$, modalSize$, ModalSize } from "$lib/layout";
  import { selector$ } from "$lib/near";

  export let showAccountMenu$: Writable<boolean>;

  $: if (browser) setupWallet($selector$);

  let walletIconUrl: string | undefined;
  selector$.subscribe(async (s) => {
    const selector = await s;
    const wallet = await selector.wallet();
    walletIconUrl = wallet.metadata.iconUrl;
  });

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
    <Button
      variant="outlined"
      on:click={() => {
        $showAccountMenu$ = !$showAccountMenu$;
      }}
    >
      {#if walletIconUrl}
        <img
          src={walletIconUrl}
          alt="wallet icon"
          style="max-height: 100%; padding: 0.3rem;"
        />
      {:else}
        <Near style="max-height: 100%; padding: 0.3rem;" />
      {/if}
    </Button>
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
