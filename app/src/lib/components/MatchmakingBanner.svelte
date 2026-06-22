<script lang="ts">
  import { page } from '$app/state';
  import {
    searching,
    searchMinElo,
    searchMaxElo,
    searchWager,
    cancelSearch
  } from '$lib/near/matchmaking';
  import { slide } from 'svelte/transition';

  let cancelling = $state(false);
  let onGamePage = $derived(page.url?.pathname?.startsWith('/game/') ?? false);

  async function handleCancel() {
    cancelling = true;
    await cancelSearch();
    cancelling = false;
  }
</script>

{#if $searching && !onGamePage}
  <div
    class="rounded-lg border border-primary-light/40 bg-primary-transparent2 px-3 py-2 flex items-center gap-3"
    transition:slide={{ duration: 200 }}
  >
    <span
      class="inline-block w-2.5 h-2.5 rounded-full bg-primary animate-pulse shrink-0"
    ></span>
    <span class="text-sm text-primary-light flex-1 min-0">
      Searching for opponent (elo {$searchMinElo}&ndash;{$searchMaxElo}
      {#if $searchWager > 0}
        , {$searchWager} NEAR wager
      {/if})
    </span>
    <button
      class="text-xs text-white/60 hover:text-white shrink-0 disabled:opacity-50"
      onclick={handleCancel}
      disabled={cancelling}
    >
      {cancelling ? '...' : 'Cancel'}
    </button>
  </div>
{/if}
