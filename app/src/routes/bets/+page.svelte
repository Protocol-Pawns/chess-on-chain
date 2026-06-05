<script lang="ts">
  import { onMount } from 'svelte';
  import { api, type Bet } from '$lib/api/client';
  import { accountStore, isLoggedIn } from '$lib/near/account';
  import BetCard from '$lib/components/BetCard.svelte';

  let bets = $state<Bet[]>([]);
  let loading = $state(true);
  let statusFilter = $state<'pending' | 'locked' | 'resolved' | ''>('');
  let cursor = $state<string | null>(null);
  let nextCursor = $state<string | null>(null);

  async function load() {
    if (!$accountStore) return;
    try {
      const result = await api.bets(
        $accountStore,
        statusFilter || undefined,
        cursor || undefined,
        25
      );
      bets = result.items;
      nextCursor = result.next_cursor;
    } catch (e) {
      console.error('Failed to load bets:', e);
    } finally {
      loading = false;
    }
  }

  function filterByStatus(status: 'pending' | 'locked' | 'resolved' | '') {
    statusFilter = status;
    cursor = null;
    loading = true;
    load();
  }

  onMount(load);
</script>

{#if !$isLoggedIn}
  <div class="text-center py-12 text-white/50">
    Connect your wallet to view bets
  </div>
{:else if loading}
  <div class="space-y-2 animate-pulse">
    {#each Array(3) as _}
      <div class="card">
        <div class="h-4 w-2/3 rounded bg-white/10 mb-1"></div>
        <div class="h-3 w-1/3 rounded bg-white/5"></div>
      </div>
    {/each}
  </div>
{:else}
  <div class="space-y-4">
    <h2 class="text-base font-semibold">Your Bets</h2>

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

    {#if bets.length === 0}
      <p class="text-white/50 text-sm">No bets found</p>
    {:else}
      <div class="space-y-2">
        {#each bets as bet}
          <BetCard {bet} />
        {/each}
      </div>
    {/if}

    {#if nextCursor}
      <button
        class="btn-secondary text-xs w-full"
        onclick={() => { cursor = nextCursor; load(); }}
      >
        Load More
      </button>
    {/if}
  </div>
{/if}
