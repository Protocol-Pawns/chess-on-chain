<script lang="ts">
  import { onMount } from 'svelte';
  import { page } from '$app/state';
  import { goto } from '$app/navigation';
  import { api, type Challenge, type GameOverview } from '$lib/api/client';
  import { accountStore, isLoggedIn } from '$lib/near/account';
  import { contract } from '$lib/near/connector';
  import { showTxToast, showToast, decodeSuccessValue } from '$lib/toast';
  import { gameUrl, MAX_OPEN_GAMES } from '$lib/game';
  import type { GameId } from '$lib/game';
  import type { SSEEventData } from '$lib/sse';
  import { subscribe, updateWatermark } from '$lib/sse';
  import WagerInput from '$lib/components/WagerInput.svelte';
  import AccountSearch from '$lib/components/AccountSearch.svelte';
  import ChallengeCard from '$lib/components/ChallengeCard.svelte';
  import ConfirmModal from '$lib/components/ConfirmModal.svelte';
  import Pagination from '$lib/components/Pagination.svelte';
  import { getTokenPrice, estimateUsd } from '$lib/prices';
  import { formatWager, formatWagerText } from '$lib/wager';

  const MAX_OPEN_CHALLENGES = 25;
  const PER_PAGE = 25;

  let challenges = $state<Challenge[]>([]);
  let gameMap = $state<Map<string, GameOverview>>(new Map());
  let loading = $state(true);
  let currentPage = $state(1);
  let totalPages = $state(1);
  let hideRejected = $state(true);
  let challengeTarget = $state('');
  let wagerEnabled = $state(false);
  let wagerToken = $state('');
  let wagerTokenSymbol = $state('');
  let wagerAmount = $state('');
  let wagerRawAmount = $state('');
  let wagerInsufficientBalance = $state(false);
  let gameCount = $state(0);
  let ownChallengeCount = $state(0);
  let targetChallengeCount = $state<number | null>(null);
  let targetRegistered = $state<boolean | null>(null);
  let targetExistingChallenge = $state<boolean | null>(null);
  let checkingTarget = $state(false);
  let acceptTarget = $state<Challenge | null>(null);
  let rejectTarget = $state<Challenge | null>(null);
  let cancelTarget = $state<Challenge | null>(null);
  let showSendConfirm = $state(false);
  let wagerUsd = $state<string | undefined>(undefined);

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

  $effect(() => {
    if (wagerEnabled && wagerToken && wagerAmount) {
      getTokenPrice(wagerToken, wagerTokenSymbol).then(p => {
        wagerUsd = estimateUsd(wagerAmount, p);
      });
    } else {
      wagerUsd = undefined;
    }
  });

  async function load() {
    if (!$accountStore) return;
    try {
      const [result, gameIds, sent, received] = await Promise.all([
        api.challenges($accountStore, currentPage, PER_PAGE, hideRejected),
        contract.getGameIds($accountStore),
        contract.getChallenges($accountStore, true).catch(() => []),
        contract.getChallenges($accountStore, false).catch(() => [])
      ]);
      const items = 'items' in result ? result.items : result;
      challenges = items;
      if ('total_pages' in result && result.total_pages) {
        totalPages = result.total_pages;
      }
      gameCount = gameIds.length;
      ownChallengeCount = sent.length + received.length;

      const acceptedIds = items
        .filter(c => c.status === 'accepted' && c.game_id)
        .map(c => JSON.parse(c.game_id!) as [number, string, string | null]);
      if (acceptedIds.length > 0) {
        const games = await api.query(acceptedIds);
        const map = new Map<string, GameOverview>();
        games.forEach(g => {
          map.set(JSON.stringify(g.game_id), g);
        });
        gameMap = map;
      }
    } catch (e) {
      console.error('Failed to load challenges:', e);
    } finally {
      loading = false;
    }
  }

  let checkTimeout: ReturnType<typeof setTimeout> | null = null;

  async function checkTargetLimit(target: string) {
    if (checkTimeout) clearTimeout(checkTimeout);
    if (!target.trim()) {
      targetChallengeCount = null;
      targetRegistered = null;
      targetExistingChallenge = null;
      checkingTarget = false;
      return;
    }
    const t = target.trim();
    const me = $accountStore?.toLowerCase() ?? '';
    const tLower = t.toLowerCase();
    checkingTarget = true;
    checkTimeout = setTimeout(async () => {
      try {
        const [sent, received, reg] = await Promise.all([
          contract.getChallenges(t, true).catch(() => []),
          contract.getChallenges(t, false).catch(() => []),
          contract.storageBalanceOf(t)
        ]);
        targetChallengeCount = sent.length + received.length;
        targetRegistered = reg !== null;
        const myId = `${me}-vs-${tLower}`;
        const reverseId = `${tLower}-vs-${me}`;
        targetExistingChallenge = [...sent, ...received].some(
          id => id === myId || id === reverseId
        );
      } catch {
        targetChallengeCount = null;
        targetRegistered = null;
        targetExistingChallenge = null;
      } finally {
        checkingTarget = false;
      }
    }, 600);
  }

  function navigateToGame(gameId: GameId) {
    goto(gameUrl(gameId));
  }

  function sendChallenge() {
    if (!$accountStore || !challengeTarget.trim() || sendDisabled) return;
    showSendConfirm = true;
  }

  function doSend() {
    showSendConfirm = false;
    if (!$accountStore || !challengeTarget.trim()) return;
    const target = challengeTarget.trim();
    const needsRegistration = targetRegistered === false;
    const savedWagerToken = wagerEnabled ? wagerToken : null;
    const savedWagerAmount =
      wagerEnabled && wagerRawAmount ? wagerRawAmount : null;
    showToast('info', 'Sending challenge...');
    let promise;
    if (needsRegistration) {
      promise = contract.challengeWithRegistration(target);
    } else if (wagerEnabled && wagerToken && wagerRawAmount) {
      promise = contract.challengeWithWager(wagerToken, target, wagerRawAmount);
    } else {
      promise = contract.challenge(target);
    }
    promise
      .then(() => {
        challengeTarget = '';
        targetChallengeCount = null;
        targetRegistered = null;
        targetExistingChallenge = null;
        wagerEnabled = false;
        wagerToken = '';
        wagerTokenSymbol = '';
        wagerAmount = '';
        wagerRawAmount = '';
        const optimistic: Challenge = {
          id: `${$accountStore}-vs-${target}`,
          challenger: $accountStore,
          challenged: target,
          wager_token: savedWagerToken,
          wager_amount: savedWagerAmount,
          status: 'pending',
          game_id: null,
          created_at: new Date().toISOString(),
          resolved_at: null
        };
        challenges = [optimistic, ...challenges];
        ownChallengeCount += 1;
        showToast('success', 'Challenge sent!');
      })
      .catch((err: unknown) => {
        const msg = err instanceof Error ? err.message : String(err);
        showToast('error', 'Failed to send challenge', msg);
      });
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
        const idx = challenges.findIndex(c => c.id === challenge.id);
        if (idx !== -1) {
          challenges[idx] = {
            ...challenges[idx],
            status: 'accepted',
            game_id: gameId ? JSON.stringify(gameId) : null
          };
          challenges = challenges;
          ownChallengeCount = Math.max(0, ownChallengeCount - 1);
        }
        if (gameId) {
          showToast('success', 'Challenge accepted! Redirecting...');
          setTimeout(() => navigateToGame(gameId), 1000);
        } else {
          showToast('success', 'Challenge accepted!');
        }
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
      const idx = challenges.findIndex(c => c.id === challenge.id);
      if (idx !== -1) {
        challenges[idx] = { ...challenges[idx], status: 'rejected' };
        challenges = challenges;
        ownChallengeCount = Math.max(0, ownChallengeCount - 1);
      }
    }).catch(() => {});
  }

  function doCancel(challenge: Challenge) {
    cancelTarget = null;
    const p = contract.rejectChallenge(challenge.id, true);
    showTxToast(p);
    p.then(() => {
      const idx = challenges.findIndex(c => c.id === challenge.id);
      if (idx !== -1) {
        challenges[idx] = { ...challenges[idx], status: 'rejected' };
        challenges = challenges;
        ownChallengeCount = Math.max(0, ownChallengeCount - 1);
      }
    }).catch(() => {});
  }

  let selfChallenge = $derived(
    challengeTarget.trim().toLowerCase() === $accountStore?.toLowerCase()
  );

  let sendDisabled = $derived(
    !challengeTarget.trim() ||
      selfChallenge ||
      (wagerEnabled && (!wagerRawAmount || !wagerToken)) ||
      (wagerEnabled && wagerInsufficientBalance) ||
      gameCount >= MAX_OPEN_GAMES ||
      ownChallengeCount >= MAX_OPEN_CHALLENGES ||
      targetExistingChallenge === true ||
      (targetChallengeCount !== null &&
        targetChallengeCount >= MAX_OPEN_CHALLENGES)
  );

  let sendDisabledReason = $derived.by(() => {
    if (selfChallenge) return 'Cannot challenge yourself';
    if (gameCount >= MAX_OPEN_GAMES) return 'Max games reached';
    if (ownChallengeCount >= MAX_OPEN_CHALLENGES)
      return 'Max challenges reached';
    if (targetExistingChallenge)
      return 'You already have a pending challenge with this player';
    if (
      targetChallengeCount !== null &&
      targetChallengeCount >= MAX_OPEN_CHALLENGES
    )
      return 'Target has max challenges';
    return '';
  });

  $effect(() => {
    challengeTarget;
    checkTargetLimit(challengeTarget);
  });

  $effect(() => {
    if ($accountStore) load();
  });

  $effect(() => {
    hideRejected;
    if ($accountStore && !loading) {
      currentPage = 1;
      load();
    }
  });

  function handleSSEChallenge(event: SSEEventData) {
    const data = event.event_data;
    if (data.challenged !== $accountStore) return;
    const exists = challenges.some(c => c.id === data.id);
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
    challenges = [newChallenge, ...challenges];
    ownChallengeCount += 1;
  }

  function handleSSEAcceptChallenge(event: SSEEventData) {
    const data = event.event_data;
    const challengeId = data.challenge_id as string;
    const challenge = challenges.find(c => c.id === challengeId);
    if (!challenge || challenge.status === 'accepted') return;
    updateWatermark(event.trigger_block_height);
    const gameId = data.game_id as GameId | null;
    const idx = challenges.findIndex(c => c.id === challengeId);
    if (idx !== -1) {
      challenges[idx] = {
        ...challenges[idx],
        status: 'accepted',
        game_id: gameId ? JSON.stringify(gameId) : null
      };
      challenges = challenges;
    }
    if (gameId) {
      showToast('success', 'Challenge accepted! Redirecting...');
      setTimeout(() => navigateToGame(gameId), 1000);
    }
  }

  function handleSSERejectChallenge(event: SSEEventData) {
    const data = event.event_data;
    const challengeId = data.challenge_id as string;
    const challenge = challenges.find(c => c.id === challengeId);
    if (!challenge || challenge.status === 'rejected') return;
    updateWatermark(event.trigger_block_height);
    const idx = challenges.findIndex(c => c.id === challengeId);
    if (idx !== -1) {
      challenges[idx] = { ...challenges[idx], status: 'rejected' };
      challenges = challenges;
    }
    ownChallengeCount = Math.max(0, ownChallengeCount - 1);
  }

  onMount(() => {
    const target = page.url.searchParams.get('target');
    if (target) {
      challengeTarget = target;
      history.replaceState(null, '', page.url.pathname);
    }
    const unsubs = [
      subscribe('challenge', handleSSEChallenge),
      subscribe('accept_challenge', handleSSEAcceptChallenge),
      subscribe('reject_challenge', handleSSERejectChallenge)
    ];
    return () => {
      for (const u of unsubs) u();
    };
  });

  function goToPage(p: number) {
    currentPage = p;
    loading = true;
    load();
  }
