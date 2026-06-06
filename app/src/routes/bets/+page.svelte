<script lang="ts">
  import { onMount } from 'svelte';
  import { api, type Bet, type BetStats } from '$lib/api/client';
  import { accountStore, isLoggedIn } from '$lib/near/account';
  import { contract } from '$lib/near/connector';
  import { showTxToast } from '$lib/toast';
  import BetCard from '$lib/components/BetCard.svelte';

  const PER_PAGE = 10;

  let bets = $state<Bet[]>([]);
  let loading = $state(true);
  let statsLoading = $state(true);
  let statusFilter = $state<'pending' | 'locked' | 'resolved' | ''>('');
  let page = $state(1);
  let hasMore = $state(false);
  let betStats = $state<BetStats | null>(null);
  let tokens = $state<string[]>([]);
  let tokenBalances = $state<Array<[string, string]>>([]);
  let withdrawing = $state<string | null>(null);

  async function loadStats() {
    if (!$accountStore) return;
    try {
      const [s, tkns, bals] = await Promise.all([
        api.betStats($accountStore).catch(() => null),
        contract.getTokenWhitelist().catch(() => []),
        contract.getTokens($accountStore).catch(() => [])
      ]);
      betStats = s;
      tokens = tkns;
      tokenBalances = bals;
    } catch (e) {
      console.error('Failed to load bet stats:', e);
    } finally {
      statsLoading = false;
    }
  }

  async function load(p: number) {
    if (!$accountStore) return;
    loading = true;
    try {
      const offset = (p - 1) * PER_PAGE;
      const result = await api.bets(
        $accountStore,
        statusFilter || undefined,
        undefined,
        offset + PER_PAGE + 1
      );
      const allItems = result.items;
      hasMore = allItems.length > offset + PER_PAGE;
      bets = allItems.slice(offset, offset + PER_PAGE);
      page = p;
    } catch (e) {
      console.error('Failed to load bets:', e);
    } finally {
      loading = false;
    }
  }

  function filterByStatus(status: 'pending' | 'locked' | 'resolved' | '') {
    statusFilter = status;
    page = 1;
    load(1);
  }

  function goTo(p: number) {
    if (p < 1) return;
    load(p);
    window.scrollTo({ top: 0, behavior: 'smooth' });
  }

  function handleWithdraw(tokenId: string) {
    if (!$accountStore) return;
    withdrawing = tokenId;
    showTxToast(
      contract.withdrawToken(tokenId).finally(() => {
        withdrawing = null;
        setTimeout(loadStats, 4000);
      })
    );
  }

  function shortToken(id: string): string {
    if (id.length <= 24) return id;
    return id.slice(0, 12) + '...' + id.slice(-8);
  }

  onMount(() => {
    load(1);
    loadStats();
  });
</script>

{#if !$isLoggedIn}
  <div class="text-center py-12 text-white/50">
    Connect your wallet to view bets
  </div>
{:else}
  <div class="space-y-4">
    <h2 class="text-base font-semibold">Your Bets</h2>

    {#if betStats && !statsLoading}
      <div class="grid grid-cols-4 gap-3">
        <div class="text-center bg-primary-transparent2 rounded p-2">
          <div class="text-lg font-bold text-primary">{betStats.total_bets}</div>
          <div class="text-xs text-white/50">Total</div>
        </div>
        <div class="text-center bg-primary-transparent2 rounded p-2">
          <div class="text-lg font-bold text-primary-warn">{betStats.total_wagered}</div>
          <div class="text-xs text-white/50">Wagered</div>
        </div>
        <div class="text-center bg-primary-transparent2 rounded p-2">
          <div class="text-lg font-bold text-primary-green">{betStats.won_bets}</div>
          <div class="text-xs text-white/50">Won</div>
        </div>
        <div class="text-center bg-primary-transparent2 rounded p-2">
          <div class="text-lg font-bold text-primary-green">{betStats.total_won}</div>
          <div class="text-xs text-white/50">Earned</div>
        </div>
      </div>
    {/if}

    {#if tokenBalances.length > 0}
      <div class="card space-y-2">
        <h3 class="text-sm font-semibold">Token Balances</h3>
        {#each tokenBalances as [tokenId, balance]}
          <div class="flex items-center justify-between text-sm">
            <span class="text-white/70 truncate mr-2">{shortToken(tokenId)}</span>
            <div class="flex items-center gap-2 shrink-0">
              <span class="text-white/90">{balance}</span>
              <button
                class="btn-secondary text-xs py-0.5 px-2"
                disabled={withdrawing === tokenId}
                onclick={() => handleWithdraw(tokenId)}
              >
                {withdrawing === tokenId ? '...' : 'Withdraw'}
              </button>
            </div>
          </div>
        {/each}
      </div>
    {/if}

    <div class="flex gap-2">
      <button
        class="btn-secondary text-xs"
        class:btn-primary={statusFilter === ''}
        onclick={() => filterByStatus('')}
      >
        All
      </button>
      <button
        class="btn-secondary text-xs"
        class:btn-primary={statusFilter === 'pending'}
        onclick={() => filterByStatus('pending')}
      >
        Pending
      </button>
      <button
        class="btn-secondary text-xs"
        class:btn-primary={statusFilter === 'locked'}
        onclick={() => filterByStatus('locked')}
      >
        Locked
      </button>
      <button
        class="btn-secondary text-xs"
        class:btn-primary={statusFilter === 'resolved'}
        onclick={() => filterByStatus('resolved')}
      >
        Resolved
      </button>
    </div>

    {#if loading}
      <div class="space-y-2 animate-pulse">
        {#each Array(3) as _}
          <div class="card">
            <div class="h-4 w-2/3 rounded bg-white/10 mb-1"></div>
            <div class="h-3 w-1/3 rounded bg-white/5"></div>
          </div>
        {/each}
      </div>
    {:else if bets.length === 0}
      <p class="text-white/50 text-sm">No bets found</p>
    {:else}
      <div class="space-y-2">
        {#each bets as bet}
          <BetCard {bet} />
        {/each}
      </div>
    {/if}

    {#if !loading && (page > 1 || hasMore)}
      <div class="flex items-center justify-center gap-2 text-sm">
        <button
          class="btn text-xs"
          onclick={() => goTo(page - 1)}
          disabled={page <= 1}
        >
          &lt; Prev
        </button>
        <span class="text-white/50">Page {page}</span>
        <button
          class="btn text-xs"
          onclick={() => goTo(page + 1)}
          disabled={!hasMore}
        >
          Next &gt;
        </button>
      </div>
    {/if}
  </div>
{/if}
