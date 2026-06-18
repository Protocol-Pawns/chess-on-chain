<script lang="ts">
  import { getTokenMetadata, isWrapNear } from '$lib/tokens';
  import type { FtMetadata } from '$lib/tokens';
  import NEAR_ICON from '$lib/assets/near.svg';

  let {
    tokenId,
    size = 16
  }: {
    tokenId: string;
    size?: number;
  } = $props();

  let meta = $state<FtMetadata | null>(null);

  $effect(() => {
    meta = null;
    if (isWrapNear(tokenId)) {
      meta = {
        spec: '',
        name: 'NEAR',
        symbol: 'NEAR',
        icon: NEAR_ICON,
        decimals: 24,
        reference: null,
        reference_hash: null
      };
      return;
    }
    let cancelled = false;
    getTokenMetadata(tokenId)
      .then(m => {
        if (!cancelled) meta = m;
      })
      .catch(() => {
        if (!cancelled)
          meta = {
            spec: '',
            name: tokenId,
            symbol: tokenId.slice(0, 4),
            icon: null,
            decimals: 0,
            reference: null,
            reference_hash: null
          };
      });
    return () => {
      cancelled = true;
    };
  });
</script>

{#if meta?.icon}
  <img
    src={meta.icon}
    alt={meta.symbol}
    class="rounded-full shrink-0"
    style="width: {size}px; height: {size}px;"
    loading="lazy"
  />
{:else if meta}
  <div
    class="rounded-full bg-white/10 flex items-center justify-center font-bold shrink-0"
    style="width: {size}px; height: {size}px; font-size: {Math.max(
      8,
      size * 0.5
    )}px;"
  >
    {meta.symbol.slice(0, 2).toUpperCase()}
  </div>
{/if}
