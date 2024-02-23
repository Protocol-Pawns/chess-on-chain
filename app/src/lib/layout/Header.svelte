<script lang="ts">
  import { onDestroy } from "svelte";

  import { browser } from "$app/environment";
  import { ScreenSize } from "$lib/models";
  import { widthAtMost$ } from "$lib/screen-size";

  let isMobile: boolean;

  const unsubscriber = widthAtMost$(ScreenSize.Mobile).subscribe((res) => {
    isMobile = res;
  });

  onDestroy(() => {
    unsubscriber();
  });
</script>

<div class="header">
  <div>
    {#if isMobile != null}
      <a href="https://protocol-pawns.com/" target="_blank" rel="noopener">
        <img style="height: 2rem;" src="/favicon.png" alt="logo" />
      </a>
    {/if}

    <h1>Protocol Pawns</h1>
  </div>

  {#if browser}
    {#await import("$lib/auth") then { Login }}
      <div class="login">
        <Login />
      </div>
    {/await}
  {/if}
</div>

<style lang="scss">
  .header {
    display: flex;
    align-items: center;
    justify-content: end;
    flex-wrap: wrap;
    margin: 0.4rem 0.8rem;
    min-height: 3.5rem;
    flex: 0 0 auto;
    gap: 1.4rem;

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
  }

  .login {
    display: flex;
    flex-direction: row-reverse;
  }
</style>
