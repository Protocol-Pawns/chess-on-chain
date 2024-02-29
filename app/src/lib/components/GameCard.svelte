<script lang="ts">
  import Button from "@smui/button";
  import Card from "@smui/card";

  import type { AccountId, GameId } from "$abi";
  import type { GameApi } from "$lib/api";
  import { gameId$ } from "$lib/game";

  export let game: GameApi;
  export let eloRatings: Record<AccountId, number>;

  function setGameId() {
    $gameId$ = game.game_id as GameId;
  }
</script>

<Card variant="outlined" padded class="mdc-card__gap">
  <div class="section-field">
    <h3>White</h3>
    <span>
      {#if game.white.type === "Human"}
        {game.white.value}
        {#if eloRatings[game.white.value] != null}
          [ELO: {eloRatings[game.white.value]}]
        {/if}
      {:else}
        AI ({game.white.value})
      {/if}
    </span>
  </div>
  <div class="section-field">
    <h3>Black</h3>
    <span>
      {#if game.black.type === "Human"}
        {game.black.value}
        {#if eloRatings[game.white.value] != null}
          [ELO: {eloRatings[game.white.value]}]
        {/if}
      {:else}
        AI ({game.black.value})
      {/if}
    </span>
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

<style lang="scss">
  :global(.mdc-card__gap) {
    gap: 0.4rem;
  }
</style>
