<script lang="ts">
  import TokenIcon from '$lib/components/TokenIcon.svelte';
  import { getTokenMetadata, formatBalance, isWrapNear } from '$lib/tokens';
  import type { FtMetadata } from '$lib/tokens';
  import NEAR_ICON from '$lib/assets/near.svg';

  let {
    tokens,
    showWithdraw = false,
    onWithdraw,
    withdrawing = null
  }: {
    tokens: Array<[string, string]>;
    showWithdraw?: boolean;
    onWithdraw?: (tokenId: string) => void;
    withdrawing?: string | null;
  } = $props();

  let metadataMap = $state<Map<string, FtMetadata>>(new Map());

  $effect(() => {
    const ids = tokens.map(([id]) => id);
    let cancelled = false;
    Promise.allSettled(
      ids.map(async id => [id, await getTokenMetadata(id)] as const)
    ).then(results => {
      if (cancelled) return;
      const map = new Map<string, FtMetadata>();
      for (const r of results) {
        if (r.status === 'fulfilled' && r.value[1]) {
          map.set(r.value[0], r.value[1]);
        }
      }
      metadataMap = map;
    });
    return () => {
      cancelled = true;
    };
  });

  function getMeta(tokenId: string): FtMetadata | undefined {
    if (isWrapNear(tokenId)) {
      return {
        decimals: 24,
        symbol: 'NEAR',
        name: 'NEAR',
        icon: NEAR_ICON,
        spec: '',
        reference: null,
        reference_hash: null
      };
    }
    return metadataMap.get(tokenId);
  }

  function getSymbol(tokenId: string): string {
    return (
      getMeta(tokenId)?.symbol ??
      (tokenId.length > 16 ? tokenId.slice(0, 8) + '...' : tokenId)
    );
  }

  function getFormatted(tokenId: string, balance: string): string {
    const meta = getMeta(tokenId);
    if (!meta) return balance;
    return formatBalance(balance, meta.decimals);
  }
</script>

{#each tokens as [tokenId, balance] (tokenId)}
  <div class="flex items-center justify-between text-sm gap-2">
    <div class="flex items-center gap-2 min-w-0">
      <TokenIcon {tokenId} size={18} />
      <span class="text-white/70 truncate">{getSymbol(tokenId)}</span>
    </div>
    <div class="flex items-center gap-2 shrink-0">
      <span class="text-white/90">{getFormatted(tokenId, balance)}</span>
      {#if showWithdraw && onWithdraw}
        <button
          class="btn-secondary text-xs py-0.5 px-2"
          disabled={withdrawing === tokenId}
          onclick={() => onWithdraw(tokenId)}
        >
          {withdrawing === tokenId ? '...' : 'Withdraw'}
        </button>
      {/if}
    </div>
  </div>
{/each}
