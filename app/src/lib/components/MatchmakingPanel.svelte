<script lang="ts">
  import { onMount } from 'svelte';
  import { goto } from '$app/navigation';
  import { contract } from '$lib/near/connector';
  import { accountStore, isRegistered } from '$lib/near/account';
  import { showToast, decodeSuccessValue } from '$lib/toast';
  import { gameUrl, MAX_OPEN_GAMES } from '$lib/game';
  import type { GameId } from '$lib/game';
  import type { MatchmakingEntry } from '$lib/near/contract-types';
  import type { SSEEventData } from '$lib/sse';
  import { subscribe } from '$lib/sse';
  import { WRAP_NEAR_ID } from '$lib/tokens';
  import Modal from '$lib/components/Modal.svelte';
  import TokenIcon from '$lib/components/TokenIcon.svelte';

  interface Props {
    open: boolean;
    gameCount: number;
    onclose: () => void;
  }

  let { open, gameCount, onclose }: Props = $props();

  const RANGE_PRESETS = [
    { label: 'Tight (±50)', value: 50 },
    { label: 'Balanced (±100)', value: 100 },
    { label: 'Wide (±200)', value: 200 },
    { label: 'Any (±400)', value: 400 }
  ] as const;

  const WAGER_PRESETS = [0, 1, 5, 10, 15, 25, 50, 100] as const;

  let myElo = $state(1_000);
  let rangePreset = $state(100);
  let selectedWager = $state<number>(0);
  let searching = $state(false);
  let queueEntries = $state<Array<[string, MatchmakingEntry]>>([]);
  let queueElos = $state<Map<string, number>>(new Map());

  let minElo = $derived(Math.max(0, myElo - rangePreset));
  let maxElo = $derived(myElo + rangePreset);
  let wagerRawAmount = $derived(
    selectedWager > 0 ? (BigInt(selectedWager) * 10n ** 24n).toString() : null
  );

  let compatibleCount = $derived(calcCompatible());

  let unsubSSE: (() => void) | null = null;
  let pollTimer: ReturnType<typeof setInterval> | null = null;

  onMount(() => {
    if ($accountStore) {
      contract
        .getAccount($accountStore)
        .then(a => {
          myElo = a.elo ?? 1_000;
        })
        .catch(() => {});
      contract
        .isQueued($accountStore)
        .then(entry => {
          if (entry) searching = true;
        })
        .catch(() => {});
    }

    return () => {
      if (unsubSSE) unsubSSE();
      if (pollTimer) clearInterval(pollTimer);
    };
  });

  $effect(() => {
    if (open) {
      refreshQueue();
      pollTimer = setInterval(refreshQueue, 5_000);
    } else {
      if (pollTimer) {
        clearInterval(pollTimer);
        pollTimer = null;
      }
    }

    return () => {
      if (pollTimer) {
        clearInterval(pollTimer);
        pollTimer = null;
      }
    };
  });

  $effect(() => {
    if (searching) {
      unsubSSE = subscribe('create_game', handleCreateGame);
      return () => {
        if (unsubSSE) unsubSSE();
        unsubSSE = null;
      };
    } else {
      if (unsubSSE) unsubSSE();
      unsubSSE = null;
    }
  });

  async function refreshQueue() {
    try {
      const entries = await contract.getMatchmakingQueue(0, 100);
      queueEntries = entries;

      const eloMap = new Map<string, number>();
      await Promise.allSettled(
        entries.map(async ([id, _entry]) => {
          const acct = await contract.getAccount(id);
          eloMap.set(id, acct.elo ?? 1_000);
        })
      );
      queueElos = eloMap;
    } catch {
      /* ignore */
    }
  }

  function calcCompatible(): number {
    let count = 0;
    for (const [id, entry] of queueEntries) {
      const theirElo = queueElos.get(id);
      if (theirElo === undefined) continue;
      const eloOk =
        myElo >= entry.min_elo &&
        myElo <= entry.max_elo &&
        theirElo >= minElo &&
        theirElo <= maxElo;
      if (!eloOk) continue;

      const theirWager = entry.wager;
      const myWager =
        selectedWager > 0 ? [WRAP_NEAR_ID, wagerRawAmount!] : null;
      const wagerOk =
        (theirWager === null || theirWager === undefined) && myWager === null
          ? true
          : theirWager &&
              myWager &&
              theirWager[0] === myWager[0] &&
              theirWager[1] === myWager[1]
            ? true
            : false;
      if (wagerOk) count++;
    }
    return count;
  }

  function handleCreateGame(event: SSEEventData) {
    if (!searching) return;
    const data = event.event_data;
    const me = $accountStore;
    if (!me) return;
    const white = data.white as unknown[];
    const black = data.black as unknown[];
    const involved =
      extractAccount(white) === me || extractAccount(black) === me;
    if (!involved) return;
    searching = false;
    onclose();
    const gameId = data.game_id as GameId;
    if (gameId) {
      showToast('success', 'Match found! Redirecting...');
      setTimeout(() => goto(gameUrl(gameId)), 1000);
    }
  }

  function extractAccount(player: unknown[]): string | null {
    if (player[0] === 'Human' || player[0] === 0 || player[0] === '0') {
      return (player[1] as string) ?? null;
    }
    return null;
  }

  async function findMatch() {
    if (!$accountStore || !$isRegistered) return;
    if (gameCount >= MAX_OPEN_GAMES) return;
    searching = true;
    try {
      if (wagerRawAmount) {
        await contract.joinMatchmakingWithWager(
          WRAP_NEAR_ID,
          minElo,
          maxElo,
          wagerRawAmount
        );
      } else {
        const result = await contract.joinMatchmaking(minElo, maxElo);
        const gameId = decodeSuccessValue<GameId | null>(result);
        if (gameId) {
          searching = false;
          onclose();
          showToast('success', 'Match found! Redirecting...');
          setTimeout(() => goto(gameUrl(gameId)), 1000);
        }
      }
      showToast('info', 'Searching for opponent...');
    } catch (err: unknown) {
      searching = false;
      const msg = err instanceof Error ? err.message : String(err);
      showToast('error', 'Failed to find match', msg);
    }
  }

  async function cancelSearch() {
    searching = false;
    try {
      await contract.cancelMatchmaking();
      showToast('info', 'Matchmaking cancelled');
    } catch (err: unknown) {
      const msg = err instanceof Error ? err.message : String(err);
      showToast('error', 'Failed to cancel matchmaking', msg);
    }
  }
