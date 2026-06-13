<script lang="ts">
  import { onMount } from 'svelte';
  import { page } from '$app/state';
  import {
    api,
    type AccountStats,
    type GameOverview,
    type BetStats,
    type Challenge
  } from '$lib/api/client';
  import { fmtOneDecimal, truncateAddr } from '$lib/format';
  import { contract } from '$lib/near/connector';
  import { accountStore } from '$lib/near/account';
  import { loadGameFromContract, MAX_OPEN_GAMES, gameUrl } from '$lib/game';
  import type { GameId } from '$lib/game';
  import type { QuestInfo } from '$lib/near/contract-types';
  import { showToast, showTxToast, decodeSuccessValue } from '$lib/toast';
  import { goto } from '$app/navigation';
  import GameCard from '$lib/components/GameCard.svelte';
  import ChallengeCard from '$lib/components/ChallengeCard.svelte';
  import ConfirmModal from '$lib/components/ConfirmModal.svelte';
  import PppIcon from '$lib/components/PppIcon.svelte';

  let accountId = $state(page.params.id ?? '');

  let stats = $state<AccountStats | null>(null);
  let games = $state<GameOverview[]>([]);
  let excludeAi = $state(true);
  let activeGames = $state<GameOverview[]>([]);
  let elo = $state<number | null>(null);
  let points = $state<string | null>(null);
  let wins = $state(0);
  let winStreak = $state(0);
  let maxWinStreak = $state(0);
  let betsPlaced = $state(0);
  let betsWon = $state(0);
  let wagersPlayed = $state(0);
  let wagerWins = $state(0);
  let challengesSent = $state(0);
  let achievements: Array<[number, string]> = $state([]);
  let questCooldowns: Array<[number, string]> = $state([]);
  let questList: QuestInfo[] = $state([]);
  let loading = $state(true);
  let betStats = $state<BetStats | null>(null);
  let tokenBalances = $state<Array<[string, string]>>([]);
  let withdrawing = $state<string | null>(null);
  let pendingChallenges = $state<Challenge[]>([]);
  let acceptTarget = $state<Challenge | null>(null);
  let rejectTarget = $state<Challenge | null>(null);
  let cancelTarget = $state<Challenge | null>(null);

  const ACHIEVEMENT_LABELS: Record<string, string> = {
    FirstWinHuman: 'First Win vs Human',
    FirstWin: 'First Win vs Human',
    FirstWinAiEasy: 'First Win vs AI (Easy)',
    FirstWinAiMedium: 'First Win vs AI (Medium)',
    FirstWinAiHard: 'First Win vs AI (Hard)',
    FirstWinAiVeryHard: 'First Win vs AI (Very Hard)',
    Wins10: '10 Wins',
    Wins50: '50 Wins',
    Wins100: '100 Wins',
    Wins500: '500 Wins',
    WinStreak3: '3 Win Streak',
    WinStreak5: '5 Win Streak',
    WinStreak10: '10 Win Streak',
    WinStreak25: '25 Win Streak',
    FirstBet: 'First Bet',
    FirstBetWin: 'First Bet Win',
    BetsWon10: '10 Bets Won',
    BetsWon100: '100 Bets Won',
    FirstWager: 'First Wager',
    FirstWagerWin: 'First Wager Win',
    WagerWins10: '10 Wager Wins',
    WagerWins100: '100 Wager Wins',
    Elo1100: '1100 ELO',
    Elo1200: '1200 ELO',
    Elo1300: '1300 ELO',
    Elo1400: '1400 ELO',
    Elo1500: '1500 ELO',
    FirstChallenge: 'First Challenge',
    Challenges10: '10 Challenges',
    Challenges100: '100 Challenges'
  };

  const QUEST_LABELS: Record<string, string> = {
    DailyPlayMove: 'Daily Move',
    DailyGame: 'Daily Game',
    WeeklyWin: 'Weekly Win',
    WeeklyBettor: 'Weekly Bet',
    WeeklyChallenger: 'Weekly Challenge'
  };

  function formatCooldown(timestampMs: number, cooldownMs: number): string {
    const remaining = timestampMs + cooldownMs - Date.now();
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

  let filteredGames = $derived(
    excludeAi
      ? games.filter(g => g.white.type === 'Human' && g.black?.type === 'Human')
      : games
  );

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

  async function loadPendingChallenges() {
    if ($accountStore !== accountId) return;
    try {
      const all = await api.challenges(accountId);
      const items = 'items' in all ? all.items : all;
      pendingChallenges = items.filter(c => c.status === 'pending');
    } catch (e) {
      console.error('Failed to load pending challenges:', e);
    }
  }

  function doAccept(challenge: Challenge) {
    acceptTarget = null;
    showToast('info', 'Accepting challenge...');
    const promise =
      challenge.wager_token && challenge.wager_amount
        ? contract.acceptChallengeWithWager(
            challenge.wager_token,
            challenge.id,
            challenge.wager_amount
          )
        : contract.acceptChallenge(challenge.id);

    promise
      .then(result => {
        const gameId = decodeSuccessValue<GameId>(result);
        const idx = pendingChallenges.findIndex(c => c.id === challenge.id);
        if (idx !== -1) {
          pendingChallenges[idx] = {
            ...pendingChallenges[idx],
            status: 'accepted',
            game_id: gameId ? JSON.stringify(gameId) : null
          };
          pendingChallenges = pendingChallenges;
        }
        if (gameId) {
          showToast('success', 'Challenge accepted! Redirecting...');
          setTimeout(() => goto(gameUrl(gameId)), 1000);
        } else {
          showToast('success', 'Challenge accepted!');
        }
        setTimeout(loadPendingChallenges, 10000);
      })
      .catch((err: unknown) => {
        const msg = err instanceof Error ? err.message : String(err);
        showToast('error', 'Failed to accept challenge', msg);
      });
  }

  function doReject(challenge: Challenge) {
    rejectTarget = null;
    const isChallenger = $accountStore === challenge.challenger;
    const p = contract.rejectChallenge(challenge.id, isChallenger);
    showTxToast(p);
    p.then(() => {
      const idx = pendingChallenges.findIndex(c => c.id === challenge.id);
      if (idx !== -1) {
        pendingChallenges[idx] = {
          ...pendingChallenges[idx],
          status: 'rejected'
        };
        pendingChallenges = pendingChallenges;
      }
      setTimeout(loadPendingChallenges, 10000);
    }).catch(() => {});
  }

  function doCancel(challenge: Challenge) {
    cancelTarget = null;
    const p = contract.rejectChallenge(challenge.id, true);
    showTxToast(p);
    p.then(() => {
      const idx = pendingChallenges.findIndex(c => c.id === challenge.id);
      if (idx !== -1) {
        pendingChallenges[idx] = {
          ...pendingChallenges[idx],
          status: 'rejected'
        };
        pendingChallenges = pendingChallenges;
      }
      setTimeout(loadPendingChallenges, 10000);
    }).catch(() => {});
  }

  async function loadProfileData(id: string) {
    try {
      const [s, accountData, ach, qc, ql, bs] = await Promise.all([
        api.accountStats(id),
        contract.getAccount(id).catch(() => null),
        contract.getAchievements(id).catch(() => []),
        contract.getQuestCooldowns(id).catch(() => []),
        contract.getQuestList().catch(() => []),
        api.betStats(id).catch(() => null)
      ]);
      stats = s;
      if (accountData) {
        elo = accountData.elo ?? null;
        points = accountData.points;
        wins = accountData.wins ?? 0;
        winStreak = accountData.win_streak ?? 0;
        maxWinStreak = accountData.max_win_streak ?? 0;
        betsPlaced = accountData.bets_placed ?? 0;
        betsWon = accountData.bets_won ?? 0;
        wagersPlayed = accountData.wagers_played ?? 0;
        wagerWins = accountData.wager_wins ?? 0;
        challengesSent = accountData.challenges_sent ?? 0;
      }
      achievements = ach;
      questCooldowns = qc;
      questList = ql;
      betStats = bs;

      await loadTokens();
      try {
        const [contractGameIds, accountData] = await Promise.all([
          contract.getGameIds(id).catch((): GameId[] => []),
          api.account(id).catch(() => ({ finishedGameIds: [] as GameId[] }))
        ]);
        const seen = new Set<string>();
        const allGameIds: GameId[] = [];
        for (const gid of [
          ...contractGameIds,
          ...accountData.finishedGameIds
        ]) {
          const key = JSON.stringify(gid);
          if (!seen.has(key)) {
            seen.add(key);
            allGameIds.push(gid);
          }
        }
        if (allGameIds.length > 0) {
          let apiGames: GameOverview[] = [];
          try {
            apiGames = await api.query(allGameIds);
          } catch {}
          const foundIds = new Set(
            apiGames.map(g => JSON.stringify(g.game_id))
          );
          const missingIds = allGameIds.filter(
            gid => !foundIds.has(JSON.stringify(gid))
          );
          if (missingIds.length > 0) {
            const contractGames = await Promise.all(
              missingIds.map(gid => loadGameFromContract(gid))
            );
            apiGames = [...apiGames, ...contractGames];
          }
          activeGames = apiGames.filter(g => g.status !== 'finished');
          games = apiGames
            .filter(g => g.status === 'finished')
            .sort(
              (a, b) =>
                new Date(b.finished_at ?? b.created_at ?? 0).getTime() -
                new Date(a.finished_at ?? a.created_at ?? 0).getTime()
            );
        }
      } catch {}
      loadPendingChallenges();
    } catch (e) {
      console.error('Failed to load profile:', e);
    } finally {
      loading = false;
    }
  }

  $effect(() => {
    const newId = page.params.id ?? '';
    if (newId !== accountId) {
      stats = null;
      games = [];
      activeGames = [];
      elo = null;
      points = null;
      wins = 0;
      winStreak = 0;
      maxWinStreak = 0;
      betsPlaced = 0;
      betsWon = 0;
      wagersPlayed = 0;
      wagerWins = 0;
      challengesSent = 0;
      achievements = [];
      questCooldowns = [];
      questList = [];
      betStats = null;
      tokenBalances = [];
      pendingChallenges = [];
      loading = true;

      accountId = newId;
      loadProfileData(accountId);
    }
  });

  onMount(() => loadProfileData(accountId));
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
        <h2 class="text-lg font-bold text-primary">
          {truncateAddr(accountId)}
        </h2>
        <button
          onclick={copyAddress}
          class="text-white/40 hover:text-white/80 transition-colors"
          title="Copy address"
        >
          {#if copied}
            <svg
              xmlns="http://www.w3.org/2000/svg"
              width="16"
              height="16"
              viewBox="0 0 24 24"
              fill="none"
              stroke="currentColor"
              stroke-width="2"
              stroke-linecap="round"
              stroke-linejoin="round"><polyline points="20 6 9 17 4 12" /></svg
            >
          {:else}
            <svg
              xmlns="http://www.w3.org/2000/svg"
              width="16"
              height="16"
              viewBox="0 0 24 24"
              fill="none"
              stroke="currentColor"
              stroke-width="2"
              stroke-linecap="round"
              stroke-linejoin="round"
              ><rect width="14" height="14" x="8" y="8" rx="2" ry="2" /><path
                d="M4 16c-1.1 0-2-.9-2-2V4c0-1.1.9-2 2-2h10c1.1 0 2 .9 2 2"
              /></svg
            >
          {/if}
        </button>
        <a
          href="https://near.rocks/account/{accountId}"
          target="_blank"
          rel="noopener"
          class="text-white/40 hover:text-white/80 transition-colors"
          title="View on Explorer"
        >
          <svg
            xmlns="http://www.w3.org/2000/svg"
            width="16"
            height="16"
            viewBox="0 0 24 24"
            fill="none"
            stroke="currentColor"
            stroke-width="2"
            stroke-linecap="round"
            stroke-linejoin="round"
            ><path
              d="M18 13v6a2 2 0 0 1-2 2H5a2 2 0 0 1-2-2V8a2 2 0 0 1 2-2h6"
            /><polyline points="15 3 21 3 21 9" /><line
              x1="10"
              x2="21"
              y1="14"
              y2="3"
            /></svg
          >
        </a>
      </div>

      {#if $accountStore && $accountStore !== accountId}
        <button
          class="btn-primary text-sm w-full mt-3"
          onclick={() =>
            goto('/challenges?target=' + encodeURIComponent(accountId))}
        >
          Send Challenge
        </button>
      {/if}

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
              <PppIcon size={22} />
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

      {#if winStreak > 0 || maxWinStreak > 0 || betsPlaced > 0 || wagersPlayed > 0 || challengesSent > 0}
        <div class="grid grid-cols-3 gap-3 mt-3">
          {#if winStreak > 0 || maxWinStreak > 0}
            <div class="text-center bg-white/5 rounded p-2">
              <div class="text-sm font-bold text-primary-warn">
                {winStreak}<span class="text-white/30 text-xs"
                  >/{maxWinStreak}</span
                >
              </div>
              <div class="text-xs text-white/50">Streak / Best</div>
            </div>
          {/if}
          {#if betsPlaced > 0}
            <div class="text-center bg-white/5 rounded p-2">
              <div class="text-sm font-bold text-primary-green">
                {betsWon}<span class="text-white/30 text-xs">/{betsPlaced}</span
                >
              </div>
              <div class="text-xs text-white/50">Bets Won</div>
            </div>
          {/if}
          {#if wagersPlayed > 0}
            <div class="text-center bg-white/5 rounded p-2">
              <div class="text-sm font-bold text-primary">
                {wagerWins}<span class="text-white/30 text-xs"
                  >/{wagersPlayed}</span
                >
              </div>
              <div class="text-xs text-white/50">Wagers Won</div>
            </div>
          {/if}
          {#if challengesSent > 0}
            <div class="text-center bg-white/5 rounded p-2">
              <div class="text-sm font-bold">{challengesSent}</div>
              <div class="text-xs text-white/50">Challenges</div>
            </div>
          {/if}
        </div>
      {/if}
    </section>

    {#if betStats && betStats.total_bets > 0}
      <section class="card">
        <h3 class="text-base font-semibold mb-2">Betting Stats</h3>
        <div class="grid grid-cols-2 gap-3">
          <div class="text-center">
            <div class="text-lg font-bold text-primary">
              {betStats.total_bets}
            </div>
            <div class="text-xs text-white/50">Total</div>
          </div>
          <div class="text-center">
            <div class="text-lg font-bold text-primary-green">
              {betStats.won_bets}
            </div>
            <div class="text-xs text-white/50">Won</div>
          </div>
        </div>
        {#if Object.keys(betStats.by_token).length > 0}
          <div class="mt-3 space-y-1">
            <h4 class="text-sm font-semibold text-white/70">
              Wagered / Earned by Token
            </h4>
            {#each Object.entries(betStats.by_token) as [tokenId, ts]}
              <div class="flex items-center justify-between text-sm">
                <span class="text-white/70 truncate mr-2"
                  >{shortToken(tokenId)}</span
                >
                <div class="flex items-center gap-3 shrink-0">
                  <span class="text-primary-warn">{ts.wagered}</span>
                  <span class="text-white/30">/</span>
                  <span class="text-primary-green">{ts.won}</span>
                </div>
              </div>
            {/each}
          </div>
        {/if}
      </section>
    {/if}

    {#if tokenBalances.length > 0}
      <section class="card space-y-2">
        <h3 class="text-base font-semibold mb-1">Token Balances</h3>
        {#each tokenBalances as [tokenId, balance]}
          <div class="flex items-center justify-between text-sm">
            <span class="text-white/70 truncate mr-2"
              >{shortToken(tokenId)}</span
            >
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

    {#if pendingChallenges.length > 0}
      <section>
        <h3 class="text-base font-semibold mb-2">
          Pending Challenges
          <span class="text-xs text-primary-warn ml-1"
            >({pendingChallenges.length})</span
          >
        </h3>
        <div class="space-y-2">
          {#each pendingChallenges as challenge}
            <ChallengeCard
              {challenge}
              currentAccount={$accountStore!}
              gameCount={activeGames.length}
              onaccept={c => (acceptTarget = c)}
              onreject={c => (rejectTarget = c)}
              oncancel={c => (cancelTarget = c)}
            />
          {/each}
        </div>
      </section>
    {/if}

    {#if activeGames.length > 0}
      <section>
        <h3 class="text-base font-semibold mb-2">
          Active Games ({activeGames.length}/{MAX_OPEN_GAMES})
        </h3>
        <div class="space-y-2">
          {#each activeGames as game}
            <a class="block" href={gameUrl(game.game_id)}>
              <GameCard {game} />
            </a>
          {/each}
        </div>
      </section>
    {/if}

    {#if questList.length > 0}
      <section class="card">
        <h3 class="text-base font-semibold mb-2">Quest Status</h3>
        <div class="space-y-1.5">
          {#each questList as quest}
            {@const cooldown = questCooldowns.find(([, q]) => q === quest.name)}
            {@const status = cooldown
              ? formatCooldown(cooldown[0], quest.cooldown)
              : 'Ready'}
            <div class="flex justify-between items-center text-sm">
              <span class="text-white/70"
                >{QUEST_LABELS[quest.name] ?? quest.name}</span
              >
              <span
                class="text-xs {status === 'Ready'
                  ? 'text-primary-green'
                  : 'text-white/50'}"
              >
                {status}
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
      <div class="flex items-center justify-between mb-2">
        <h3 class="text-base font-semibold">Finished Games</h3>
        {#if games.length > 0}
          <label
            class="flex items-center gap-1.5 text-xs text-white/50 cursor-pointer select-none"
          >
            <input type="checkbox" bind:checked={excludeAi} />
            Hide AI games
          </label>
        {/if}
      </div>
      {#if filteredGames.length === 0}
        <p class="text-white/50 text-sm">No games yet</p>
      {:else}
        <div class="space-y-2">
          {#each filteredGames as game}
            <a class="block" href={gameUrl(game.game_id)}>
              <GameCard {game} />
            </a>
          {/each}
        </div>
      {/if}
    </section>
  </div>
{/if}

<ConfirmModal
  open={acceptTarget !== null}
  title="Accept Challenge?"
  message={acceptTarget
    ? `Accept the challenge from ${acceptTarget.challenger}?` +
      (acceptTarget.wager_token && acceptTarget.wager_amount
        ? ` This includes a wager of ${acceptTarget.wager_amount}.`
        : '')
    : ''}
  confirmLabel="Accept"
  confirmClass="btn-primary text-sm"
  onconfirm={() => acceptTarget && doAccept(acceptTarget)}
  onclose={() => (acceptTarget = null)}
/>

<ConfirmModal
  open={rejectTarget !== null}
  title="Reject Challenge?"
  message={rejectTarget
    ? `Reject the challenge from ${rejectTarget.challenger}?`
    : ''}
  confirmLabel="Reject"
  onconfirm={() => rejectTarget && doReject(rejectTarget)}
  onclose={() => (rejectTarget = null)}
/>

<ConfirmModal
  open={cancelTarget !== null}
  title="Cancel Challenge?"
  message={cancelTarget
    ? `Cancel your challenge to ${cancelTarget.challenged}?`
    : ''}
  confirmLabel="Cancel"
  onconfirm={() => cancelTarget && doCancel(cancelTarget)}
  onclose={() => (cancelTarget = null)}
/>
