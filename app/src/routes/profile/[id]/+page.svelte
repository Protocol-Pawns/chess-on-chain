<script lang="ts">
  import { onMount } from 'svelte';
  import { page } from '$app/state';
  import { api, type AccountStats, type GameOverview, type BetStats } from '$lib/api/client';
  import { fmtOneDecimal, truncateAddr } from '$lib/format';
  import { contract } from '$lib/near/connector';
  import { accountStore } from '$lib/near/account';
  import { showTxToast } from '$lib/toast';
  import GameCard from '$lib/components/GameCard.svelte';

  const accountId = page.params.id ?? '';

  let stats = $state<AccountStats | null>(null);
  let games = $state<GameOverview[]>([]);
  let activeGame = $state<GameOverview | null>(null);
  let elo = $state<number | null>(null);
  let points = $state<string | null>(null);
  let achievements: Array<[number, string]> = $state([]);
  let questCooldowns: Array<[number, string]> = $state([]);
  let loading = $state(true);
  let betStats = $state<BetStats | null>(null);
  let tokenBalances = $state<Array<[string, string]>>([]);
  let withdrawing = $state<string | null>(null);

  const ACHIEVEMENT_LABELS: Record<string, string> = {
    FirstWinHuman: 'First Win vs Human',
    FirstWinAiEasy: 'First Win vs AI (Easy)',
    FirstWinAiMedium: 'First Win vs AI (Medium)',
    FirstWinAiHard: 'First Win vs AI (Hard)'
  };

  const QUEST_LABELS: Record<string, string> = {
    DailyPlayMove: 'Daily Move',
    WeeklyWinHuman: 'Weekly Win'
  };

  function formatCooldown(timestampMs: number): string {
    const remaining = timestampMs + 57600000 - Date.now();
    if (remaining <= 0) return 'Ready';
    const hours = Math.floor(remaining / 3600000);
    const mins = Math.floor((remaining % 3600000) / 60000);
    return `${hours}h ${mins}m`;
  }

  function formatPoints(raw: string): string {
    const val = BigInt(raw);
    const whole = val / BigInt(1000000);
    const frac = val % BigInt(1000000);
    return `${whole}.${frac.toString().padStart(6, '0').slice(0, 2)}`;
  }

  function shortToken(id: string): string {
    if (id.length <= 24) return id;
    return id.slice(0, 12) + '...' + id.slice(-8);
  }

  let copied = $state(false);

  function copyAddress() {
    navigator.clipboard.writeText(accountId).then(() => {
      copied = true;
      setTimeout(() => (copied = false), 2000);
    });
  }

  function handleWithdraw(tokenId: string) {
    withdrawing = tokenId;
    showTxToast(
      contract.withdrawToken(tokenId).finally(() => {
        withdrawing = null;
        setTimeout(loadTokens, 4000);
      })
    );
  }

  async function loadTokens() {
    try {
      tokenBalances = await contract.getTokens(accountId);
    } catch {
      tokenBalances = [];
    }
  }

  onMount(async () => {
    try {
      const [s, accountData, ach, qc, bs] = await Promise.all([
        api.accountStats(accountId),
        contract.getAccount(accountId).catch(() => null),
        contract.getAchievements(accountId).catch(() => []),
        contract.getQuestCooldowns(accountId).catch(() => []),
        api.betStats(accountId).catch(() => null)
      ]);
      stats = s;
      if (accountData) {
        elo = accountData.elo;
        points = accountData.points;
      }
      achievements = ach;
      questCooldowns = qc;
      betStats = bs;

      const [account, ag, tb] = await Promise.all([
        api.account(accountId),
        api.activeGame(accountId).catch(() => null),
        loadTokens()
      ]);
      if (ag) activeGame = ag as unknown as GameOverview;
      if (account.finishedGameIds.length > 0) {
        games = await api.query(account.finishedGameIds);
      }
    } catch (e) {
      console.error('Failed to load profile:', e);
    } finally {
      loading = false;
    }
  });