</script>

<Modal {open} {onclose}>
  <div class="card max-w-sm w-full bg-surface space-y-4">
    {#if searching}
      <div class="space-y-3">
        <h3 class="text-base font-semibold">Finding Match…</h3>
        <div class="flex items-center gap-3">
          <span
            class="inline-block w-3 h-3 rounded-full bg-primary animate-pulse"
          ></span>
          <span class="text-sm text-white/70"
            >Searching (elo {minElo}–{maxElo})…</span
          >
        </div>
        <div class="text-sm text-white/50">
          {#if selectedWager > 0}
            Wager: {selectedWager} NEAR
          {:else}
            No wager
          {/if}
        </div>
        <button class="btn-secondary text-sm w-full" onclick={cancelSearch}>
          Cancel Search
        </button>
      </div>
    {:else}
      <h3 class="text-base font-semibold">Find a Match</h3>

      <div class="space-y-1">
        <div class="flex items-center gap-2 text-sm">
          <span class="text-white/50">Your ELO:</span>
          <span class="font-semibold">{myElo}</span>
        </div>
        <span class="text-xs text-white/50">Opponent elo range</span>
        <select
          class="w-full bg-white/5 border border-white/10 rounded px-2 py-1.5 text-sm"
          bind:value={rangePreset}
        >
          {#each RANGE_PRESETS as preset}
            <option value={preset.value}>{preset.label}</option>
          {/each}
        </select>
        <p class="text-xs text-white/40">Accepting {minElo}–{maxElo}</p>
      </div>

      <div class="space-y-1">
        <span class="text-xs text-white/50">Wager (wNEAR)</span>
        <div class="grid grid-cols-4 gap-1.5">
          {#each WAGER_PRESETS as amt}
            <button
              class="rounded px-2 py-1.5 text-sm font-medium transition-colors {selectedWager ===
              amt
                ? 'bg-primary-transparent2 border border-primary-light text-primary-light'
                : 'bg-white/5 border border-white/10 hover:bg-white/10'}"
              onclick={() => (selectedWager = amt)}
            >
              {#if amt === 0}
                None
              {:else}
                <span class="flex items-center justify-center gap-1">
                  {amt}
                  <TokenIcon tokenId={WRAP_NEAR_ID} size={12} />
                </span>
              {/if}
            </button>
          {/each}
        </div>
      </div>

      {#if compatibleCount > 0}
        <div
          class="flex items-center gap-2 text-sm text-primary-green bg-primary-greenTransparent/20 rounded px-3 py-2"
        >
          <svg viewBox="0 0 24 24" class="w-4 h-4 shrink-0" fill="currentColor"
            ><path d="M9 16.17L4.83 12l-1.42 1.41L9 19 21 7l-1.41-1.41z" /></svg
          >
          {compatibleCount} compatible
          {compatibleCount === 1 ? 'player' : 'players'} in queue — match likely!
        </div>
      {:else}
        <div class="text-sm text-white/40 bg-white/5 rounded px-3 py-2">
          No compatible players in queue right now — you'll be queued.
        </div>
      {/if}

      <div class="flex gap-2 justify-end">
        <button class="btn-secondary text-sm" onclick={onclose}>Cancel</button>
        <button
          class="btn-primary text-sm"
          onclick={findMatch}
          disabled={gameCount >= MAX_OPEN_GAMES}
          title={gameCount >= MAX_OPEN_GAMES ? 'Max games reached' : ''}
        >
          Find Match
        </button>
      </div>
    {/if}
  </div>
</Modal>
