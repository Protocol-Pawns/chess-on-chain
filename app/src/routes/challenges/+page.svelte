<script lang="ts">
  import { goto } from '$app/navigation';
  import { api, type Challenge } from '$lib/api/client';
  import { accountStore, isLoggedIn } from '$lib/near/account';
  import { contract } from '$lib/near/connector';
  import { showTxToast, showToast, decodeSuccessValue } from '$lib/toast';
  import { truncateAddr } from '$lib/format';
  import { gameUrl } from '$lib/game';
  import type { GameId } from '$lib/game';
  import WagerInput from '$lib/components/WagerInput.svelte';

  let challenges = $state<Challenge[]>([]);
  let loading = $state(true);
  let challengeTarget = $state('');
  let wagerEnabled = $state(false);
  let wagerToken = $state('');
  let wagerAmount = $state('');

  async function load() {
    if (!$accountStore) return;
    try {
      challenges = await api.challenges($accountStore);
    } catch (e) {
      console.error('Failed to load challenges:', e);
    } finally {
      loading = false;
    }
  }

  function navigateToGame(gameId: GameId) {
    goto(gameUrl(gameId));
  }

  function sendChallenge() {
    if (!$accountStore || !challengeTarget.trim()) return;
    const target = challengeTarget.trim();
    challengeTarget = '';
    if (wagerEnabled && wagerToken && wagerAmount) {
      showTxToast(contract.challengeWithWager(wagerToken, target, wagerAmount));
    } else {
      showTxToast(contract.challenge(target));
    }
    setTimeout(load, 4000);
  }

  function acceptChallenge(challenge: Challenge) {
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
        if (gameId) {
          showToast('success', 'Challenge accepted! Redirecting...');
          setTimeout(() => navigateToGame(gameId), 1000);
        } else {
          showToast('success', 'Challenge accepted!');
          load();
        }
      })
      .catch((err: unknown) => {
        const msg = err instanceof Error ? err.message : String(err);
        showToast('error', 'Failed to accept challenge', msg);
      });
  }

  function rejectChallenge(id: string) {
    showTxToast(contract.rejectChallenge(id));
    setTimeout(load, 4000);
  }

  $effect(() => {
    if ($accountStore) load();
  });
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
        <input
          type="text"
          bind:value={challengeTarget}
          placeholder="wallet.near"
          class="flex-1 bg-transparent border border-white/15 rounded px-2 py-1.5 text-sm focus:outline-none focus:border-primary"
        />
        <button
          class="btn-primary text-sm"
          onclick={sendChallenge}
          disabled={!challengeTarget.trim() ||
            (wagerEnabled && (!wagerAmount || !wagerToken))}
        >
          Challenge
        </button>
      </div>
      <WagerInput
        bind:enabled={wagerEnabled}
        bind:tokenId={wagerToken}
        bind:amount={wagerAmount}
      />
    </section>

    <section>
      <h2 class="text-base font-semibold mb-2">Your Challenges</h2>
      {#if challenges.length === 0}
        <p class="text-white/50 text-sm">No challenges yet</p>
      {:else}
        <div class="space-y-2">
          {#each challenges as challenge}
            <div class="card flex items-center justify-between">
              <div>
                <div class="font-medium text-sm">
                  {challenge.challenger === $accountStore ? '→' : '←'}
                  {challenge.challenger === $accountStore
                    ? truncateAddr(challenge.challenged)
                    : truncateAddr(challenge.challenger)}
                </div>
                <div class="text-xs text-white/50">
                  {challenge.status}
                  {#if challenge.wager_token && challenge.wager_amount}
                    <span class="text-yellow-400 ml-1">
                      Wager: {challenge.wager_amount}
                    </span>
                  {/if}
                </div>
              </div>
              <div class="flex gap-2">
                {#if challenge.status === 'pending' && challenge.challenged === $accountStore}
                  <button
                    class="btn-primary text-xs"
                    onclick={() => acceptChallenge(challenge)}>Accept</button
                  >
                  <button
                    class="btn-secondary text-xs"
                    onclick={() => rejectChallenge(challenge.id)}>Reject</button
                  >
                {/if}
                {#if challenge.status === 'accepted' && challenge.game_id}
                  <a
                    href="/game/{encodeURIComponent(challenge.game_id)}"
                    class="btn-primary text-xs">View Game</a
                  >
                {/if}
              </div>
            </div>
          {/each}
        </div>
      {/if}
    </section>
  </div>
{/if}
