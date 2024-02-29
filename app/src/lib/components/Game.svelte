<script lang="ts">
  import { mdiStepBackward } from "@mdi/js";
  import Button, { Icon } from "@smui/button";

  import ProgressSpinner from "./ProgressSpinner.svelte";

  import type { GameId } from "$abi";
  import { apiClient, type GameApi } from "$lib/api";
  import { gameId$ } from "$lib/game";

  export let gameId: GameId;
  let game: GameApi | undefined;
  apiClient.games.game[":game_id"]
    .$get({
      param: {
        game_id: encodeURI(JSON.stringify(gameId)),
      },
    })
    .then((res) => res.json())
    .then((gameRes) => {
      game = gameRes;
    });
</script>

<div class="game">
  <Button
    style="align-self: flex-start;"
    on:click={() => {
      $gameId$ = undefined;
    }}
  >
    <Icon tag="svg" viewBox="0 0 24 24">
      <path fill="currentColor" d={mdiStepBackward} />
    </Icon>
    Back
  </Button>

  {#if game}
    <div class="board">
      {JSON.stringify(game.moves)}
    </div>
  {:else}
    <ProgressSpinner inline />
  {/if}
</div>

<style lang="scss">
  .game {
    display: flex;
    flex-direction: column;
    gap: 1rem;
  }

  .board {
  }
</style>
