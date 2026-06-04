<script lang="ts">
  import Button from "@smui/button";
  import Card from "@smui/card";

  import Player from "./Player.svelte";

  import type { GameId } from "$abi";
  import type { GameApi } from "$lib/api";
  import { gameId$ } from "$lib/game";

  export let game: GameApi;

  function setGameId() {
    $gameId$ = game.game_id as GameId;
  }
</script>

<Card variant="outlined" padded class="section-field-gap">
  <div class="section-field">
    <h3>White</h3>
    <Player player={game.white} />
  </div>
  <div class="section-field">
    <h3>Black</h3>
    <Player player={game.black} />
  </div>
  {#if game.outcome}
    <div class="section-field">
      {#if game.outcome.result === "Victory"}
        <h3>Winner</h3>
        <span>
          {game.outcome.color}
          {#if game.resigner}
            (resigned by: {game.resigner})
          {/if}
        </span>
      {:else}
        <h3>{game.outcome.result}</h3>
      {/if}
    </div>
  {/if}
  <div class="section-field">
    <Button variant="outlined" on:click={setGameId}>Open</Button>
  </div>
</Card>
