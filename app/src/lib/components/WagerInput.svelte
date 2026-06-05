<script lang="ts">
  import { contract } from '$lib/near/connector';
  import { onMount } from 'svelte';

  let {
    enabled = $bindable(false),
    tokenId = $bindable(''),
    amount = $bindable('')
  } = $props();

  let tokens = $state<string[]>([]);
  let show = $state(false);

  onMount(async () => {
    try {
      tokens = await contract.getTokenWhitelist();
      if (tokens.length > 0) tokenId = tokens[0];
    } catch {
      tokens = [];
    }
  });

  function toggle() {
    show = !show;
    if (!show) {
      enabled = false;
      amount = '';
    } else {
      enabled = true;
    }
  }
</script>

<div class="space-y-2">
  <label class="flex items-center gap-2 text-sm cursor-pointer select-none">
    <input type="checkbox" checked={show} onchange={toggle} class="accent-primary" />
    Add Wager
  </label>

  {#if show}
    <div class="flex gap-2">
      <select
        bind:value={tokenId}
        class="bg-transparent border border-primary rounded px-2 py-1.5 text-sm focus:outline-none focus:border-primary-light"
      >
        {#each tokens as token}
          <option value={token}>{token}</option>
        {/each}
      </select>
      <input
        type="text"
        bind:value={amount}
        placeholder="Amount"
        class="flex-1 bg-transparent border border-primary rounded px-2 py-1.5 text-sm focus:outline-none focus:border-primary-light"
      />
    </div>
  {/if}
</div>
