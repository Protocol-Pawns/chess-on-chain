<script lang="ts">
  import type { Player } from "$abi";
  import { eloRatings$ } from "$lib/game";

  export let player: Player;
</script>

<span>
  <slot />
  {#if player.type === "Human"}
    {player.value}
    {#if $eloRatings$[player.value] != null}
      [ELO: {$eloRatings$[player.value]}]
    {/if}
  {:else}
    AI ({player.value})
  {/if}
</span>

<style lang="scss">
  span {
    display: inline-flex;
    align-items: stretch;
  }
</style>
