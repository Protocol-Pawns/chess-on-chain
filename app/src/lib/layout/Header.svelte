<script lang="ts">
  import { mdiMenu, mdiMenuClose } from "@mdi/js";
  import type { Account } from "@near-wallet-selector/core";
  import Button from "@smui/button";
  import IconButton, { Icon } from "@smui/icon-button";
  import { writable } from "svelte/store";
  import { slide } from "svelte/transition";

  import { navigating } from "$app/stores";
  import { account$, showWalletSelector } from "$lib/auth";
  import { selector$ } from "$lib/near";
  import { showSnackbar } from "$lib/snackbar";

  let showMenu = false;
  let showAccountMenu$ = writable(false);
  let path$ = writable(window.location.pathname);

  $: if (showMenu) {
    $showAccountMenu$ = false;
  }
  showAccountMenu$.subscribe((show) => {
    if (show) {
      showMenu = false;
    }
  });

  navigating.subscribe(() => {
    $path$ = window.location.pathname;
    showMenu = false;
    $showAccountMenu$ = false;
  });

  async function signOut(account?: Account | null) {
    if (!account) return;
    const selector = await $selector$;
    const wallet = await selector.wallet();
    await wallet.signOut();
    showSnackbar(`Disconnected Near account ${account.accountId}`);
    $account$ = null;
  }
</script>

<div class="header">
  <a href={window.location.origin} class="novisit">
    <img style="height: 2rem;" src="/favicon.png" alt="logo" />
    <h1>Protocol Pawns</h1>
  </a>

  {#await import("$lib/auth") then { Login }}
    <Login {showAccountMenu$} />
  {/await}

  <IconButton
    size="button"
    class="material-icons"
    on:click={() => {
      showMenu = !showMenu;
    }}
  >
    {#if showMenu}
      <Icon tag="svg" viewBox="0 0 24 24">
        <path fill="currentColor" d={mdiMenuClose} />
      </Icon>
    {:else}
      <Icon tag="svg" viewBox="0 0 24 24">
        <path fill="currentColor" d={mdiMenu} />
      </Icon>
    {/if}
  </IconButton>

  {#if showMenu}
    <nav transition:slide>
      {#if $path$ === "/"}
        <Button class="mdc-button__nav-link" variant="raised" disabled>
          Home
        </Button>
      {:else}
        <Button class="mdc-button__nav-link" href="/" variant="outlined">
          Home
        </Button>
      {/if}
      {#if $path$ === "/about"}
        <Button class="mdc-button__nav-link" variant="raised" disabled>
          About
        </Button>
      {:else}
        <Button class="mdc-button__nav-link" href="/about" variant="outlined">
          About
        </Button>
      {/if}
      {#if $path$ === "/partners"}
        <Button class="mdc-button__nav-link" variant="raised" disabled>
          Partners
        </Button>
      {:else}
        <Button
          class="mdc-button__nav-link"
          href="/partners"
          variant="outlined"
        >
          Partners
        </Button>
      {/if}
    </nav>
  {/if}

  {#if $showAccountMenu$}
    <nav transition:slide>
      {#if $account$}
        {#if $path$ === "/account"}
          <Button class="mdc-button__nav-link" variant="raised" disabled>
            {$account$.accountId}
          </Button>
        {:else}
          <Button
            class="mdc-button__nav-link"
            href="/account"
            variant="outlined"
          >
            {$account$.accountId}
          </Button>
        {/if}

        <Button
          color="secondary"
          variant="outlined"
          on:click={() => signOut($account$)}>Logout</Button
        >
      {:else}
        <Button variant="outlined" on:click={showWalletSelector}>Login</Button>
      {/if}
    </nav>
  {/if}
</div>

<style lang="scss">
  .header {
    display: flex;
    align-items: center;
    justify-content: end;
    flex-wrap: wrap;
    padding: 0.4rem 0.8rem;
    min-height: 3.5rem;
    flex: 0 0 auto;
    gap: 1.4rem;
    background-color: var(--color-light-green-transparent);

    > :first-child {
      flex: 1 1 auto;
      display: flex;
      align-items: center;
      justify-content: center;
      gap: 0.8rem;
    }
  }

  h1 {
    text-align: center;

    @include breakpoint(mobile, max) {
      font-size: 1.6rem;

      @include breakpoint(phone, max) {
        display: none;
      }
    }
  }

  .login {
    display: flex;
    flex-direction: row-reverse;
  }

  nav {
    width: 100vw;
    display: flex;
    flex-direction: column;
    gap: 0.4rem;
    align-items: center;
    font-size: 1.2rem;

    :global(.mdc-button__nav-link) {
      width: 15rem;
      max-width: 15rem;
      color: unset;
      text-align: center;

      &:hover {
        background-color: var(--color-light-green-transparent);
      }

      &:disabled {
        background-color: var(--color-light-green-transparent-2);
      }
    }
  }
</style>
