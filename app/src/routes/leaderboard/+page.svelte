<script lang="ts">
  import { onMount } from 'svelte';
  import {
    fmtDecimals,
    fmtOneDecimal,
    fmtPPP,
    truncateAddr
  } from '$lib/format';
  import {
    api,
    type RankingPage,
    type BetLeaderboardEntry
  } from '$lib/api/client';
  import Pagination from '$lib/components/Pagination.svelte';

  type SortBy = 'elo' | 'ppp';
  type SortDir = 'desc' | 'asc';

  let loading = $state(true);
  let tab = $state<'rankings' | 'bets'>('rankings');
  let sortBy = $state<SortBy>('elo');
  let eloDir = $state<SortDir>('desc');
  let page = $state(1);
  const PER_PAGE = 25;
  let data: RankingPage | null = $state(null);
  let betEntries = $state<BetLeaderboardEntry[]>([]);
  let betCursor = $state<string | null>(null);
  let betCursors = $state<(string | null)[]>([null]);
  let betPage = $state(1);
  let betTotalPages = $state(1);
  let betLoading = $state(false);

  async function loadRankings(p: number, sort: SortBy) {
    loading = true;
    try {
      if (sort === 'ppp') {
        data = await api.leaderboardPpp(p, PER_PAGE);
      } else {
        data = await api.leaderboardElo(p, PER_PAGE, eloDir);
      }
      page = data.page;
    } catch (e) {
      console.error('Failed to load rankings:', e);
    } finally {
      loading = false;
    }
  }

  async function loadBets(reset = false) {
    betLoading = true;
    try {
      if (reset) {
        betCursors = [null];
        betPage = 1;
      }
      const cursor = betCursors[betPage - 1] ?? undefined;
      const result = await api.betLeaderboard(cursor, PER_PAGE);
      betEntries = result.items;
      const nextCursor = result.next_cursor;
      if (nextCursor && betPage >= betCursors.length) {
        betCursors = [...betCursors, nextCursor];
      }
      betTotalPages = nextCursor ? betPage + 1 : betPage;
      betCursor = nextCursor;
    } catch (e) {
      console.error('Failed to load bet leaderboard:', e);
    } finally {
      betLoading = false;
    }
  }

  function goToBetPage(p: number) {
    if (p < 1 || p > betTotalPages) return;
    betPage = p;
    loadBets();
    window.scrollTo({ top: 0, behavior: 'smooth' });
  }

  function goTo(p: number) {
    if (p < 1 || !data || p > data.total_pages) return;
    loadRankings(p, sortBy);
    window.scrollTo({ top: 0, behavior: 'smooth' });
  }

  function toggleSort(s: SortBy) {
    if (sortBy === 'elo' && s === 'elo') {
      eloDir = eloDir === 'desc' ? 'asc' : 'desc';
      page = 1;
      loadRankings(1, s);
      return;
    }
    sortBy = s;
    if (s === 'elo') eloDir = 'desc';
    page = 1;
    loadRankings(1, s);
  }

  function switchTab(t: 'rankings' | 'bets') {
    tab = t;
    if (t === 'rankings' && !data) loadRankings(1, sortBy);
    if (t === 'bets' && betEntries.length === 0) loadBets(true);
  }

  onMount(() => loadRankings(1, sortBy));
</script>

<svelte:head>
  <title>Protocol Pawns - Leaderboard</title>
</svelte:head>

