<script lang="ts">
  import type { Bet } from '$lib/api/client';

  let { bet }: { bet: Bet } = $props();

  const statusColors: Record<string, string> = {
    pending: 'text-yellow-400',
    locked: 'text-blue-400',
    resolved: 'text-green-400'
  };

  function shortId(id: string): string {
    if (id.length <= 20) return id;
    return id.slice(0, 8) + '...' + id.slice(-6);
  }
</script>

<div class="card flex items-center justify-between">
  <div>
    <div class="font-medium text-sm">
      {shortId(bet.winner)}
    </div>
    <div class="text-xs text-white/50">
      {bet.amount}
      {shortId(bet.token_id)}
    </div>
  </div>
  <div class="text-right">
    <div class="text-xs {statusColors[bet.status] || 'text-white/50'}">
      {bet.status}
    </div>
    {#if bet.payout}
      <div class="text-xs text-green-400">
        +{bet.payout}
      </div>
    {/if}
  </div>
</div>
