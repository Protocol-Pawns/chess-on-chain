<script lang="ts">
  import type { Writable } from "svelte/store";

  export let board: string[];
  export let moveFrom$: Writable<[number, number] | undefined>;
  export let moveTo$: Writable<[number, number] | undefined>;
</script>

<div class="board">
  {#each [...board].reverse() as row, rowIndex}
    <div class="board-row">
      <div class="legend">{8 - rowIndex}</div>
      {#each row.split("") as field, colIndex}
        <div
          class="board-field"
          style={`background: ${(rowIndex + colIndex) % 2 === 0 ? "#ddd" : "#555"}`}
        >
          <div
            class="overlay"
            class:from={$moveFrom$ &&
              $moveFrom$[0] === rowIndex &&
              $moveFrom$[1] === colIndex}
            class:to={$moveTo$ &&
              $moveTo$[0] === rowIndex &&
              $moveTo$[1] === colIndex}
          />
          {#if field === "p"}
            <img alt="black pawn" src="./pieces/bP.svg" />
          {:else if field === "P"}
            <img alt="white pawn" src="./pieces/wP.svg" />
          {:else if field === "n"}
            <img alt="black knight" src="./pieces/bN.svg" />
          {:else if field === "N"}
            <img alt="white knight" src="./pieces/wN.svg" />
          {:else if field === "b"}
            <img alt="black bishop" src="./pieces/bB.svg" />
          {:else if field === "B"}
            <img alt="white bishop" src="./pieces/wB.svg" />
          {:else if field === "r"}
            <img alt="black rook" src="./pieces/bR.svg" />
          {:else if field === "R"}
            <img alt="white rook" src="./pieces/wR.svg" />
          {:else if field === "q"}
            <img alt="black queen" src="./pieces/bQ.svg" />
          {:else if field === "Q"}
            <img alt="white queen" src="./pieces/wQ.svg" />
          {:else if field === "k"}
            <img alt="black king" src="./pieces/bK.svg" />
          {:else if field === "K"}
            <img alt="white king" src="./pieces/wK.svg" />
          {/if}
        </div>
      {/each}
    </div>
  {/each}
  <div class="board-row">
    <div class="legend"></div>
    <div class="legend">A</div>
    <div class="legend">B</div>
    <div class="legend">C</div>
    <div class="legend">D</div>
    <div class="legend">E</div>
    <div class="legend">F</div>
    <div class="legend">G</div>
    <div class="legend">H</div>
  </div>
</div>

<style lang="scss">
  :root {
    --field-size: 4rem;
  }

  .board {
    display: flex;
    flex-direction: column;
    width: 30rem;
    max-width: 100%;
    min-width: 16rem;
  }

  .board-row {
    display: flex;
    justify-content: center;
    width: 100%;
  }

  .board-field {
    flex: 1 1 auto;
    max-width: var(--field-size);
    aspect-ratio: 1 / 1;
    position: relative;

    .overlay {
      z-index: 1000;
      min-width: 100%;
      min-height: 100%;
      max-width: 100%;
      position: absolute;
      bottom: 0;
      left: 0;
      right: 0;

      &.from {
        background-color: var(--color-err-transparent);
      }
      &.to {
        background-color: var(--color-ok-transparent);
      }
    }

    img {
      min-width: 100%;
      min-height: 100%;
      max-width: 100%;
      position: absolute;
      bottom: 0;
      left: 0;
      right: 0;
    }
  }

  .legend {
    flex: 1 1 0;
    max-width: var(--field-size);
    aspect-ratio: 1 / 1;
    font-size: 1.6rem;
    font-weight: 600;
    display: flex;
    justify-content: center;
    align-items: center;
  }
</style>
