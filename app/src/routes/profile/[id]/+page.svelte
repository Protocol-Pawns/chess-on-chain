<script lang="ts">
  import { onMount } from 'svelte';
  import { page } from '$app/state';
  import { api, type AccountStats, type GameOverview } from '$lib/api/client';
  import { contract } from '$lib/near/connector';
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

  onMount(async () => {
    try {
      const [s, accountData, ach, qc] = await Promise.all([
        api.accountStats(accountId),
        contract.getAccount(accountId).catch(() => null),
        contract.getAchievements(accountId).catch(() => []),
        contract.getQuestCooldowns(accountId).catch(() => [])
      ]);
      stats = s;
      if (accountData) {
        elo = accountData.elo;
        points = accountData.points;
      }
      achievements = ach;
      questCooldowns = qc;

      const [account, ag] = await Promise.all([
        api.account(accountId),
        api.activeGame(accountId).catch(() => null)
      ]);
      if (ag) activeGame = ag as unknown as GameOverview;
      if (account.finishedGameIds.length > 0) {
        const res = await fetch('https://api.protocol-pawns.com/query', {
          method: 'POST',
          headers: { 'Content-Type': 'application/json' },
          body: JSON.stringify({ gameIds: account.finishedGameIds })
        });
        if (res.ok) games = await res.json();
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
      <h2 class="text-lg font-bold mb-1 text-primary">{accountId}</h2>

      {#if elo !== null || points !== null}
        <div
          class="grid {elo !== null && points !== null
            ? 'grid-cols-2'
            : 'grid-cols-1'} gap-3 mb-3"
        >
          {#if elo !== null}
            <div class="text-center bg-primary-transparent2 rounded p-2">
              <div class="text-xl font-bold text-primary-warn">{elo}</div>
              <div class="text-xs text-white/50">ELO</div>
            </div>
          {/if}
          {#if points !== null}
            <div class="text-center bg-primary-transparent2 rounded p-2">
              <div class="text-xl font-bold text-primary">
                {formatPoints(points)} PPP
              </div>
              <div class="text-xs text-white/50">Points</div>
            </div>
          {/if}
        </div>
      {/if}

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
                ? ((stats.wins / stats.total_games) * 100).toFixed(1)
                : 0}%
            </div>
            <div class="text-xs text-white/50">Win Rate</div>
          </div>
        </div>
      {/if}
    </section>

    {#if activeGame}
      <section>
        <h3 class="text-base font-semibold mb-2">Currently Playing</h3>
        <a
          href="/game/{encodeURIComponent(JSON.stringify(activeGame.game_id))}"
        >
          <div class="card flex items-center justify-between">
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
              class="text-xs px-2 py-1 rounded bg-primary-transparent2 text-primary border border-primary/30"
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
            <a href="/game/{encodeURIComponent(JSON.stringify(game.game_id))}">
              <GameCard {game} />
            </a>
          {/each}
        </div>
      {/if}
    </section>
  </div>
{/if}
