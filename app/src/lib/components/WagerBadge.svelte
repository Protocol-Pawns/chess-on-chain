<script lang="ts">
  import { formatWager, type WagerDisplay } from '$lib/wager';
  import TokenIcon from './TokenIcon.svelte';

  let {
    tokenId,
    rawAmount
  }: {
    tokenId: string | null | undefined;
    rawAmount: string | null | undefined;
  } = $props();

  let display = $state<WagerDisplay | null>(null);
  let loading = $state(false);

  $effect(() => {
    display = null;
    loading = false;
    if (!tokenId || !rawAmount || rawAmount === '0') return;

    loading = true;
    let cancelled = false;
    formatWager(rawAmount, tokenId).then(d => {
      if (!cancelled) {
        display = d;
        loading = false;
      }
    });
    return () => {
      cancelled = true;
    };
  });
</script>

{#if tokenId && rawAmount && rawAmount !== '0'}
  <span class="inline-flex items-center gap-1 text-yellow-400">
    <TokenIcon {tokenId} size={14} />
    {#if loading}
      <span class="text-white/40">…</span>
    {:else if display}
      <span class="font-medium tabular-nums">{display.amount}</span>
      <span class="text-white/50">{display.symbol}</span>
      {#if display.usd}
        <span class="text-white/40">· {display.usd}</span>
      {/if}
    {/if}
  </span>
{/if}
