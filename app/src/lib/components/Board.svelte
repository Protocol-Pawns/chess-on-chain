<script lang="ts">
  import {
    boardFromInput,
    posToAlgebraic,
    type Square
  } from '$lib/chess/board';

  let {
    board,
    fen,
    onMove,
    flipped = false,
    lastMove,
    disabled = false
  }: {
    board?: string[];
    fen?: string;
    onMove?: (from: string, to: string) => void;
    flipped?: boolean;
    lastMove?: { from: string; to: string } | null;
    disabled?: boolean;
  } = $props();

  let selected: [number, number] | null = $state(null);
  let dragFrom: [number, number] | null = $state(null);
  let dragOverPos: [number, number] | null = $state(null);

  let squares = $derived(boardFromInput(board, fen));

  const PIECE_IMG: Record<string, string> = {
    K: '/pieces/wK.webp',
    Q: '/pieces/wQ.webp',
    R: '/pieces/wR.webp',
    B: '/pieces/wB.webp',
    N: '/pieces/wN.webp',
    P: '/pieces/wP.webp',
    k: '/pieces/bK.webp',
    q: '/pieces/bQ.webp',
    r: '/pieces/bR.webp',
    b: '/pieces/bB.webp',
    n: '/pieces/bN.webp',
    p: '/pieces/bP.webp'
  };

  function getRows(): Square[][] {
    const rows = [...squares];
    if (flipped) rows.reverse();
    return rows.map(r => (flipped ? [...r].reverse() : r));
  }

  function actualCoords(r: number, c: number): [number, number] {
    return flipped ? [7 - r, 7 - c] : [r, c];
  }

  function handleClick(r: number, c: number) {
    if (disabled) return;
    const [ar, ac] = actualCoords(r, c);
    const sq = squares[ar][ac];

    if (selected) {
      const [sr, sc] = selected;
      if (sr === ar && sc === ac) {
        selected = null;
        return;
      }
      const from = posToAlgebraic(sr, sc);
      const to = posToAlgebraic(ar, ac);
      onMove?.(from, to);
      selected = null;
    } else if (sq.piece) {
      selected = [ar, ac];
    }
  }

  function handleDragStart(r: number, c: number, e: DragEvent) {
    if (disabled) return;
    const [ar, ac] = actualCoords(r, c);
    dragFrom = [ar, ac];
    selected = null;
    e.dataTransfer!.effectAllowed = 'move';
  }

  function handleDragOver(r: number, c: number, e: DragEvent) {
    e.preventDefault();
    e.dataTransfer!.dropEffect = 'move';
    const [ar, ac] = actualCoords(r, c);
    dragOverPos = [ar, ac];
  }

  function handleDrop(r: number, c: number, e: DragEvent) {
    e.preventDefault();
    dragOverPos = null;
    if (!dragFrom) return;
    const [ar, ac] = actualCoords(r, c);
    const from = posToAlgebraic(dragFrom[0], dragFrom[1]);
    const to = posToAlgebraic(ar, ac);
    onMove?.(from, to);
    dragFrom = null;
  }

  function handleDragEnd() {
    dragFrom = null;
    dragOverPos = null;
  }

  function cellClass(r: number, c: number): string {
    const [ar, ac] = actualCoords(r, c);
    const sq = squares[ar][ac];
    const isLight = sq.isLight;

    let bg = isLight ? 'bg-board-light' : 'bg-board-dark';

    if (lastMove) {
      const pos = posToAlgebraic(ar, ac);
      if (lastMove.from === pos || lastMove.to === pos) {
        bg = isLight ? 'bg-[#b6da95]' : 'bg-[#6a9f4b]';
      }
    }

    let overlay = '';
    if (selected && selected[0] === ar && selected[1] === ac) {
      overlay = 'ring-2 ring-inset ring-primary-green';
    }
    if (dragFrom && dragFrom[0] === ar && dragFrom[1] === ac) {
      overlay = 'opacity-60';
    }
    if (dragOverPos && dragOverPos[0] === ar && dragOverPos[1] === ac) {
      overlay = 'ring-2 ring-inset ring-primary-green';
    }

    return `${bg} ${overlay}`;
  }

  const ranks = ['8', '7', '6', '5', '4', '3', '2', '1'];
  const files = ['a', 'b', 'c', 'd', 'e', 'f', 'g', 'h'];
</script>

<div
  class="inline-grid grid-cols-8 select-none"
  style="width: min(100%, 30rem); aspect-ratio: 1;"
>
  {#each getRows() as row, r}
    {#each row as sq, c}
      <button
        class="flex items-center justify-center relative aspect-square transition-colors {cellClass(
          r,
          c
        )} {sq.piece && !disabled ? 'cursor-pointer' : ''}"
        draggable={sq.piece && !disabled ? true : undefined}
        onclick={() => handleClick(r, c)}
        ondragstart={e => handleDragStart(r, c, e)}
        ondragover={e => handleDragOver(r, c, e)}
        ondrop={e => handleDrop(r, c, e)}
        ondragend={handleDragEnd}
      >
        {#if sq.piece}
          <img
            src={PIECE_IMG[sq.piece]}
            alt={sq.piece}
            class="w-[85%] h-[85%] object-contain pointer-events-none"
            draggable="false"
          />
        {/if}
        {#if c === 0}
          <span
            class="absolute top-0.5 left-1 text-xs font-semibold {sq.isLight
              ? 'text-board-dark'
              : 'text-board-light'}"
          >
            {flipped ? ranks[7 - r] : ranks[r]}
          </span>
        {/if}
        {#if r === 7}
          <span
            class="absolute bottom-0.5 right-1 text-xs font-semibold {sq.isLight
              ? 'text-board-dark'
              : 'text-board-light'}"
          >
            {flipped ? files[7 - c] : files[c]}
          </span>
        {/if}
      </button>
    {/each}
  {/each}
</div>
