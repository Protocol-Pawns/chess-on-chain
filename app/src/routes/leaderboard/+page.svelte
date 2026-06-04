<script lang="ts">
  import { onMount } from 'svelte';
  import {
    api,
    type EloLeaderboardPage,
    type AccountStats
  } from '$lib/api/client';

  let loading = $state(true);
  let page = $state(1);
  const PER_PAGE = 25;
  let data: EloLeaderboardPage | null = $state(null);
  let statsMap = $state<Map<string, AccountStats>>(new Map());

  async function load(p: number) {
    loading = true;
    try {
      const result = await api.leaderboardElo(p, PER_PAGE);
      data = result;
      page = result.page;

      const statsResults = await Promise.all(
        result.entries.map(e =>
          api
            .accountStats(e.account_id)
            .then(s => [e.account_id, s] as const)
            .catch(() => null)
        )
      );
      const map = new Map<string, AccountStats>();
      for (const r of statsResults) {
        if (r) map.set(r[0], r[1]);
      }
      statsMap = map;
    } catch (e) {
      console.error('Failed to load leaderboard:', e);
    } finally {
      loading = false;
    }
  }

  function goTo(p: number) {
    if (p < 1 || !data || p > data.total_pages) return;
    load(p);
    window.scrollTo({ top: 0, behavior: 'smooth' });
  }

  onMount(() => load(1));
</script>

<div class="flex flex-col gap-4">
  <h2 class="text-xl font-bold text-primary text-center">Leaderboard</h2>

  {#if loading}
    <div class="space-y-1.5 animate-pulse">
      {#each Array(10) as _}
        <div class="card flex items-center gap-3 py-2">
          <div class="h-4 w-6 rounded bg-white/10"></div>
          <div class="h-4 w-28 rounded bg-white/10 flex-1"></div>
          <div class="h-4 w-10 rounded bg-white/5"></div>
        </div>
      {/each}
    </div>
  {:else if data}
    <div class="card">
      <table class="w-full text-sm">
        <thead>
          <tr class="text-white/50 text-xs">
            <th class="pb-2 text-left">#</th>
            <th class="pb-2 text-left">Player</th>
            <th class="pb-2 text-right">ELO</th>
            <th class="pb-2 text-right">W</th>
            <th class="pb-2 text-right">L</th>
            <th class="pb-2 text-right">D</th>
            <th class="pb-2 text-right">Rate</th>
          </tr>
        </thead>
        <tbody>
          {#each data.entries as entry}
            {@const stats = statsMap.get(entry.account_id)}
            <tr class="border-t border-primary/20">
              <td class="py-1.5 text-white/40">{entry.rank}</td>
              <td class="py-1.5">
                <a
                  href="/profile/{entry.account_id}"
                  class="text-primary hover:underline text-xs"
                  >{entry.account_id}</a
                >
              </td>
              <td class="py-1.5 text-right text-primary-warn font-semibold"
                >{entry.elo}</td
              >
              {#if stats}
                <td class="py-1.5 text-right text-primary-green"
                  >{stats.wins}</td
                >
                <td class="py-1.5 text-right text-primary-err"
                  >{stats.losses}</td
                >
                <td class="py-1.5 text-right text-white/50">{stats.draws}</td>
                <td class="py-1.5 text-right text-white/70"
                  >{stats.total_games > 0
                    ? ((stats.wins / stats.total_games) * 100).toFixed(1)
                    : 0}%</td
                >
              {:else}
                <td class="py-1.5 text-right text-white/30">-</td>
                <td class="py-1.5 text-right text-white/30">-</td>
                <td class="py-1.5 text-right text-white/30">-</td>
                <td class="py-1.5 text-right text-white/30">-</td>
              {/if}
            </tr>
          {/each}
        </tbody>
      </table>
    </div>

    {#if data.total_pages > 1}
      <div class="flex items-center justify-center gap-2 text-sm">
        <button
          class="btn text-xs"
          onclick={() => goTo(page - 1)}
          disabled={page <= 1}
        >
          &lt; Prev
        </button>
        <span class="text-white/50">
          Page {page} of {data.total_pages}
        </span>
        <button
          class="btn text-xs"
          onclick={() => goTo(page + 1)}
          disabled={page >= data.total_pages}
        >
          Next &gt;
        </button>
      </div>
    {/if}

    {#if data.entries.length === 0}
      <p class="text-white/50 text-sm text-center">
        No players yet. Be the first!
      </p>
    {/if}
  {/if}
</div>
