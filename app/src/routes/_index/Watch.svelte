<script lang="ts">
  import type { AccountId } from "$abi";
  import { apiClient, type GameApi } from "$lib/api";
  import { GameCard } from "$lib/components";
  import ProgressSpinner from "$lib/components/ProgressSpinner.svelte";
  import { updateEloRatings } from "$lib/game";

  let recentGames: GameApi[];
  let updatedEloRatings = false;

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
    await updateEloRatings(Array.from(playerIds));
    updatedEloRatings = true;
  }
  fetchRecentGames();
</script>

{#if recentGames && updatedEloRatings}
  {#each recentGames as game}
    <GameCard {game} />
  {/each}
{:else}
  <ProgressSpinner inline />
{/if}
