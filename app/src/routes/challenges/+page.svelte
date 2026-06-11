<script lang="ts">
  import { goto } from '$app/navigation';
  import { api, type Challenge, type GameOverview } from '$lib/api/client';
  import { accountStore, isLoggedIn } from '$lib/near/account';
  import { contract } from '$lib/near/connector';
  import { showTxToast, showToast, decodeSuccessValue } from '$lib/toast';
  import { gameUrl, MAX_OPEN_GAMES } from '$lib/game';
  import type { GameId } from '$lib/game';
  import WagerInput from '$lib/components/WagerInput.svelte';
  import AccountSearch from '$lib/components/AccountSearch.svelte';
  import ChallengeCard from '$lib/components/ChallengeCard.svelte';
  import ConfirmModal from '$lib/components/ConfirmModal.svelte';
  import Pagination from '$lib/components/Pagination.svelte';
  import { getTokenPrice, estimateUsd } from '$lib/prices';

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
  let checkingTarget = $state(false);
  let acceptTarget = $state<Challenge | null>(null);
  let rejectTarget = $state<Challenge | null>(null);
  let cancelTarget = $state<Challenge | null>(null);
  let showSendConfirm = $state(false);
  let wagerUsd = $state<string | undefined>(undefined);

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
      checkingTarget = false;
      return;
    }
    const t = target.trim();
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
      } catch {
        targetChallengeCount = null;
        targetRegistered = null;
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
    challengeTarget = '';
    targetChallengeCount = null;
    targetRegistered = null;
    let promise;
    if (needsRegistration) {
      promise = contract.challengeWithRegistration(target);
    } else if (wagerEnabled && wagerToken && wagerRawAmount) {
      promise = contract.challengeWithWager(wagerToken, target, wagerRawAmount);
    } else {
      promise = contract.challenge(target);
    }
    showTxToast(promise);
    promise
      .then(() => {
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
        setTimeout(load, 10000);
      })
      .catch(() => {});
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
        setTimeout(load, 10000);
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
      setTimeout(load, 10000);
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
      setTimeout(load, 10000);
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
      (targetChallengeCount !== null &&
        targetChallengeCount >= MAX_OPEN_CHALLENGES)
  );

  let sendDisabledReason = $derived.by(() => {
    if (selfChallenge) return 'Cannot challenge yourself';
    if (gameCount >= MAX_OPEN_GAMES) return 'Max games reached';
    if (ownChallengeCount >= MAX_OPEN_CHALLENGES)
      return 'Max challenges reached';
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
