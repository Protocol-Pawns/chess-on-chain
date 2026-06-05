<script lang="ts">
  import { contract } from '$lib/near/connector';
  import { accountStore } from '$lib/near/account';
  import { showTxToast } from '$lib/toast';
  import { onMount } from 'svelte';

  let {
    playerWhite,
    playerBlack,
    disabled = false
  } = $props();

  let tokens = $state<string[]>([]);
  let selectedToken = $state('');
  let amount = $state('');
  let winner = $state('');
  let betInfo = $state<{
    is_locked: boolean;
    bets: Record<string, Array<[string, { amount: string; winner: string }]>>;
  } | null>(null);

  onMount(async () => {
    try {
      tokens = await contract.getTokenWhitelist();
      if (tokens.length > 0) selectedToken = tokens[0];
    } catch {}
    await loadBetInfo();
  });

  async function loadBetInfo() {
    if (!playerWhite || !playerBlack) return;
    try {
      const players = [playerWhite, playerBlack].sort() as [string, string];
      betInfo = await contract.getBetInfo(players);
    } catch {
      betInfo = null;
    }
  }

  function placeBet() {
    if (!$accountStore || !amount || !winner || !selectedToken) return;
    const players = [playerWhite, playerBlack].sort() as [string, string];
    showTxToast(contract.placeBet(selectedToken, players, winner, amount));
    setTimeout(loadBetInfo, 4000);
  }
</script>

<div class="card space-y-3">
  <h3 class="text-sm font-semibold">Bets</h3>

  {#if betInfo?.is_locked}
    <p class="text-xs text-white/50">Betting is locked (game in progress)</p>
  {:else if disabled}
    <p class="text-xs text-white/50">Place bets before the game starts</p>
  {:else}
    <div class="space-y-2">
      {#if betInfo?.bets}
        {@html ''}
        <div class="space-y-1">
          {#each Object.entries(betInfo.bets) as [tokenId, bets]}
            <div class="text-xs text-white/70">
              {tokenId}: {bets.length} bet{bets.length !== 1 ? 's' : ''}
            </div>
          {/each}
        </div>
      {/if}

      {#if $accountStore}
        <div class="space-y-2">
          <div class="flex gap-2">
            <select
              bind:value={selectedToken}
              class="bg-transparent border border-primary rounded px-2 py-1 text-xs focus:outline-none"
            >
              {#each tokens as token}
                <option value={token}>{token}</option>
              {/each}
            </select>
            <input
              type="text"
              bind:value={amount}
              placeholder="Amount"
              class="flex-1 bg-transparent border border-primary rounded px-2 py-1 text-xs focus:outline-none"
            />
          </div>
          <div class="flex gap-2">
            <button
              class="btn-secondary text-xs flex-1"
              class:opacity-50={winner !== playerWhite}
              onclick={() => (winner = playerWhite)}
            >
              White wins
            </button>
            <button
              class="btn-secondary text-xs flex-1"
              class:opacity-50={winner !== playerBlack}
              onclick={() => (winner = playerBlack)}
            >
              Black wins
            </button>
          </div>
          <button
            class="btn-primary text-xs w-full"
            onclick={placeBet}
            disabled={!amount || !winner || !selectedToken}
          >
            Place Bet
          </button>
        </div>
      {/if}
    </div>
  {/if}
</div>