</script>

{#if !$isLoggedIn}
  <div class="text-center py-12 text-white/50">
    Connect your wallet to view challenges
  </div>
{:else if loading}
  <div class="space-y-6 animate-pulse">
    <div class="card">
      <div class="h-4 w-32 rounded bg-white/10 mb-2"></div>
      <div class="flex gap-2">
        <div class="flex-1 h-8 rounded bg-white/5"></div>
        <div class="h-8 w-20 rounded bg-white/5"></div>
      </div>
    </div>
    <div class="space-y-2">
      {#each Array(2) as _}
        <div class="card">
          <div class="h-4 w-2/3 rounded bg-white/10 mb-1"></div>
          <div class="h-3 w-1/3 rounded bg-white/5"></div>
        </div>
      {/each}
    </div>
  </div>
{:else}
  <div class="space-y-6">
    <h2 class="text-xl font-bold text-primary text-center">Challenges</h2>
    <section class="card space-y-3">
      <h2 class="text-base font-semibold">Challenge a Player</h2>
      <div class="flex gap-2">
        <div class="flex-1">
          <AccountSearch bind:value={challengeTarget} onenter={sendChallenge} />
        </div>
        <button
          class="btn-primary text-sm"
          onclick={sendChallenge}
          disabled={sendDisabled}
          title={sendDisabledReason}
        >
          Challenge
        </button>
      </div>
      {#if ownChallengeCount >= MAX_OPEN_CHALLENGES}
        <p class="text-xs text-red-400">
          You have reached the max open challenges ({ownChallengeCount}/{MAX_OPEN_CHALLENGES})
        </p>
      {/if}
      {#if targetChallengeCount !== null && targetChallengeCount >= MAX_OPEN_CHALLENGES}
        <p class="text-xs text-red-400">
          This player has reached the max open challenges
        </p>
      {/if}
      <WagerInput
        bind:enabled={wagerEnabled}
        bind:tokenId={wagerToken}
        bind:tokenSymbol={wagerTokenSymbol}
        bind:amount={wagerAmount}
        bind:rawAmount={wagerRawAmount}
        bind:insufficientBalance={wagerInsufficientBalance}
      />
    </section>

    <section>
      <div class="flex items-center justify-between mb-2">
        <h2 class="text-base font-semibold">Your Challenges</h2>
        <label
          class="flex items-center gap-1.5 text-xs text-white/50 cursor-pointer select-none"
        >
          <input type="checkbox" bind:checked={hideRejected} />
          Hide rejected
        </label>
      </div>
      {#if gameCount >= MAX_OPEN_GAMES}
        <p class="text-xs text-red-400 mb-2">
          Max games reached ({gameCount}/{MAX_OPEN_GAMES})
        </p>
      {/if}
      {#if challenges.length === 0}
        <p class="text-white/50 text-sm">No challenges yet</p>
      {:else}
        <div class="space-y-2">
          {#each challenges as challenge}
            <ChallengeCard
              {challenge}
              currentAccount={$accountStore!}
              {gameCount}
              game={challenge.game_id
                ? (gameMap.get(challenge.game_id) ?? null)
                : null}
              onaccept={c => (acceptTarget = c)}
              onreject={c => (rejectTarget = c)}
              oncancel={c => (cancelTarget = c)}
            />
          {/each}
        </div>
        <Pagination page={currentPage} {totalPages} onchange={goToPage} />
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

<ConfirmModal
  open={showSendConfirm}
  title="Send Challenge?"
  message={`Challenge ${challengeTarget.trim()} to a game?` +
    (checkingTarget ? ' Checking player info...' : '') +
    (targetRegistered === false
      ? ' This player is not yet registered. An additional 0.05 N will be charged to register them.'
      : '') +
    (wagerEnabled && wagerToken && wagerAmount
      ? ` This includes a wager of ${wagerAmount} ${wagerTokenSymbol}${wagerUsd ? ` (${wagerUsd})` : ''}.`
      : '')}
  confirmLabel="Send"
  confirmClass="btn-primary text-sm"
  confirmDisabled={checkingTarget || targetRegistered === null}
  onconfirm={doSend}
  onclose={() => (showSendConfirm = false)}
/>
