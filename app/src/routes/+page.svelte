<script lang="ts">
  import Accordion from "@smui-extra/accordion";

  import { account$ } from "$lib/auth";
  import { ProgressSpinner, MessageBox, Wallet } from "$lib/components";
</script>

<div class="page">
  {#if $account$}
    <Accordion multiple>
      <Wallet account={$account$} />
    </Accordion>
  {:else if $account$ === undefined}
    <ProgressSpinner />
  {:else}
    <MessageBox type="info">Please log in!</MessageBox>
  {/if}
</div>

<style lang="scss">
  .page {
    display: flex;
    flex-direction: column;
    gap: 1.2rem;
  }
</style>
