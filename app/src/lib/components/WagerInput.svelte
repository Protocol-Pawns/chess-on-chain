<script lang="ts">
  import TokenInput from './TokenInput.svelte';

  let {
    enabled = $bindable(false),
    tokenId = $bindable(''),
    tokenSymbol = $bindable(''),
    amount = $bindable(''),
    rawAmount = $bindable(''),
    insufficientBalance = $bindable(false)
  } = $props();

  let show = $state(false);

  function toggle() {
    show = !show;
    if (!show) {
      enabled = false;
      amount = '';
      rawAmount = '';
    } else {
      enabled = true;
    }
  }
</script>

<div class="space-y-2">
  <label class="flex items-center gap-2 text-sm cursor-pointer select-none">
    <input type="checkbox" checked={show} onchange={toggle} />
    Add Wager
  </label>

  {#if show}
    <TokenInput
      bind:tokenId
      bind:tokenSymbol
      bind:amount
      bind:rawAmount
      bind:insufficientBalance
    />
  {/if}
</div>
