<script lang="ts">
  import type { GameOverview } from '$lib/api/client';

  let { game }: { game: GameOverview } = $props();
</script>

<div class="card-hover">
  <div class="flex justify-between items-start mb-1">
    <div class="text-sm space-y-0.5">
      <div class="font-medium">
        <span
          class="inline-block w-3 h-3 rounded-full bg-white mr-1 align-middle"
        ></span>
        {game.white.type === 'Human' ? game.white.value : 'AI'}
      </div>
      <div>
        <span
          class="inline-block w-3 h-3 rounded-full bg-gray-700 border border-gray-500 mr-1 align-middle"
        ></span>
        {game.black?.type === 'Human'
          ? game.black.value
          : game.black?.type === 'AI'
            ? 'AI'
            : 'Waiting...'}
      </div>
    </div>
    <span
      class="text-xs {game.status === 'in_progress'
        ? 'text-primary-green'
        : game.status === 'finished'
          ? 'text-white/50'
          : 'text-primary-err'}"
    >
      {game.status === 'in_progress' ? 'Live' : game.status}
    </span>
  </div>
  {#if game.outcome}
    <div class="text-xs text-white/60">
      {game.outcome.result === 'Victory'
        ? `${game.outcome.color} wins`
        : 'Draw'}
    </div>
  {/if}
</div>
