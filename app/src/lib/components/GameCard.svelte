<script lang="ts">
  import dayjs from 'dayjs';
  import { colorFromFEN } from '$lib/chess/board';
  import { truncateAddr } from '$lib/format';
  import { accountStore } from '$lib/near/account';
  import type { GameOverview } from '$lib/api/client';

  let { game }: { game: GameOverview } = $props();

  let turn = $derived(
    game.status === 'in_progress' && game.fen ? colorFromFEN(game.fen) : null
  );

  let isMyTurn = $derived.by(() => {
    if (!turn || !$accountStore) return false;
    const myColor =
      game.white.value === $accountStore
        ? 'White'
        : game.black?.value === $accountStore
          ? 'Black'
          : null;
    return turn === myColor;
  });
</script>

<div class={isMyTurn ? 'card-accent' : 'card-hover'}>
  <div class="flex justify-between items-start mb-1">
    <div class="text-sm space-y-0.5">
      <div class="font-medium">
        <span
          class="inline-block w-3 h-3 rounded-full bg-white mr-1 align-middle"
        ></span>
        {game.white.type === 'Human' ? truncateAddr(game.white.value) : 'AI'}
      </div>
      <div>
        <span
          class="inline-block w-3 h-3 rounded-full bg-gray-700 border border-gray-500 mr-1 align-middle"
        ></span>
        {game.black?.type === 'Human'
          ? truncateAddr(game.black.value ?? '')
          : game.black?.type?.toLowerCase() === 'ai'
            ? `AI (${game.black.value})`
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
  <div class="flex justify-between items-center text-xs text-white/40">
    <span>
      {#if game.outcome}
        <span class="text-white/60">
          {#if game.outcome.result === 'Stalemate'}
            Draw
          {:else if game.resigner}
            {game.outcome.color} wins (resign)
          {:else}
            {game.outcome.color} wins (checkmate)
          {/if}
        </span>
      {:else if isMyTurn}
        <span
          class="inline-block text-xs font-bold px-2 py-0.5 rounded bg-primary-bgOk text-primary-green animate-pulse"
        >
          Your turn!
        </span>
      {:else if turn}
        {turn}'s turn
      {/if}
    </span>
    {#if game.created_at}
      <span>{dayjs(game.created_at).format('lll')}</span>
    {/if}
  </div>
</div>
