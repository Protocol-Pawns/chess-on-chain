<script lang="ts">
  import { account$ } from "$lib/auth";
  import { ProgressSpinner, MessageBox, Wallet } from "$lib/components";
</script>

<div class="page">
  <h2>Wallet</h2>
  {#if $account$}
    <Wallet account={$account$} />
  {:else if $account$ === undefined}
    <ProgressSpinner />
  {:else}
    <MessageBox type="info">Please log in!</MessageBox>
  {/if}
</div>

<style lang="scss">
  h2 {
    font-size: 1.7rem;
    align-self: center;
  }

  .page {
    display: flex;
    flex-direction: column;
    gap: 1.2rem;
  }
</style>