</script>

{#if loading}
  <div class="space-y-6 animate-pulse">
    <div class="card">
      <div class="h-5 w-40 rounded bg-white/10 mb-3"></div>
      <div class="grid grid-cols-4 gap-3">
        {#each Array(4) as _}
          <div class="text-center">
            <div class="h-5 w-8 mx-auto rounded bg-white/10 mb-1"></div>
            <div class="h-3 w-10 mx-auto rounded bg-white/5"></div>
          </div>
        {/each}
      </div>
    </div>
    <div class="space-y-2">
      {#each Array(3) as _}
        <div class="card">
          <div class="h-4 w-2/3 rounded bg-white/10 mb-2"></div>
          <div class="h-3 w-1/3 rounded bg-white/5"></div>
        </div>
      {/each}
    </div>
  </div>
{:else}
  <div class="space-y-6">
    <section class="card">
      <div class="flex items-center gap-2 mb-1">
        <h2 class="text-lg font-bold text-primary">{truncateAddr(accountId)}</h2>
        <button
          onclick={copyAddress}
          class="text-white/40 hover:text-white/80 transition-colors"
          title="Copy address"
        >
          {#if copied}
            <svg xmlns="http://www.w3.org/2000/svg" width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><polyline points="20 6 9 17 4 12"/></svg>
          {:else}
            <svg xmlns="http://www.w3.org/2000/svg" width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><rect width="14" height="14" x="8" y="8" rx="2" ry="2"/><path d="M4 16c-1.1 0-2-.9-2-2V4c0-1.1.9-2 2-2h10c1.1 0 2 .9 2 2"/></svg>
          {/if}
        </button>
        <a
          href="https://near.rocks/account/{accountId}"
          target="_blank"
          rel="noopener"
          class="text-white/40 hover:text-white/80 transition-colors"
          title="View on Explorer"
        >
          <svg xmlns="http://www.w3.org/2000/svg" width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><path d="M18 13v6a2 2 0 0 1-2 2H5a2 2 0 0 1-2-2V8a2 2 0 0 1 2-2h6"/><polyline points="15 3 21 3 21 9"/><line x1="10" x2="21" y1="14" y2="3"/></svg>
        </a>
      </div>

      <div
        class="grid {points !== null
          ? 'grid-cols-2'
          : 'grid-cols-1'} gap-3 mb-3"
      >
        <div class="text-center bg-primary-transparent2 rounded p-2">
          <div class="text-xl font-bold text-primary-warn">{elo ?? 1000}</div>
          <div class="text-xs text-white/50">ELO</div>
        </div>
        {#if points !== null}
          <div class="text-center bg-primary-transparent2 rounded p-2">
            <div class="text-xl font-bold text-primary">
              {formatPoints(points)} PPP
            </div>
            <div class="text-xs text-white/50">Points</div>
          </div>
        {/if}
      </div>

      {#if stats}
        <div class="grid grid-cols-4 gap-3">
          <div class="text-center">
            <div class="text-lg font-bold text-primary-green">{stats.wins}</div>
            <div class="text-xs text-white/50">Wins</div>
          </div>
          <div class="text-center">
            <div class="text-lg font-bold text-primary-err">{stats.losses}</div>
            <div class="text-xs text-white/50">Losses</div>
          </div>
          <div class="text-center">
            <div class="text-lg font-bold">{stats.draws}</div>
            <div class="text-xs text-white/50">Draws</div>
          </div>
          <div class="text-center">
            <div class="text-lg font-bold text-primary">
              {stats.total_games > 0
                ? fmtOneDecimal((stats.wins / stats.total_games) * 100)
                : 0}%
            </div>
            <div class="text-xs text-white/50">Win Rate</div>
          </div>
        </div>
      {/if}
    </section>

    {#if betStats && betStats.total_bets > 0}
      <section class="card">
        <h3 class="text-base font-semibold mb-2">Betting Stats</h3>
        <div class="grid grid-cols-4 gap-3">
          <div class="text-center">
            <div class="text-lg font-bold text-primary">{betStats.total_bets}</div>
            <div class="text-xs text-white/50">Total</div>
          </div>
          <div class="text-center">
            <div class="text-lg font-bold text-primary-warn">{betStats.total_wagered}</div>
            <div class="text-xs text-white/50">Wagered</div>
          </div>
          <div class="text-center">
            <div class="text-lg font-bold text-primary-green">{betStats.won_bets}</div>
            <div class="text-xs text-white/50">Won</div>
          </div>
          <div class="text-center">
            <div class="text-lg font-bold text-primary-green">{betStats.total_won}</div>
            <div class="text-xs text-white/50">Earned</div>
          </div>
        </div>
      </section>
    {/if}

    {#if tokenBalances.length > 0}
      <section class="card space-y-2">
        <h3 class="text-base font-semibold mb-1">Token Balances</h3>
        {#each tokenBalances as [tokenId, balance]}
          <div class="flex items-center justify-between text-sm">
            <span class="text-white/70 truncate mr-2">{shortToken(tokenId)}</span>
            <div class="flex items-center gap-2 shrink-0">
              <span class="text-white/90">{balance}</span>
              {#if $accountStore === accountId}
                <button
                  class="btn-secondary text-xs py-0.5 px-2"
                  disabled={withdrawing === tokenId}
                  onclick={() => handleWithdraw(tokenId)}
                >
                  {withdrawing === tokenId ? '...' : 'Withdraw'}
                </button>
              {/if}
            </div>
          </div>
        {/each}
      </section>
    {/if}

    {#if activeGame}
      <section>
        <h3 class="text-base font-semibold mb-2">Currently Playing</h3>
        <a
          href="/game/{encodeURIComponent(JSON.stringify(activeGame.game_id))}"
        >
          <div class="card-accent flex items-center justify-between">
            <div class="text-sm">
              <span
                class="inline-block w-3 h-3 rounded-full bg-white mr-1 align-middle"
              ></span>
              {activeGame.white.type === 'Human'
                ? activeGame.white.value
                : 'AI'}
              <span class="text-white/40 mx-1">vs</span>
              {activeGame.black?.type === 'Human'
                ? activeGame.black.value
                : 'AI'}
            </div>
            <span class="text-xs text-primary-green">Live</span>
          </div>
        </a>
      </section>
    {/if}

    {#if questCooldowns.length > 0}
      <section class="card">
        <h3 class="text-base font-semibold mb-2">Quest Status</h3>
        <div class="space-y-1.5">
          {#each questCooldowns as [ts, quest]}
            <div class="flex justify-between items-center text-sm">
              <span class="text-white/70">{QUEST_LABELS[quest] ?? quest}</span>
              <span
                class="text-xs {formatCooldown(ts) === 'Ready'
                  ? 'text-primary-green'
                  : 'text-white/50'}"
              >
                {formatCooldown(ts)}
              </span>
            </div>
          {/each}
        </div>
      </section>
    {/if}

    {#if achievements.length > 0}
      <section class="card">
        <h3 class="text-base font-semibold mb-2">Achievements</h3>
        <div class="flex flex-wrap gap-2">
          {#each achievements as [ts, ach]}
            <span
              class="text-xs px-2 py-1 rounded bg-primary-transparent2 text-primary border border-white/20"
            >
              {ACHIEVEMENT_LABELS[ach] ?? ach}
            </span>
          {/each}
        </div>
      </section>
    {/if}

    <section>
      <h3 class="text-base font-semibold mb-2">Finished Games</h3>
      {#if games.length === 0}
        <p class="text-white/50 text-sm">No games yet</p>
      {:else}
        <div class="space-y-2">
          {#each games as game}
            <a class="block" href="/game/{encodeURIComponent(JSON.stringify(game.game_id))}">
              <GameCard {game} />
            </a>
          {/each}
        </div>
      {/if}
    </section>
  </div>
{/if}