<div class="flex flex-col gap-4">
  <h2 class="text-xl font-bold text-primary text-center">Leaderboard</h2>

  <div class="flex gap-2 justify-center">
    <button
      class="btn text-xs"
      class:btn-primary={tab === 'rankings'}
      onclick={() => switchTab('rankings')}
    >
      Rankings
    </button>
    <button
      class="btn text-xs"
      class:btn-primary={tab === 'bets'}
      onclick={() => switchTab('bets')}
    >
      Betting
    </button>
  </div>

  {#if tab === 'rankings'}
    {#if loading}
      <div class="space-y-1.5 animate-pulse">
        {#each Array(10) as _}
          <div class="card flex items-center gap-3 py-2">
            <div class="h-4 w-6 rounded bg-white/10"></div>
            <div class="h-4 w-28 rounded bg-white/10 flex-1"></div>
            <div class="h-4 w-10 rounded bg-white/5"></div>
            <div class="h-4 w-10 rounded bg-white/5"></div>
            <div class="h-4 w-6 rounded bg-white/5"></div>
            <div class="h-4 w-6 rounded bg-white/5"></div>
            <div class="h-4 w-6 rounded bg-white/5"></div>
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
              <th
                class="pb-2 text-right cursor-pointer select-none"
                onclick={() => toggleSort('elo')}
              >
                ELO
                {#if sortBy === 'elo'}
                  <span class="ml-0.5 text-[10px]"
                    >{eloDir === 'desc' ? '▼' : '▲'}</span
                  >
                {/if}
              </th>
              <th
                class="pb-2 text-right cursor-pointer select-none"
                onclick={() => toggleSort('ppp')}
              >
                PPP
                {#if sortBy === 'ppp'}
                  <span class="ml-0.5 text-[10px]">▼</span>
                {/if}
              </th>
              <th class="pb-2 text-right">W</th>
              <th class="pb-2 text-right">L</th>
              <th class="pb-2 text-right">D</th>
              <th class="pb-2 text-right">Rate</th>
            </tr>
          </thead>
          <tbody>
            {#each data.entries as entry}
              <tr class="border-t border-white/10">
                <td class="py-1.5 text-white/40">{entry.rank}</td>
                <td class="py-1.5">
                  <a
                    href="/profile/{entry.account_id}"
                    class="text-primary hover:underline text-xs"
                    >{truncateAddr(entry.account_id)}</a
                  >
                </td>
                <td class="py-1.5 text-right text-primary-warn font-semibold"
                  >{entry.elo != null ? fmtDecimals(entry.elo) : '-'}</td
                >
                <td class="py-1.5 text-right text-primary font-semibold"
                  >{fmtPPP(entry.ppp)}</td
                >
                <td class="py-1.5 text-right text-primary-green"
                  >{entry.wins}</td
                >
                <td class="py-1.5 text-right text-primary-err"
                  >{entry.losses}</td
                >
                <td class="py-1.5 text-right text-white/50">{entry.draws}</td>
                <td class="py-1.5 text-right text-white/70"
                  >{entry.total_games > 0
                    ? fmtOneDecimal((entry.wins / entry.total_games) * 100)
                    : 0}%</td
                >
              </tr>
            {/each}
          </tbody>
        </table>
      </div>

      {#if data.total_pages > 1}
        <Pagination {page} totalPages={data.total_pages} onchange={goTo} />
      {/if}

      {#if data.entries.length === 0}
        <p class="text-white/50 text-sm text-center">
          No players yet. Be the first!
        </p>
      {:else}
        <p class="text-white/30 text-[10px] text-center">
          W / L / D stats reflect human vs human games only
        </p>
      {/if}
    {/if}
  {:else}
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
              <th class="pb-2 text-right">Bets</th>
              <th class="pb-2 text-right">Win Rate</th>
            </tr>
          </thead>
          <tbody>
            {#each betEntries as entry, i}
              <tr class="border-t border-white/10">
                <td class="py-1.5 text-white/40">{i + 1}</td>
                <td class="py-1.5">
                  <a
                    href="/profile/{entry.account_id}"
                    class="text-primary hover:underline text-xs"
                    >{truncateAddr(entry.account_id)}</a
                  >
                  {#if Object.keys(entry.by_token).length > 0}
                    <div class="text-[10px] text-white/40 mt-0.5">
                      {#each Object.entries(entry.by_token) as [tokenId, ts], ti}
                        {#if ti > 0}<span class="text-white/20"> · </span>{/if}
                        <span class="text-primary-warn">{ts.wagered}</span>
                        <span class="text-white/20">/</span>
                        <span class="text-primary-green">{ts.won}</span>
                      {/each}
                    </div>
                  {/if}
                </td>
                <td class="py-1.5 text-right text-white/70"
                  >{entry.total_bets}</td
                >
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

      {#if betTotalPages > 1 || betPage > 1}
        <Pagination
          page={betPage}
          totalPages={betTotalPages}
          onchange={goToBetPage}
        />
      {/if}

      {#if betEntries.length === 0}
        <p class="text-white/50 text-sm text-center">No bets placed yet.</p>
      {/if}
    {/if}
  {/if}
</div>
