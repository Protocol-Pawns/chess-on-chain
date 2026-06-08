<script lang="ts">
  import type { GameMove } from '$lib/api/client';

  let {
    moves,
    selectedMoveIndex,
    onSelectMove,
    isViewingCurrent,
    isLiveMyTurn
  }: {
    moves: GameMove[];
    selectedMoveIndex: number | null;
    onSelectMove: (index: number | null) => void;
    isViewingCurrent: boolean;
    isLiveMyTurn: boolean;
  } = $props();

  let containerEl: HTMLDivElement | undefined = $state();

  function goPrev() {
    if (selectedMoveIndex === null) {
      if (moves.length > 1) onSelectMove(moves.length - 2);
      else if (moves.length > 0) onSelectMove(-1);
    } else if (selectedMoveIndex > -1) {
      onSelectMove(selectedMoveIndex - 1);
    }
  }

  function goNext() {
    if (selectedMoveIndex === null) return;
    if (selectedMoveIndex < moves.length - 1) {
      onSelectMove(selectedMoveIndex + 1);
    } else {
      onSelectMove(null);
    }
  }

  function goLatest() {
    onSelectMove(null);
  }

  let canGoPrev = $derived(
    selectedMoveIndex === null ? moves.length > 0 : selectedMoveIndex > -1
  );
  let canGoNext = $derived(
    selectedMoveIndex !== null && selectedMoveIndex < moves.length - 1
  );

  $effect(() => {
    if (!containerEl) return;
    const idx =
      selectedMoveIndex !== null && selectedMoveIndex >= 0
        ? selectedMoveIndex
        : isViewingCurrent && moves.length > 0
          ? moves.length - 1
          : null;
    if (idx === null) return;
    const el = containerEl.querySelector(
      `[data-move-idx="${idx}"]`
    ) as HTMLElement;
    if (el) {
      el.scrollIntoView({ block: 'nearest', behavior: 'smooth' });
    }
  });

  function handleKeydown(e: KeyboardEvent) {
    if (e.key === 'ArrowLeft') {
      e.preventDefault();
      goPrev();
    } else if (e.key === 'ArrowRight') {
      e.preventDefault();
      goNext();
    }
  }
</script>

<svelte:window onkeydown={handleKeydown} />

<div class="card">
  <div class="flex items-center justify-between mb-2">
    <h3 class="text-base font-semibold">Moves</h3>
    <div class="flex items-center gap-0.5">
      <button
        class="p-1 rounded border border-white/20 hover:bg-white/10 disabled:opacity-30 disabled:cursor-default transition-colors"
        onclick={() => onSelectMove(-1)}
        disabled={selectedMoveIndex === -1 || moves.length === 0}
        title="Start"
        ><svg
          xmlns="http://www.w3.org/2000/svg"
          width="16"
          height="16"
          viewBox="0 0 24 24"
          fill="none"
          stroke="currentColor"
          stroke-width="2"
          stroke-linecap="round"
          stroke-linejoin="round"
          ><line x1="3" y1="4" x2="3" y2="20" /><polygon
            points="7 4 19 12 7 20"
          /></svg
        ></button
      >
      <button
        class="p-1 rounded border border-white/20 hover:bg-white/10 disabled:opacity-30 disabled:cursor-default transition-colors"
        onclick={goPrev}
        disabled={!canGoPrev}
        title="Previous move"
        ><svg
          xmlns="http://www.w3.org/2000/svg"
          width="16"
          height="16"
          viewBox="0 0 24 24"
          fill="none"
          stroke="currentColor"
          stroke-width="2"
          stroke-linecap="round"
          stroke-linejoin="round"><polyline points="15 18 9 12 15 6" /></svg
        ></button
      >
      <button
        class="p-1 rounded border border-white/20 hover:bg-white/10 disabled:opacity-30 disabled:cursor-default transition-colors"
        onclick={goNext}
        disabled={!canGoNext}
        title="Next move"
        ><svg
          xmlns="http://www.w3.org/2000/svg"
          width="16"
          height="16"
          viewBox="0 0 24 24"
          fill="none"
          stroke="currentColor"
          stroke-width="2"
          stroke-linecap="round"
          stroke-linejoin="round"><polyline points="9 18 15 12 9 6" /></svg
        ></button
      >
      <button
        class="p-1 rounded border border-white/20 hover:bg-white/10 disabled:opacity-30 disabled:cursor-default transition-colors"
        onclick={goLatest}
        disabled={isViewingCurrent || selectedMoveIndex === moves.length - 1}
        title="Latest position"
        ><svg
          xmlns="http://www.w3.org/2000/svg"
          width="16"
          height="16"
          viewBox="0 0 24 24"
          fill="none"
          stroke="currentColor"
          stroke-width="2"
          stroke-linecap="round"
          stroke-linejoin="round"
          ><polygon points="5 4 17 12 5 20" /><line
            x1="21"
            y1="4"
            x2="21"
            y2="20"
          /></svg
        ></button
      >
    </div>
  </div>

  <div class="max-h-34 overflow-y-auto" bind:this={containerEl}>
    {#if moves.length === 0}
      <div class="text-sm text-white/40 py-2">No moves yet</div>
    {:else}
      <div class="flex flex-wrap text-sm">
        {#each moves as move, i}
          <button
            data-move-idx={i}
            class="flex gap-2 text-left rounded px-1 py-0.5 transition-colors w-1/2 {selectedMoveIndex ===
              i ||
            (isViewingCurrent && i === moves.length - 1)
              ? 'bg-white/10 ring-1 ring-white/30'
              : 'hover:bg-white/5'}"
            onclick={() => onSelectMove(i)}
          >
            <span class="text-white/50 w-6 text-right shrink-0"
              >{move.move_number}.</span
            >
            <span class="font-mono">{move.move_notation}</span>
          </button>
        {/each}
      </div>
    {/if}
  </div>

  {#if !isViewingCurrent && isLiveMyTurn}
    <div class="mt-2 text-center">
      <button
        class="text-xs font-semibold px-3 py-1 rounded bg-primary-green/20 text-primary-green border border-primary-green/30 hover:bg-primary-green/30 animate-pulse transition-colors"
        onclick={goLatest}>Return to current &mdash; it's your turn</button
      >
    </div>
  {/if}
</div>
