<script lang="ts">
  import { onMount, onDestroy } from 'svelte';
  import { contract } from '$lib/near/connector';
  import { accountStore, isRegistered } from '$lib/near/account';
  import { startSearch } from '$lib/near/matchmaking';
  import { searching } from '$lib/near/matchmaking';
  import { MAX_OPEN_GAMES } from '$lib/game';
  import type { MatchmakingEntry } from '$lib/near/contract-types';
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
  let queueEntries = $state<Array<[string, MatchmakingEntry]>>([]);
  let queueElos = $state<Map<string, number>>(new Map());
  let activeOpponents = $state<Set<string>>(new Set());
  let pollTimer: ReturnType<typeof setInterval> | null = null;
  let submitting = $state(false);

  let minElo = $derived(Math.max(0, myElo - rangePreset));
  let maxElo = $derived(myElo + rangePreset);
  let wagerRawAmount = $derived(
    selectedWager > 0 ? (BigInt(selectedWager) * 10n ** 24n).toString() : null
  );
  let compatibleCount = $derived(calcCompatible());

  onMount(() => {
    if ($accountStore) {
      contract
        .getAccount($accountStore)
        .then(a => {
          myElo = a.elo ?? 1_000;
        })
        .catch(() => {});
    }
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

  onDestroy(() => {
    if (pollTimer) clearInterval(pollTimer);
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

      if ($accountStore) {
        const gameIds = await contract.getGameIds($accountStore);
        const opponents = new Set<string>();
        for (const gid of gameIds) {
          const [white, black] = [gid[1], gid[2]];
          if (white && white !== $accountStore) opponents.add(white);
          if (black && black !== $accountStore) opponents.add(black);
        }
        activeOpponents = opponents;
      }
    } catch {
      /* ignore */
    }
  }

  function calcCompatible(): number {
    let count = 0;
    for (const [id, entry] of queueEntries) {
      if (activeOpponents.has(id)) continue;
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

  async function handleFindMatch() {
    if (!$accountStore || !$isRegistered) return;
    if (gameCount >= MAX_OPEN_GAMES) return;
    submitting = true;
    const ok = await startSearch(minElo, maxElo, selectedWager);
    submitting = false;
    if (ok) onclose();
  }
</script>

<Modal {open} {onclose}>
  <div class="card max-w-sm w-full bg-surface space-y-4">
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
      <p class="text-xs text-white/40">Accepting {minElo}&ndash;{maxElo}</p>
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
        {compatibleCount === 1 ? 'player' : 'players'} in queue &mdash; match likely!
      </div>
    {:else}
      <div class="text-sm text-white/40 bg-white/5 rounded px-3 py-2">
        No compatible players in queue right now &mdash; you'll be queued.
      </div>
    {/if}

    <div class="flex gap-2 justify-end">
      <button class="btn-secondary text-sm" onclick={onclose}>Cancel</button>
      <button
        class="btn-primary text-sm"
        onclick={handleFindMatch}
        disabled={submitting || gameCount >= MAX_OPEN_GAMES || $searching}
        title={gameCount >= MAX_OPEN_GAMES ? 'Max games reached' : ''}
      >
        {submitting ? '...' : 'Find Match'}
      </button>
    </div>
  </div>
</Modal>
