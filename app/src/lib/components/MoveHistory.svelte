<script lang="ts">
  import type { GameMove } from '$lib/api/client';

  let { moves }: { moves: GameMove[] } = $props();
</script>

<div class="card max-h-64 overflow-y-auto">
  <h3 class="text-base font-semibold mb-2">Moves</h3>
  <div class="grid grid-cols-2 gap-x-4 gap-y-0.5 text-sm">
    {#each moves as move}
      <div class="flex gap-2 {move.color === 'White' ? '' : 'col-start-2'}">
        <span class="text-white/50 w-6 text-right">{move.move_number}.</span>
        <span class="font-mono">{move.move_notation}</span>
      </div>
    {/each}
  </div>
</div>
