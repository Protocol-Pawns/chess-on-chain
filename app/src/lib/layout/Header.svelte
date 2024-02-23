<script lang="ts">
  import { mdiMenu, mdiMenuClose } from "@mdi/js";
  import Button from "@smui/button";
  import IconButton, { Icon } from "@smui/icon-button";
  import { writable } from "svelte/store";
  import { slide } from "svelte/transition";

  import { browser } from "$app/environment";
  import { navigating } from "$app/stores";

  let showMenu = false;
  let path$ = writable(window.location.pathname);

  navigating.subscribe(() => {
    $path$ = window.location.pathname;
  });
</script>

<div class="header">
  <a href={window.location.origin} class="novisit">
    <img style="height: 2rem;" src="/favicon.png" alt="logo" />
    <h1>Protocol Pawns</h1>
  </a>

  {#if browser}
    {#await import("$lib/auth") then { Login }}
      <div class="login">
        <Login />
      </div>
    {/await}
  {/if}

  <IconButton
    size="button"
    class="material-icons"
    on:click={() => {
      showMenu = !showMenu;
    }}
    toggle
  >
    <Icon tag="svg" viewBox="0 0 24 24" on>
      <path fill="currentColor" d={mdiMenuClose} />
    </Icon>
    <Icon tag="svg" viewBox="0 0 24 24">
      <path fill="currentColor" d={mdiMenu} />
    </Icon>
  </IconButton>

  {#if showMenu}
    <nav transition:slide>
      {#if browser && $path$ === "/"}
        <Button class="mdc-button__nav-link" variant="raised" disabled>
          Home
        </Button>
      {:else}
        <Button class="mdc-button__nav-link" href="/" variant="outlined">
          Home
        </Button>
      {/if}
      {#if browser && $path$ === "/about"}
        <Button class="mdc-button__nav-link" variant="raised" disabled>
          About
        </Button>
      {:else}
        <Button class="mdc-button__nav-link" href="/about" variant="outlined">
          About
        </Button>
      {/if}
      {#if browser && $path$ === "/partners"}
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

    @include breakpoint(phone, max) {
      font-size: 1.6rem;
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
    }
  }
</style>
