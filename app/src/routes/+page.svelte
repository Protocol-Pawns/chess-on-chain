<script lang="ts">
  import { onMount } from 'svelte';
  import { goto } from '$app/navigation';
  import { api, type GlobalStats, type GameOverview } from '$lib/api/client';
  import { contract } from '$lib/near/connector';
  import {
    isLoggedIn,
    accountStore,
    isRegistered,
    isCheckingRegistration,
    register
  } from '$lib/near/account';
  import { showToast, showTxToast, decodeSuccessValue } from '$lib/toast';
  import { loadGameFromContract, gameUrl } from '$lib/game';
  import type { GameId } from '$lib/game';
  import GameCard from '$lib/components/GameCard.svelte';
  import PushSettings from '$lib/components/PushSettings.svelte';

  let stats = $state<GlobalStats | null>(null);
  let myGames = $state<GameOverview[]>([]);
  let activeGames = $state<GameOverview[]>([]);
  let finishedGames = $state<GameOverview[]>([]);
  let finishedCursor = $state<string | null>(null);
  let loadingMore = $state(false);
  let loading = $state(true);
  let showAiMenu = $state(false);

  async function loadMyGames() {
    if (!$accountStore) {
      console.log('[loadMyGames] no accountStore, skipping');
      return;
    }
    try {
      console.log('[loadMyGames] fetching game IDs for', $accountStore);
      const gameIds: GameId[] = await contract.getGameIds($accountStore);
      console.log('[loadMyGames] gameIds:', JSON.stringify(gameIds));
      if (gameIds.length === 0) {
        console.log('[loadMyGames] no game IDs returned');
        myGames = [];
        return;
      }

      let apiGames: GameOverview[] = [];
      try {
        apiGames = await api.query(gameIds);
        console.log(
          '[loadMyGames] api.query returned',
          apiGames.length,
          'games'
        );
      } catch (e) {
        console.warn(
          '[loadMyGames] api.query failed, will use contract fallback:',
          e
        );
      }

      const foundIds = new Set(apiGames.map(g => JSON.stringify(g.game_id)));
      const missingIds = gameIds.filter(
        id => !foundIds.has(JSON.stringify(id))
      );

      if (missingIds.length === 0) {
        myGames = apiGames;
        return;
      }

      console.log(
        '[loadMyGames] fetching',
        missingIds.length,
        'games from contract fallback'
      );
      const contractGames = await Promise.all(
        missingIds.map(id => loadGameFromContract(id))
      );
      myGames = [...apiGames, ...contractGames];
      console.log('[loadMyGames] total myGames:', myGames.length);
    } catch (e) {
      console.error('[loadMyGames] FAILED:', e);
    }
  }

  async function loadLobby() {
    try {
      const [s, ag, fg] = await Promise.all([
        api.stats(),
        api.games('active', undefined, 20),
        api.games('finished', undefined, 20)
      ]);
      stats = s;
      activeGames = ag.items;
      finishedGames = fg.items;
      finishedCursor = fg.next_cursor;
    } catch (e) {
      console.error('Failed to load lobby data:', e);
    }
  }

  onMount(() => {
    loadLobby().then(() => {
      loading = false;
    });
  });

  $effect(() => {
    if ($accountStore) {
      loadMyGames();
    }
  });

  function navigateToGame(gameId: GameId) {
    goto(gameUrl(gameId));
  }

  function createAiGame(difficulty: 'Easy' | 'Medium' | 'Hard') {
    showAiMenu = false;
    showToast('info', 'Creating AI game...');
    contract
      .createAiGame(difficulty)
      .then(result => {
        const gameId = decodeSuccessValue<GameId>(result);
        if (gameId) {
          showToast('success', 'Game created! Redirecting...');
          setTimeout(() => navigateToGame(gameId), 1000);
        } else {
          showToast('success', 'Game created!');
          loadMyGames();
        }
      })
      .catch((err: unknown) => {
        const msg = err instanceof Error ? err.message : String(err);
        showToast('error', 'Failed to create game', msg);
      });
  }

  async function loadMoreFinished() {
    if (!finishedCursor || loadingMore) return;
    loadingMore = true;
    try {
      const res = await api.games('finished', finishedCursor, 20);
      finishedGames = [...finishedGames, ...res.items];
      finishedCursor = res.next_cursor;
    } catch (e) {
      console.error('Failed to load more:', e);
    } finally {
      loadingMore = false;
    }
  }
</script>

