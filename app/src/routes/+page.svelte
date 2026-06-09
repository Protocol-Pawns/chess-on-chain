<script lang="ts">
  import { onMount } from 'svelte';
  import { goto } from '$app/navigation';
  import {
    api,
    type GlobalStats,
    type GameOverview,
    type Challenge
  } from '$lib/api/client';
  import { contract } from '$lib/near/connector';
  import {
    isLoggedIn,
    accountStore,
    isRegistered,
    isCheckingRegistration,
    register
  } from '$lib/near/account';
  import { showToast, showTxToast, decodeSuccessValue } from '$lib/toast';
  import { loadGameFromContract, gameUrl, MAX_OPEN_GAMES } from '$lib/game';
  import type { GameId } from '$lib/game';
  import GameCard from '$lib/components/GameCard.svelte';
  import ChallengeCard from '$lib/components/ChallengeCard.svelte';
  import Pagination from '$lib/components/Pagination.svelte';
  import PushSettings from '$lib/components/PushSettings.svelte';
  import PwaInstallCard from '$lib/components/PwaInstallCard.svelte';
  import ConfirmModal from '$lib/components/ConfirmModal.svelte';

  const PER_PAGE = 10;

  let stats = $state<GlobalStats | null>(null);
  let myGames = $state<GameOverview[]>([]);
  let finishedGames = $state<GameOverview[]>([]);
  let finishedPage = $state(1);
  let finishedTotalPages = $state(1);
  let excludeAi = $state(true);
  let loadingMore = $state(false);
  let loading = $state(true);
  let showAiMenu = $state(false);
  let selectedDifficulty = $state<'Easy' | 'Medium' | 'Hard'>('Easy');
  let showAiConfirm = $state(false);
  let pendingChallenges = $state<Challenge[]>([]);
  let acceptTarget = $state<Challenge | null>(null);
  let rejectTarget = $state<Challenge | null>(null);
  let cancelTarget = $state<Challenge | null>(null);

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

  async function loadPendingChallenges() {
    if (!$accountStore) return;
    try {
      const all = await api.challenges($accountStore);
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
          setTimeout(() => navigateToGame(gameId), 1000);
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

  async function loadLobby() {
    try {
      const [s, fg] = await Promise.all([
        api.stats(),
        api.games('finished', undefined, PER_PAGE, 1, excludeAi)
      ]);
      stats = s;
      finishedGames = fg.items;
      finishedPage = fg.page ?? 1;
      finishedTotalPages = fg.total_pages ?? 1;
    } catch (e) {
      console.error('Failed to load lobby data:', e);
    }
  }

  async function loadFinishedPage(p: number) {
    if (loadingMore) return;
    loadingMore = true;
    try {
      const res = await api.games(
        'finished',
        undefined,
        PER_PAGE,
        p,
        excludeAi
      );
      finishedGames = res.items;
      finishedPage = res.page ?? p;
      finishedTotalPages = res.total_pages ?? 1;
    } catch (e) {
      console.error('Failed to load page:', e);
    } finally {
      loadingMore = false;
    }
  }

  function toggleAiFilter() {
    finishedPage = 1;
    loadFinishedPage(1);
  }

  onMount(() => {
    loadLobby().then(() => {
      loading = false;
    });
  });

  $effect(() => {
    if ($accountStore) {
      loadMyGames();
      loadPendingChallenges();
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
</script>

<div class="flex flex-col gap-5">
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
          disabled={myGames.length >= MAX_OPEN_GAMES}
          title={myGames.length >= MAX_OPEN_GAMES ? 'Max games reached' : ''}
        >
          Play vs AI
        </button>
        {#if showAiMenu}
          <div
            class="fixed inset-0 z-40"
            onclick={() => (showAiMenu = false)}
          ></div>
          <div class="dropdown right-0 top-full mt-1 min-w-28 space-y-0.5">
            <button
              class="btn-secondary w-full text-left text-sm"
              onclick={() => {
                selectedDifficulty = 'Easy';
                showAiMenu = false;
                showAiConfirm = true;
              }}>Easy</button
            >
            <button
              class="btn-secondary w-full text-left text-sm"
              onclick={() => {
                selectedDifficulty = 'Medium';
                showAiMenu = false;
                showAiConfirm = true;
              }}>Medium</button
            >
            <button
              class="btn-secondary w-full text-left text-sm"
              onclick={() => {
                selectedDifficulty = 'Hard';
                showAiMenu = false;
                showAiConfirm = true;
              }}>Hard</button
            >
          </div>
        {/if}
      </div>
    </div>
    <PwaInstallCard />
    <PushSettings />
  {/if}

  {#if $isRegistered && pendingChallenges.length > 0}
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
            gameCount={myGames.length}
            onaccept={c => (acceptTarget = c)}
            onreject={c => (rejectTarget = c)}
            oncancel={c => (cancelTarget = c)}
          />
        {/each}
      </div>
    </section>
  {/if}

  {#if myGames.length > 0}
    <section>
      <h3 class="text-base font-semibold mb-2">
        My Games ({myGames.length}/{MAX_OPEN_GAMES})
      </h3>
      <div
        class="space-y-2 overflow-hidden"
        style="transition: max-height 0.3s ease-out; max-height: {myGames.length *
          150}px;"
      >
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
        <h3 class="text-base font-semibold mb-2">Global Stats</h3>
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
      {#if finishedGames.length > 0 || finishedTotalPages > 0}
        <div>
          <div class="flex items-center justify-between mb-2">
            <h3 class="text-base font-semibold">Recent Games</h3>
            <label
              class="flex items-center gap-1.5 text-xs text-white/50 cursor-pointer select-none"
            >
              <input
                type="checkbox"
                bind:checked={excludeAi}
                onchange={toggleAiFilter}
                class="accent-primary"
              />
              Hide AI games
            </label>
          </div>
          {#if loadingMore && finishedGames.length === 0}
            <div class="space-y-2 animate-pulse">
              {#each Array(3) as _}
                <div class="card">
                  <div class="h-4 w-2/3 rounded bg-white/10 mb-1"></div>
                  <div class="h-3 w-1/3 rounded bg-white/5"></div>
                </div>
              {/each}
            </div>
          {:else}
            <div class="space-y-2">
              {#each finishedGames as game}
                <a
                  class="block"
                  href="/game/{encodeURIComponent(
                    JSON.stringify(game.game_id)
                  )}"
                >
                  <GameCard {game} />
                </a>
              {/each}
            </div>
          {/if}
          <div class="mt-3">
            <Pagination
              page={finishedPage}
              totalPages={finishedTotalPages}
              onchange={p => {
                loadFinishedPage(p);
                window.scrollTo({ top: 0, behavior: 'smooth' });
              }}
            />
          </div>
        </div>
      {/if}
    </section>
  {/if}
</div>

<ConfirmModal
  open={showAiConfirm}
  title="Start AI Game?"
  message={`Start a ${selectedDifficulty.toLowerCase()} AI game? This will create an on-chain game.`}
  confirmLabel="Start Game"
  onconfirm={() => {
    showAiConfirm = false;
    createAiGame(selectedDifficulty);
  }}
  onclose={() => (showAiConfirm = false)}
/>

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
