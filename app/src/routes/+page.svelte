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
  import {
    loadGameFromContract,
    gameUrl,
    findAcceptedGameId,
    MAX_OPEN_GAMES
  } from '$lib/game';
  import type { GameId } from '$lib/game';
  import type { Difficulty } from '$lib/near/contract-types';
  import type { SSEEventData } from '$lib/sse';
  import { subscribe, updateWatermark } from '$lib/sse';
  import GameCard from '$lib/components/GameCard.svelte';
  import ChallengeCard from '$lib/components/ChallengeCard.svelte';
  import Pagination from '$lib/components/Pagination.svelte';
  import PushSettings from '$lib/components/PushSettings.svelte';
  import PwaInstallCard from '$lib/components/PwaInstallCard.svelte';
  import ConfirmModal from '$lib/components/ConfirmModal.svelte';
  import MatchmakingPanel from '$lib/components/MatchmakingPanel.svelte';
  import { searching as mmSearching } from '$lib/near/matchmaking';
  import { formatWager, formatWagerText } from '$lib/wager';
  import { fmtTGas, AI_MOVE_GAS_BUDGET } from '$lib/format';

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
  let selectedDifficulty = $state<Difficulty>('Easy');
  let showAiConfirm = $state(false);
  let showMatchmaking = $state(false);
  let pendingChallenges = $state<Challenge[]>([]);
  let acceptTarget = $state<Challenge | null>(null);
  let rejectTarget = $state<Challenge | null>(null);
  let cancelTarget = $state<Challenge | null>(null);

  let acceptWagerText = $state('');

  $effect(() => {
    const target = acceptTarget;
    acceptWagerText = '';
    if (
      target?.wager_token &&
      target?.wager_amount &&
      target.wager_amount !== '0'
    ) {
      let cancelled = false;
      formatWager(target.wager_amount, target.wager_token).then(d => {
        if (!cancelled) acceptWagerText = formatWagerText(d);
      });
      return () => {
        cancelled = true;
      };
    }
  });

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

  async function doAccept(challenge: Challenge) {
    acceptTarget = null;
    showToast('info', 'Accepting challenge...');
    try {
      let gameId: GameId | null = null;
      if (challenge.wager_token && challenge.wager_amount) {
        const account = $accountStore;
        const before = account ? await contract.getGameIds(account) : [];
        await contract.acceptChallengeWithWager(
          challenge.wager_token,
          challenge.id,
          challenge.wager_amount
        );
        const after = account ? await contract.getGameIds(account) : [];
        gameId = findAcceptedGameId(
          before,
          after,
          challenge.challenger,
          challenge.challenged
        );
      } else {
        const result = await contract.acceptChallenge(challenge.id);
        gameId = decodeSuccessValue<GameId>(result);
      }
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
    } catch (err: unknown) {
      const msg = err instanceof Error ? err.message : String(err);
      showToast('error', 'Failed to accept challenge', msg);
    }
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

    const unsubs = [
      subscribe('challenge', handleSSEChallenge),
      subscribe('accept_challenge', handleSSEAcceptChallenge),
      subscribe('reject_challenge', handleSSERejectChallenge)
    ];
    return () => {
      for (const u of unsubs) u();
    };
  });

  $effect(() => {
    if ($accountStore) {
      loadMyGames();
      loadPendingChallenges();
    }
  });

  function handleSSEChallenge(event: SSEEventData) {
    const data = event.event_data;
    if (data.challenged !== $accountStore) return;
    const exists = pendingChallenges.some(c => c.id === data.id);
    if (exists) return;
    updateWatermark(event.trigger_block_height);
    const newChallenge: Challenge = {
      id: data.id as string,
      challenger: data.challenger as string,
      challenged: data.challenged as string,
      wager_token: ((data.wager as unknown[])?.[0] as string | null) ?? null,
      wager_amount: ((data.wager as unknown[])?.[1] as string | null) ?? null,
      status: 'pending',
      game_id: null,
      created_at: new Date(
        Number(event.trigger_block_timestamp) / 1_000_000
      ).toISOString(),
      resolved_at: null
    };
    pendingChallenges = [newChallenge, ...pendingChallenges];
  }

  function handleSSEAcceptChallenge(event: SSEEventData) {
    const data = event.event_data;
    const challengeId = data.challenge_id as string;
    const challenge = pendingChallenges.find(c => c.id === challengeId);
    if (!challenge || challenge.status === 'accepted') return;
    updateWatermark(event.trigger_block_height);
    const gameId = data.game_id as GameId | null;
    const idx = pendingChallenges.findIndex(c => c.id === challengeId);
    if (idx !== -1) {
      pendingChallenges[idx] = {
        ...pendingChallenges[idx],
        status: 'accepted',
        game_id: gameId ? JSON.stringify(gameId) : null
      };
      pendingChallenges = pendingChallenges;
    }
    loadMyGames();
  }

  function handleSSERejectChallenge(event: SSEEventData) {
    const data = event.event_data;
    const challengeId = data.challenge_id as string;
    const challenge = pendingChallenges.find(c => c.id === challengeId);
    if (!challenge || challenge.status === 'rejected') return;
    updateWatermark(event.trigger_block_height);
    const idx = pendingChallenges.findIndex(c => c.id === challengeId);
    if (idx !== -1) {
      pendingChallenges[idx] = {
        ...pendingChallenges[idx],
        status: 'rejected'
      };
      pendingChallenges = pendingChallenges;
    }
  }

  function navigateToGame(gameId: GameId) {
    goto(gameUrl(gameId));
  }

  function createAiGame(difficulty: Difficulty) {
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

<svelte:head>
  <title>Protocol Pawns</title>
</svelte:head>

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
          <div class="dropdown right-0 top-full mt-1 min-w-36 space-y-0.5">
            <button
              class="btn-secondary w-full text-left text-sm flex items-center gap-2"
              onclick={() => {
                selectedDifficulty = 'Easy';
                showAiMenu = false;
                showAiConfirm = true;
              }}
              ><svg
                xmlns="http://www.w3.org/2000/svg"
                width="16"
                height="16"
                viewBox="0 0 24 24"
                fill="none"
                stroke="currentColor"
                stroke-width="2"
                stroke-linecap="round"
                stroke-linejoin="round"
                ><circle cx="12" cy="12" r="7" /><circle
                  cx="12"
                  cy="12"
                  r="3"
                  fill="currentColor"
                /></svg
              >Easy</button
            >
            <button
              class="btn-secondary w-full text-left text-sm flex items-center gap-2"
              onclick={() => {
                selectedDifficulty = 'Medium';
                showAiMenu = false;
                showAiConfirm = true;
              }}
              ><svg
                xmlns="http://www.w3.org/2000/svg"
                width="16"
                height="16"
                viewBox="0 0 24 24"
                fill="none"
                stroke="currentColor"
                stroke-width="2"
                stroke-linecap="round"
                stroke-linejoin="round"
                ><polygon
                  points="13 2 3 14 12 14 11 22 21 10 12 10 13 2"
                /></svg
              >Medium</button
            >
            <button
              class="btn-secondary w-full text-left text-sm flex items-center gap-2"
              onclick={() => {
                selectedDifficulty = 'Hard';
                showAiMenu = false;
                showAiConfirm = true;
              }}
              ><svg
                xmlns="http://www.w3.org/2000/svg"
                width="16"
                height="16"
                viewBox="0 0 24 24"
                fill="none"
                stroke="currentColor"
                stroke-width="2"
                stroke-linecap="round"
                stroke-linejoin="round"
                ><path d="M12 22s8-4 8-10V5l-8-3-8 3v7c0 6 8 10 8 10z" /></svg
              >Hard</button
            >
            <button
              class="btn-secondary w-full text-left text-sm flex items-center gap-2"
              onclick={() => {
                selectedDifficulty = 'VeryHard';
                showAiMenu = false;
                showAiConfirm = true;
              }}
              ><svg
                xmlns="http://www.w3.org/2000/svg"
                width="16"
                height="16"
                viewBox="0 0 24 24"
                fill="none"
                stroke="currentColor"
                stroke-width="2"
                stroke-linecap="round"
                stroke-linejoin="round"
                ><polygon
                  points="12 2 15.09 8.26 22 9.27 17 14.14 18.18 21.02 12 17.77 5.82 21.02 7 14.14 2 9.27 8.91 8.26 12 2"
                /></svg
              >Very Hard</button
            >
          </div>
        {/if}
      </div>
      <div class="relative">
        <button
          class="btn-primary text-sm"
          onclick={() => (showMatchmaking = true)}
          disabled={myGames.length >= MAX_OPEN_GAMES || $mmSearching}
          title={myGames.length >= MAX_OPEN_GAMES
            ? 'Max games reached'
            : $mmSearching
              ? 'Already searching'
              : ''}
        >
          {$mmSearching ? 'Searching...' : 'Find Match'}
        </button>
      </div>
    </div>
    <MatchmakingPanel
      open={showMatchmaking}
      onclose={() => (showMatchmaking = false)}
      gameCount={myGames.length}
    />
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
                <a class="block" href={gameUrl(game.game_id)}>
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
  message={`Start a ${selectedDifficulty.toLowerCase()} AI game? This will create an on-chain game. Each AI move consumes ${fmtTGas(AI_MOVE_GAS_BUDGET[selectedDifficulty])}.`}
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
        ? acceptWagerText
          ? ` This includes a wager of ${acceptWagerText}.`
          : ` This includes a wager.`
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