<div class="flex flex-col gap-3">
  <section class="text-center flex flex-col gap-3">
    <h2 class="text-xl font-bold text-primary">Welcome to Protocol Pawns!</h2>
    <p class="text-sm text-white/80 leading-relaxed">
      Protocol Pawns is the very first fully decentralized on-chain chess game
      built on NEAR Protocol. Challenge other wallets to play against you or
      play against an AI. Earn points by playing and winning. Complete recurring
      quests and collect achievements!
    </p>
    <p class="text-sm text-white/60">
      Learn more about the game in the <a
        href="/about"
        class="text-primary hover:underline">about section</a
      >.
    </p>
  </section>

  {#if !$isLoggedIn}
    <div class="card border-primary-info text-sm text-center py-3">
      Please login in order to play chess via Protocol Pawns!
    </div>
  {:else if !$isRegistered && !$isCheckingRegistration}
    <div class="card text-sm flex flex-col gap-2">
      <p>
        In order to play you first need to register your account. This will cost
        a small fee of 0.05 N in order for the contract to pay for the used
        storage.
      </p>
      <button
        class="btn-primary text-sm self-start"
        onclick={() => showTxToast(register())}
      >
        Register
      </button>
    </div>
  {:else if $isRegistered}
    <div class="flex gap-2 justify-center">
      <a href="/challenges" class="btn-primary text-sm">Challenge Player</a>
      <div class="relative">
        <button
          class="btn-primary text-sm"
          onclick={() => (showAiMenu = !showAiMenu)}
        >
          Play vs AI
        </button>
        {#if showAiMenu}
          <div
            class="absolute right-0 top-full mt-1 card min-w-28 z-50 space-y-1 bg-[#1a1a2e]"
          >
            <button
              class="btn-secondary w-full text-left text-sm"
              onclick={() => createAiGame('Easy')}>Easy</button
            >
            <button
              class="btn-secondary w-full text-left text-sm"
              onclick={() => createAiGame('Medium')}>Medium</button
            >
            <button
              class="btn-secondary w-full text-left text-sm"
              onclick={() => createAiGame('Hard')}>Hard</button
            >
          </div>
        {/if}
      </div>
    </div>
    <PushSettings />
  {/if}

  {#if myGames.length > 0}
    <section>
      <h3 class="text-base font-semibold mb-2">My Games</h3>
      <div class="space-y-2 overflow-hidden" style="transition: max-height 0.3s ease-out; max-height: {myGames.length * 150}px;">
        {#each myGames as game}
          <button
            class="w-full text-left"
            onclick={() => navigateToGame(game.game_id)}
          >
            <GameCard {game} />
          </button>
        {/each}
      </div>
    </section>
  {/if}

  {#if loading}
    <div class="grid grid-cols-2 gap-3">
      {#each Array(4) as _}
        <div class="card text-center animate-pulse">
          <div class="h-6 w-8 mx-auto rounded bg-white/10 mb-1"></div>
          <div class="h-3 w-16 mx-auto rounded bg-white/5"></div>
        </div>
      {/each}
    </div>
  {:else}
    {#if stats}
      <section>
        <div class="grid grid-cols-2 gap-3">
          <div class="card text-center">
            <div class="text-xl font-bold text-primary">
              {stats.total_games}
            </div>
            <div class="text-xs text-white/50">Total Games</div>
          </div>
          <div class="card text-center">
            <div class="text-xl font-bold text-primary-green">
              {stats.active_games}
            </div>
            <div class="text-xs text-white/50">Active</div>
          </div>
          <div class="card text-center">
            <div class="text-xl font-bold">{stats.finished_games}</div>
            <div class="text-xs text-white/50">Finished</div>
          </div>
          <div class="card text-center">
            <div class="text-xl font-bold">{stats.total_moves}</div>
            <div class="text-xs text-white/50">Total Moves</div>
          </div>
        </div>
      </section>
    {/if}

    <section class="space-y-6">
      {#if activeGames.length > 0}
        <div>
          <h3 class="text-base font-semibold mb-2">Active Games</h3>
          <div class="space-y-2">
            {#each activeGames as game}
              <a
                class="block"
                href="/game/{encodeURIComponent(JSON.stringify(game.game_id))}"
              >
                <GameCard {game} />
              </a>
            {/each}
          </div>
        </div>
      {/if}

      {#if finishedGames.length > 0}
        <div>
          <h3 class="text-base font-semibold mb-2">Recent Games</h3>
          <div class="space-y-2">
            {#each finishedGames as game}
              <a
                class="block"
                href="/game/{encodeURIComponent(JSON.stringify(game.game_id))}"
              >
                <GameCard {game} />
              </a>
            {/each}
          </div>
          {#if finishedCursor}
            <button
              class="btn-secondary text-sm w-full mt-3"
              onclick={loadMoreFinished}
              disabled={loadingMore}
            >
              {loadingMore ? 'Loading...' : 'Load More'}
            </button>
          {/if}
        </div>
      {/if}
    </section>
  {/if}
</div>
