<script lang="ts">
  import { onMount } from 'svelte';
  import { fmtOneDecimal } from '$lib/format';
  import {
    api,
    type EloLeaderboardPage,
    type AccountStats,
    type BetLeaderboardEntry
  } from '$lib/api/client';

  interface PppEntry {
    account_id: string;
    balance: string;
  }

  let loading = $state(true);
  let tab = $state<'elo' | 'bets' | 'ppp'>('elo');
  let page = $state(1);
  const PER_PAGE = 25;
  let data: EloLeaderboardPage | null = $state(null);
  let statsMap = $state<Map<string, AccountStats>>(new Map());
  let betEntries = $state<BetLeaderboardEntry[]>([]);
  let betCursor = $state<string | null>(null);
  let betNextCursor = $state<string | null>(null);
  let betHasMore = $state(false);
  let betPage = $state(1);
  let betLoading = $state(false);
  let pppEntries = $state<PppEntry[]>([]);
  let pppLoading = $state(false);

  function truncateAddr(id: string, max = 20): string {
    if (id.length <= max) return id;
    const head = Math.ceil((max - 3) / 2);
    const tail = Math.floor((max - 3) / 2);
    return `${id.slice(0, head)}...${id.slice(-tail)}`;
  }

  const pppFmt = new Intl.NumberFormat('en-US', {
    maximumFractionDigits: 6
  });

  function fmtPpp(rawBalance: string): string {
    return pppFmt.format(Number(rawBalance) / 1_000_000);
  }

  async function loadElo(p: number) {
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

  async function loadPPP() {
    pppLoading = true;
    try {
      const res = await fetch('https://api.fastnear.com/v1/ft/app.chess-game.near/top');
      if (!res.ok) throw new Error(`HTTP ${res.status}`);
      const json = await res.json();
      pppEntries = (json.accounts ?? []) as PppEntry[];
    } catch (e) {
      console.error('Failed to load PPP leaderboard:', e);
    } finally {
      pppLoading = false;
    }
  }

  async function loadBets(reset = false) {
    betLoading = true;
    try {
      const cursor = reset ? undefined : betNextCursor ?? undefined;
      const result = await api.betLeaderboard(cursor, PER_PAGE);
      if (reset) {
        betEntries = result.items;
        betCursor = null;
        betPage = 1;
      } else {
        betEntries = [...betEntries, ...result.items];
      }
      betNextCursor = result.next_cursor;
      betHasMore = result.next_cursor !== null;
    } catch (e) {
      console.error('Failed to load bet leaderboard:', e);
    } finally {
      betLoading = false;
    }
  }

  function goTo(p: number) {
    if (p < 1 || !data || p > data.total_pages) return;
    loadElo(p);
    window.scrollTo({ top: 0, behavior: 'smooth' });
  }

  function switchTab(t: 'elo' | 'bets' | 'ppp') {
    tab = t;
    if (t === 'elo' && !data) loadElo(1);
    if (t === 'bets' && betEntries.length === 0) loadBets(true);
    if (t === 'ppp' && pppEntries.length === 0) loadPPP();
  }

  onMount(() => loadElo(1));
</script>

<div class="flex flex-col gap-4">
  <h2 class="text-xl font-bold text-primary text-center">Leaderboard</h2>

  <div class="flex gap-2 justify-center">
    <button
      class="btn text-xs"
      class:btn-primary={tab === 'elo'}
      onclick={() => switchTab('elo')}
    >
      ELO
    </button>
    <button
      class="btn text-xs"
      class:btn-primary={tab === 'bets'}
      onclick={() => switchTab('bets')}
    >
      Betting
    </button>
    <button
      class="btn text-xs"
      class:btn-primary={tab === 'ppp'}
      onclick={() => switchTab('ppp')}
    >
      PPP
    </button>
  </div>

  {#if tab === 'elo'}
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
                    >{truncateAddr(entry.account_id)}</a
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
                      ? fmtOneDecimal((stats.wins / stats.total_games) * 100)
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
  {:else if tab === 'bets'}
    {#if betLoading && betEntries.length === 0}
      <div class="space-y-1.5 animate-pulse">
        {#each Array(10) as _}
          <div class="card flex items-center gap-3 py-2">
            <div class="h-4 w-6 rounded bg-white/10"></div>
            <div class="h-4 w-28 rounded bg-white/10 flex-1"></div>
            <div class="h-4 w-10 rounded bg-white/5"></div>
          </div>
        {/each}
      </div>
    {:else}
      <div class="card">
        <table class="w-full text-sm">
          <thead>
            <tr class="text-white/50 text-xs">
              <th class="pb-2 text-left">#</th>
              <th class="pb-2 text-left">Bettor</th>
              <th class="pb-2 text-right">Wagered</th>
              <th class="pb-2 text-right">Won</th>
              <th class="pb-2 text-right">Bets</th>
              <th class="pb-2 text-right">Win Rate</th>
            </tr>
          </thead>
          <tbody>
            {#each betEntries as entry, i}
              <tr class="border-t border-primary/20">
                <td class="py-1.5 text-white/40">{i + 1}</td>
                <td class="py-1.5">
                  <a
                    href="/profile/{entry.account_id}"
                    class="text-primary hover:underline text-xs"
                    >{truncateAddr(entry.account_id)}</a
                  >
                </td>
                <td class="py-1.5 text-right text-white/70">{entry.total_wagered}</td>
                <td class="py-1.5 text-right text-primary-green">{entry.total_won}</td>
                <td class="py-1.5 text-right text-white/70">{entry.total_bets}</td>
                <td class="py-1.5 text-right text-white/70"
                  >{entry.total_bets > 0
                    ? fmtOneDecimal((entry.won_bets / entry.total_bets) * 100)
                    : 0}%</td
                >
              </tr>
            {/each}
          </tbody>
        </table>
      </div>

      {#if betHasMore}
        <div class="flex items-center justify-center">
          <button
            class="btn text-xs"
            onclick={() => {
              betPage++;
              loadBets();
            }}
            disabled={betLoading}
          >
            {betLoading ? 'Loading...' : 'Load More'}
          </button>
        </div>
      {/if}

      {#if betEntries.length === 0}
        <p class="text-white/50 text-sm text-center">
          No bets placed yet.
        </p>
      {/if}
    {/if}
  {:else}
    {#if pppLoading && pppEntries.length === 0}
      <div class="space-y-1.5 animate-pulse">
        {#each Array(10) as _}
          <div class="card flex items-center gap-3 py-2">
            <div class="h-4 w-6 rounded bg-white/10"></div>
            <div class="h-4 w-28 rounded bg-white/10 flex-1"></div>
            <div class="h-4 w-10 rounded bg-white/5"></div>
          </div>
        {/each}
      </div>
    {:else}
      <div class="card">
        <table class="w-full text-sm">
          <thead>
            <tr class="text-white/50 text-xs">
              <th class="pb-2 text-left">#</th>
              <th class="pb-2 text-left">Player</th>
              <th class="pb-2 text-right">PPP</th>
            </tr>
          </thead>
          <tbody>
            {#each pppEntries as entry, i}
              <tr class="border-t border-primary/20">
                <td class="py-1.5 text-white/40">{i + 1}</td>
                <td class="py-1.5">
                  <a
                    href="/profile/{entry.account_id}"
                    class="text-primary hover:underline text-xs"
                    >{truncateAddr(entry.account_id)}</a
                  >
                </td>
                <td class="py-1.5 text-right text-primary font-semibold"
                  >{fmtPpp(entry.balance)}</td
                >
              </tr>
            {/each}
          </tbody>
        </table>
      </div>

      {#if pppEntries.length === 0}
        <p class="text-white/50 text-sm text-center">
          No PPP holders yet.
        </p>
      {/if}
    {/if}
  {/if}
</div>
