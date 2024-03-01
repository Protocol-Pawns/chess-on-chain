<script lang="ts">
  import { mdiSkipNext, mdiSkipPrevious, mdiStepBackward } from "@mdi/js";
  import Button, { Icon } from "@smui/button";
  import Accordion, { Panel, Content, Header } from "@smui-extra/accordion";
  import { writable } from "svelte/store";

  import ChessBoard from "./ChessBoard.svelte";
  import Player from "./Player.svelte";
  import ProgressSpinner from "./ProgressSpinner.svelte";

  import type { GameId } from "$abi";
  import { apiClient, type GameApi } from "$lib/api";
  import { defaultBoard, gameId$, updateEloRatings } from "$lib/game";

  export let gameId: GameId;
  export let watchMode = false;

  let game: GameApi | undefined;
  let currentBoard: string[] | undefined;
  let currentMove: GameApi["moves"][0] | undefined;
  let currentIndex: number | undefined;
  let moveFrom$ = writable<[number, number] | undefined>();
  let moveTo$ = writable<[number, number] | undefined>();
  let updatedEloRatings = false;

  apiClient.games.game[":game_id"]
    .$get({
      param: {
        game_id: encodeURI(JSON.stringify(gameId)),
      },
    })
    .then((res) => res.json())
    .then(async (gameRes) => {
      game = gameRes;
      if (watchMode) {
        currentIndex = -1;
      }
      const playerIds = [];
      if (game.white.type === "Human") {
        playerIds.push(game.white.value);
      }
      if (game.black.type === "Human") {
        playerIds.push(game.black.value);
      }
      await updateEloRatings(playerIds);
      updatedEloRatings = true;
    });

  $: if (currentIndex != null && game != null) {
    if (currentIndex === -1) {
      currentBoard = defaultBoard;
      currentMove = undefined;
      $moveFrom$ = undefined;
      $moveTo$ = undefined;
    } else {
      currentBoard = game.moves[currentIndex].board;
      currentMove = game.moves[currentIndex];
      const split = currentMove.mv.split(" to ");
      if (split.length === 2) {
        $moveFrom$ = [
          8 - Number(split[0].charAt(1) ?? 0),
          (split[0].codePointAt(0) ?? 0) - 97,
        ];
        $moveTo$ = [
          8 - Number(split[1].charAt(1) ?? 0),
          (split[1].codePointAt(0) ?? 0) - 97,
        ];
      }
    }
  }
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

  {#if game && currentBoard && currentIndex != null && updatedEloRatings}
    <div class="content">
      <Accordion>
        <Panel>
          <Header>
            <div class="header">
              <Player player={game.white}>
                <img class="icon" alt="white" src="./pieces/wP.svg" />
              </Player>
              <span>VS</span>
              <Player player={game.black}>
                <img class="icon" alt="black" src="./pieces/bP.svg" />
              </Player>
            </div>
          </Header>
          <Content class="section-field-gap">
            <div class="section-field">
              <h3>Game ID</h3>
              <span>{game.game_id}</span>
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
          </Content>
        </Panel>
      </Accordion>

      <div class="navigation">
        <Button
          variant="outlined"
          style={`visibility: ${currentIndex === -1 ? "hidden" : "visible"};`}
          on:click={() => {
            if (currentIndex == null) return;
            currentIndex--;
          }}
        >
          <Icon tag="svg" viewBox="0 0 24 24">
            <path fill="currentColor" d={mdiSkipPrevious} />
          </Icon>
          Prev
        </Button>
        <Button
          variant="outlined"
          style={`visibility: ${currentIndex + 1 === game.moves.length ? "hidden" : "visible"};`}
          on:click={() => {
            if (currentIndex == null) return;
            currentIndex++;
          }}
        >
          Next
          <Icon tag="svg" viewBox="0 0 24 24">
            <path fill="currentColor" d={mdiSkipNext} />
          </Icon>
        </Button>
      </div>
      <ChessBoard board={currentBoard} {moveFrom$} {moveTo$} />
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

  .header {
    width: 100%;
    display: flex;
    flex-direction: column;
    align-items: center;
    gap: 0.2rem;
  }

  .content {
    display: flex;
    flex-direction: column;
    align-items: stretch;
    gap: 1rem;

    .navigation {
      display: flex;
      justify-content: space-around;
      gap: 0.8rem;
    }
  }

  img.icon {
    height: 1.7rem;
    padding-bottom: 0.4rem;
  }
</style>
