<script lang="ts">
  import type { AccountId } from "$abi";
  import { apiClient, type GameApi } from "$lib/api";
  import { GameCard } from "$lib/components";
  import ProgressSpinner from "$lib/components/ProgressSpinner.svelte";
  import { contract$ } from "$lib/near";

  let recentGames: GameApi[];
  let eloRatings: Record<AccountId, number>;

  async function fetchRecentGames() {
    const res = await apiClient.games.recent.finished.$get();
    recentGames = await res.json();
    const playerIds = new Set<AccountId>();
    for (const game of recentGames) {
      if (game.white.type === "Human") {
        playerIds.add(game.white.value);
      }
      if (game.black.type === "Human") {
        playerIds.add(game.black.value);
      }
    }
    const contract = await $contract$;
    eloRatings = Object.fromEntries(
      await contract.get_elo_ratings_by_ids({
        account_ids: Array.from(playerIds),
      }),
    );
  }
  fetchRecentGames();
</script>

{#if recentGames && eloRatings}
  {#each recentGames as game}
    <GameCard {game} {eloRatings} />
  {/each}
{:else}
  <ProgressSpinner inline />
{/if}
