<script lang="ts">
  import { onMount } from 'svelte';
  import {
    api,
    type Bet,
    type BetStats,
    type Challenge
  } from '$lib/api/client';
  import { accountStore, isLoggedIn } from '$lib/near/account';
  import { contract } from '$lib/near/connector';
  import { showTxToast } from '$lib/toast';
  import BetCard from '$lib/components/BetCard.svelte';
  import Pagination from '$lib/components/Pagination.svelte';
  import type { BetInfo } from '$lib/near/contract-types';

  const PER_PAGE = 10;

  type Tab = 'marketplace' | 'my-bets' | 'all-bets';

  let tab = $state<Tab>('marketplace');
  let loading = $state(true);

  let challenges = $state<Challenge[]>([]);
  let challengesLoading = $state(false);
  let challengesHasMore = $state(false);
  let challengeBetInfo = $state<Record<string, BetInfo | null>>({});
  let challengeTokens = $state<string[]>([]);
  let betForms = $state<
    Record<
      string,
      { token: string; amount: string; winner: string; submitting: boolean }
    >
  >({});

  let myBets = $state<Bet[]>([]);
  let myBetsLoading = $state(false);
  let myBetsHasMore = $state(false);
  let myBetsPage = $state(1);
  let statusFilter = $state<'pending' | 'locked' | 'resolved' | ''>('');
  let betStats = $state<BetStats | null>(null);
  let statsLoading = $state(true);
  let tokens = $state<string[]>([]);
  let tokenBalances = $state<Array<[string, string]>>([]);
  let withdrawing = $state<string | null>(null);

  let allBets = $state<Bet[]>([]);
  let allBetsLoading = $state(false);
  let allBetsHasMore = $state(false);
  let allBetsCursor = $state<string | null>(null);
  let allBetsCursors = $state<(string | null)[]>([null]);
  let allBetsPage = $state(1);
  let allBetsStatusFilter = $state<'pending' | 'locked' | 'resolved' | ''>('');

  let newBetPlayer0 = $state('');
  let newBetPlayer1 = $state('');
  let newBetWinner = $state('');
  let newBetToken = $state('');
  let newBetAmount = $state('');
  let newBetSubmitting = $state(false);
  let newBetError = $state('');

  function challengeKey(c: Challenge): string {
    return [c.challenger, c.challenged].sort().join('|');
  }

  function playerPair(c: Challenge): [string, string] {
    return [c.challenger, c.challenged].sort() as [string, string];
  }

  function shortId(id: string): string {
    if (id.length <= 20) return id;
    return id.slice(0, 8) + '...' + id.slice(-6);
  }

  function shortToken(id: string): string {
    if (id.length <= 24) return id;
    return id.slice(0, 12) + '...' + id.slice(-8);
  }

  async function loadChallenges() {
    challengesLoading = true;
    try {
      const result = await api.openChallenges(undefined, PER_PAGE + 1);
      challengesHasMore = result.items.length > PER_PAGE;
      challenges = result.items.slice(0, PER_PAGE);

      const uniquePairs: Record<string, [string, string]> = {};
      for (const c of challenges) {
        const key = challengeKey(c);
        if (!uniquePairs[key]) {
          uniquePairs[key] = playerPair(c);
        }
      }

      const infoEntries = await Promise.all(
        Object.entries(uniquePairs).map(async ([key, players]) => {
          try {
            const info = await contract.getBetInfo(players);
            return [key, info] as const;
          } catch {
            return [key, null] as const;
          }
        })
      );
      challengeBetInfo = Object.fromEntries(infoEntries);

      const tkns = await contract.getTokenWhitelist().catch(() => []);
      challengeTokens = tkns;

      for (const c of challenges) {
        const key = challengeKey(c);
        if (!betForms[key]) {
          betForms[key] = {
            token: tkns[0] || '',
            amount: '',
            winner: '',
            submitting: false
          };
        }
      }
    } catch (e) {
      console.error('Failed to load challenges:', e);
    } finally {
      challengesLoading = false;
    }
  }

  function getBetTotals(
    info: BetInfo | null,
    player0: string,
    player1: string
  ) {
    if (!info?.bets) return { player0Total: '0', player1Total: '0' };
    let p0 = 0;
    let p1 = 0;
    for (const bettors of Object.values(info.bets)) {
      for (const [, bet] of bettors) {
        const amt = Number(bet.amount) || 0;
        if (bet.winner === player0) p0 += amt;
        else p1 += amt;
      }
    }
    return { player0Total: String(p0), player1Total: String(p1) };
  }

  function placeBet(c: Challenge) {
    if (!$accountStore) return;
    const key = challengeKey(c);
    const form = betForms[key];
    if (!form || !form.amount || !form.winner || !form.token) return;
    form.submitting = true;
    const players = playerPair(c);
    showTxToast(
      contract
        .placeBet(form.token, players, form.winner, form.amount)
        .then(() => {
          form.amount = '';
          form.winner = '';
          setTimeout(() => loadChallengeBetInfo(key, players), 4000);
        })
        .finally(() => {
          form.submitting = false;
        })
    );
  }

  async function loadChallengeBetInfo(key: string, players: [string, string]) {
    try {
      const info = await contract.getBetInfo(players);
      challengeBetInfo[key] = info;
    } catch {
      challengeBetInfo[key] = null;
    }
  }

  function submitNewBet() {
    if (!$accountStore) return;
    newBetError = '';
    const p0 = newBetPlayer0.trim();
    const p1 = newBetPlayer1.trim();
    if (!p0 || !p1) {
      newBetError = 'Both player addresses are required';
      return;
    }
    if (p0 === p1) {
      newBetError = 'Players must be different';
      return;
    }
    if ($accountStore === p0 || $accountStore === p1) {
      newBetError = 'You cannot bet on your own game';
      return;
    }
    if (!newBetWinner) {
      newBetError = 'Select a winner';
      return;
    }
    if (!newBetAmount || Number(newBetAmount) <= 0) {
      newBetError = 'Enter a valid amount';
      return;
    }
    if (!newBetToken) {
      newBetError = 'Select a token';
      return;
    }
    const players: [string, string] = [p0, p1].sort() as [string, string];
    newBetSubmitting = true;
    showTxToast(
      contract
        .placeBet(newBetToken, players, newBetWinner, newBetAmount)
        .then(() => {
          newBetPlayer0 = '';
          newBetPlayer1 = '';
          newBetWinner = '';
          newBetAmount = '';
          newBetError = '';
        })
        .catch((e: unknown) => {
          const msg = e instanceof Error ? e.message : String(e);
          newBetError = msg;
        })
        .finally(() => {
          newBetSubmitting = false;
        })
    );
  }

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

  async function loadMyBets(p: number) {
    if (!$accountStore) return;
    myBetsLoading = true;
    try {
      const offset = (p - 1) * PER_PAGE;
      const result = await api.bets(
        $accountStore,
        statusFilter || undefined,
        undefined,
        offset + PER_PAGE + 1
      );
      const allItems = result.items;
      myBetsHasMore = allItems.length > offset + PER_PAGE;
      myBets = allItems.slice(offset, offset + PER_PAGE);
      myBetsPage = p;
    } catch (e) {
      console.error('Failed to load bets:', e);
    } finally {
      myBetsLoading = false;
    }
  }

  function filterByStatus(status: 'pending' | 'locked' | 'resolved' | '') {
    statusFilter = status;
    myBetsPage = 1;
    loadMyBets(1);
  }

  async function loadAllBets(reset = false) {
    allBetsLoading = true;
    try {
      if (reset) {
        allBetsCursors = [null];
        allBetsPage = 1;
      }
      const cursor = allBetsCursors[allBetsPage - 1] ?? undefined;
      const result = await api.globalBets(
        allBetsStatusFilter || undefined,
        cursor,
        PER_PAGE
      );
      allBets = result.items;
      const nextCursor = result.next_cursor;
      if (nextCursor && allBetsPage >= allBetsCursors.length) {
        allBetsCursors = [...allBetsCursors, nextCursor];
      }
      allBetsHasMore = nextCursor !== null;
      allBetsCursor = nextCursor;
    } catch (e) {
      console.error('Failed to load global bets:', e);
    } finally {
      allBetsLoading = false;
    }
  }

  function goToAllBetsPage(p: number) {
    const totalPages = allBetsHasMore ? allBetsPage + 1 : allBetsPage;
    if (p < 1 || p > totalPages) return;
    allBetsPage = p;
    loadAllBets();
    window.scrollTo({ top: 0, behavior: 'smooth' });
  }

  function filterAllBets(status: 'pending' | 'locked' | 'resolved' | '') {
    allBetsStatusFilter = status;
    loadAllBets(true);
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

  function switchTab(t: Tab) {
    tab = t;
    if (t === 'marketplace' && challenges.length === 0) loadChallenges();
    if (t === 'my-bets' && myBets.length === 0) {
      loadMyBets(1);
      loadStats();
    }
    if (t === 'all-bets' && allBets.length === 0) loadAllBets(true);
  }

  function isMyChallenge(c: Challenge): boolean {
    return (
      !!$accountStore &&
      (c.challenger === $accountStore || c.challenged === $accountStore)
    );
  }

  onMount(async () => {
    if ($isLoggedIn) {
      const tkns = await contract.getTokenWhitelist().catch(() => []);
      challengeTokens = tkns;
      if (tkns.length > 0) newBetToken = tkns[0];
      loadChallenges();
      loadStats();
    }
  });
</script>

{#if !$isLoggedIn}
  <div class="text-center py-12 text-white/50">
    Connect your wallet to view bets
  </div>
{:else}
  <div class="space-y-4">
    <div class="flex gap-2 justify-center">
      <button
        class="btn text-xs"
        class:btn-primary={tab === 'marketplace'}
        onclick={() => switchTab('marketplace')}
      >
        Marketplace
      </button>
      <button
        class="btn text-xs"
        class:btn-primary={tab === 'my-bets'}
        onclick={() => switchTab('my-bets')}
      >
        My Bets
      </button>
      <button
        class="btn text-xs"
        class:btn-primary={tab === 'all-bets'}
        onclick={() => switchTab('all-bets')}
      >
        All Bets
      </button>
    </div>

    {#if tab === 'marketplace'}
      <div class="card space-y-3">
        <h2 class="text-sm font-semibold">Place a Bet</h2>

        {#if newBetError}
          <p class="text-xs text-red-400">{newBetError}</p>
        {/if}

        <div class="grid grid-cols-2 gap-2">
          <input
            type="text"
            value={newBetPlayer0}
            oninput={e => (newBetPlayer0 = e.currentTarget.value)}
            placeholder="Player 1 address"
            class="bg-transparent border border-white/15 rounded px-2 py-1 text-xs focus:outline-none focus:border-primary"
          />
          <input
            type="text"
            value={newBetPlayer1}
            oninput={e => (newBetPlayer1 = e.currentTarget.value)}
            placeholder="Player 2 address"
            class="bg-transparent border border-white/15 rounded px-2 py-1 text-xs focus:outline-none focus:border-primary"
          />
        </div>

        {#if newBetPlayer0.trim() && newBetPlayer1.trim() && newBetPlayer0.trim() !== newBetPlayer1.trim()}
          <div class="flex gap-2">
            <button
              class="btn-secondary text-xs flex-1"
              class:opacity-50={newBetWinner !== newBetPlayer0.trim()}
              onclick={() => (newBetWinner = newBetPlayer0.trim())}
            >
              {shortId(newBetPlayer0.trim())} wins
            </button>
            <button
              class="btn-secondary text-xs flex-1"
              class:opacity-50={newBetWinner !== newBetPlayer1.trim()}
              onclick={() => (newBetWinner = newBetPlayer1.trim())}
            >
              {shortId(newBetPlayer1.trim())} wins
            </button>
          </div>
        {/if}

        <div class="flex gap-2">
          <select
            value={newBetToken}
            onchange={e => (newBetToken = e.currentTarget.value)}
            class="bg-transparent border border-white/15 rounded px-2 py-1 text-xs focus:outline-none focus:border-primary"
          >
            {#each challengeTokens as token}
              <option value={token}>{shortToken(token)}</option>
            {/each}
          </select>
          <input
            type="text"
            value={newBetAmount}
            oninput={e => (newBetAmount = e.currentTarget.value)}
            placeholder="Amount"
            class="flex-1 bg-transparent border border-white/15 rounded px-2 py-1 text-xs focus:outline-none focus:border-primary"
          />
        </div>

        <button
          class="btn-primary text-xs w-full"
          disabled={newBetSubmitting ||
            !newBetPlayer0.trim() ||
            !newBetPlayer1.trim() ||
            !newBetWinner ||
            !newBetAmount ||
            !newBetToken}
          onclick={submitNewBet}
        >
          {newBetSubmitting ? 'Placing...' : 'Place Bet'}
        </button>
      </div>

      <h3 class="text-sm font-semibold text-white/60">Open Challenges</h3>

      {#if challengesLoading}
        <div class="space-y-2 animate-pulse">
          {#each Array(3) as _}
            <div class="card">
              <div class="h-4 w-2/3 rounded bg-white/10 mb-1"></div>
              <div class="h-3 w-1/3 rounded bg-white/5"></div>
            </div>
          {/each}
        </div>
      {:else if challenges.length === 0}
        <p class="text-white/50 text-sm">No open challenges to bet on</p>
      {:else}
        <div class="space-y-3">
          {#each challenges as c (c.id)}
            {@const key = challengeKey(c)}
            {@const pair = playerPair(c)}
            {@const info = challengeBetInfo[key]}
            {@const totals = getBetTotals(info, pair[0], pair[1])}
            {@const isOwn = isMyChallenge(c)}
            {@const [p0, p1] = pair}
            <div class="card space-y-3">
              <div class="flex items-center justify-between">
                <div class="text-sm font-medium">
                  <span class="text-white">{shortId(p0)}</span>
                  <span class="text-white/40 mx-1">vs</span>
                  <span class="text-white">{shortId(p1)}</span>
                </div>
                {#if isOwn}
                  <span
                    class="text-xs text-primary bg-primary-transparent2 px-2 py-0.5 rounded"
                    >Your challenge</span
                  >
                {/if}
              </div>

              <div class="flex gap-3 text-xs text-white/60">
                <div>
                  <span class="text-white/40">Bet on </span>
                  <span class="text-yellow-400">{shortId(p0)}</span>:
                  <span class="text-white">{totals.player0Total}</span>
                </div>
                <div>
                  <span class="text-white/40">Bet on </span>
                  <span class="text-blue-400">{shortId(p1)}</span>:
                  <span class="text-white">{totals.player1Total}</span>
                </div>
              </div>

              {#if $accountStore && !isOwn && betForms[key]}
                <div class="space-y-2 border-t border-white/10 pt-2">
                  <div class="flex gap-2">
                    <select
                      value={betForms[key].token}
                      onchange={e => {
                        betForms[key].token = e.currentTarget.value;
                      }}
                      class="bg-transparent border border-white/15 rounded px-2 py-1 text-xs focus:outline-none focus:border-primary"
                    >
                      {#each challengeTokens as token}
                        <option value={token}>{shortToken(token)}</option>
                      {/each}
                    </select>
                    <input
                      type="text"
                      value={betForms[key].amount}
                      oninput={e => {
                        betForms[key].amount = e.currentTarget.value;
                      }}
                      placeholder="Amount"
                      class="flex-1 bg-transparent border border-white/15 rounded px-2 py-1 text-xs focus:outline-none focus:border-primary"
                    />
                  </div>
                  <div class="flex gap-2">
                    <button
                      class="btn-secondary text-xs flex-1"
                      class:opacity-50={betForms[key].winner !== p0}
                      onclick={() => (betForms[key].winner = p0)}
                    >
                      {shortId(p0)} wins
                    </button>
                    <button
                      class="btn-secondary text-xs flex-1"
                      class:opacity-50={betForms[key].winner !== p1}
                      onclick={() => (betForms[key].winner = p1)}
                    >
                      {shortId(p1)} wins
                    </button>
                  </div>
                  <button
                    class="btn-primary text-xs w-full"
                    disabled={!betForms[key].amount ||
                      !betForms[key].winner ||
                      !betForms[key].token ||
                      betForms[key].submitting}
                    onclick={() => placeBet(c)}
                  >
                    {betForms[key].submitting ? 'Placing...' : 'Place Bet'}
                  </button>
                </div>
              {:else if isOwn}
                <p class="text-xs text-white/40">
                  You cannot bet on your own game
                </p>
              {/if}
            </div>
          {/each}
        </div>
      {/if}
    {:else if tab === 'my-bets'}
      <h2 class="text-base font-semibold">My Bets</h2>

      {#if betStats && !statsLoading}
        <div class="grid grid-cols-4 gap-3">
          <div class="text-center bg-primary-transparent2 rounded p-2">
            <div class="text-lg font-bold text-primary">
              {betStats.total_bets}
            </div>
            <div class="text-xs text-white/50">Total</div>
          </div>
          <div class="text-center bg-primary-transparent2 rounded p-2">
            <div class="text-lg font-bold text-primary-warn">
              {betStats.total_wagered}
            </div>
            <div class="text-xs text-white/50">Wagered</div>
          </div>
          <div class="text-center bg-primary-transparent2 rounded p-2">
            <div class="text-lg font-bold text-primary-green">
              {betStats.won_bets}
            </div>
            <div class="text-xs text-white/50">Won</div>
          </div>
          <div class="text-center bg-primary-transparent2 rounded p-2">
            <div class="text-lg font-bold text-primary-green">
              {betStats.total_won}
            </div>
            <div class="text-xs text-white/50">Earned</div>
          </div>
        </div>
      {/if}

      {#if tokenBalances.length > 0}
        <div class="card space-y-2">
          <h3 class="text-sm font-semibold">Token Balances</h3>
          {#each tokenBalances as [tokenId, balance]}
            <div class="flex items-center justify-between text-sm">
              <span class="text-white/70 truncate mr-2"
                >{shortToken(tokenId)}</span
              >
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

      {#if myBetsLoading}
        <div class="space-y-2 animate-pulse">
          {#each Array(3) as _}
            <div class="card">
              <div class="h-4 w-2/3 rounded bg-white/10 mb-1"></div>
              <div class="h-3 w-1/3 rounded bg-white/5"></div>
            </div>
          {/each}
        </div>
      {:else if myBets.length === 0}
        <p class="text-white/50 text-sm">No bets found</p>
      {:else}
        <div class="space-y-2">
          {#each myBets as bet}
            <BetCard {bet} />
          {/each}
        </div>
      {/if}

      {#if !myBetsLoading && (myBetsPage > 1 || myBetsHasMore)}
        <Pagination
          page={myBetsPage}
          totalPages={myBetsHasMore ? myBetsPage + 1 : myBetsPage}
          onchange={p => {
            loadMyBets(p);
            window.scrollTo({ top: 0, behavior: 'smooth' });
          }}
        />
      {/if}
    {:else if tab === 'all-bets'}
      <h2 class="text-base font-semibold">All Bets</h2>

      <div class="flex gap-2">
        <button
          class="btn-secondary text-xs"
          class:btn-primary={allBetsStatusFilter === ''}
          onclick={() => filterAllBets('')}
        >
          All
        </button>
        <button
          class="btn-secondary text-xs"
          class:btn-primary={allBetsStatusFilter === 'pending'}
          onclick={() => filterAllBets('pending')}
        >
          Pending
        </button>
        <button
          class="btn-secondary text-xs"
          class:btn-primary={allBetsStatusFilter === 'locked'}
          onclick={() => filterAllBets('locked')}
        >
          Locked
        </button>
        <button
          class="btn-secondary text-xs"
          class:btn-primary={allBetsStatusFilter === 'resolved'}
          onclick={() => filterAllBets('resolved')}
        >
          Resolved
        </button>
      </div>

      {#if allBetsLoading && allBets.length === 0}
        <div class="space-y-2 animate-pulse">
          {#each Array(5) as _}
            <div class="card">
              <div class="h-4 w-2/3 rounded bg-white/10 mb-1"></div>
              <div class="h-3 w-1/3 rounded bg-white/5"></div>
            </div>
          {/each}
        </div>
      {:else if allBets.length === 0}
        <p class="text-white/50 text-sm">No bets found</p>
      {:else}
        <div class="space-y-2">
          {#each allBets as bet}
            <div class="card flex items-center justify-between">
              <div>
                <div class="font-medium text-sm">
                  <span class="text-white/70">{shortId(bet.bettor)}</span>
                  <span class="text-white/40 mx-1">bet on</span>
                  <span class="text-primary">{shortId(bet.winner)}</span>
                </div>
                <div class="text-xs text-white/50">
                  {bet.amount}
                  {shortToken(bet.token_id)}
                </div>
              </div>
              <div class="text-right">
                <div
                  class="text-xs {bet.status === 'pending'
                    ? 'text-yellow-400'
                    : bet.status === 'locked'
                      ? 'text-blue-400'
                      : 'text-green-400'}"
                >
                  {bet.status}
                </div>
                {#if bet.payout}
                  <div class="text-xs text-green-400">+{bet.payout}</div>
                {/if}
              </div>
            </div>
          {/each}
        </div>

        {#if allBetsHasMore || allBetsPage > 1}
          <Pagination
            page={allBetsPage}
            totalPages={allBetsHasMore ? allBetsPage + 1 : allBetsPage}
            onchange={goToAllBetsPage}
          />
        {/if}
      {/if}
    {/if}
  </div>
{/if}
