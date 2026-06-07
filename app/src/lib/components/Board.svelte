<script lang="ts">
  import { Chess, type Square as ChessSquare } from 'chess.js';
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

  let chess = $derived.by(() => {
    if (!fen) return null;
    try {
      const c = new Chess(fen);
      return c;
    } catch {
      return null;
    }
  });

  let legalTargets = $derived.by(() => {
    const src = selected ?? dragFrom;
    if (!src || !chess) return new Set<string>();
    const [r, c] = src;
    const sq = posToAlgebraic(r, c) as ChessSquare;
    try {
      const moves = chess.moves({ square: sq, verbose: true });
      return new Set(moves.map(m => m.to));
    } catch {
      return new Set<string>();
    }
  });

  function isLegalTarget(r: number, c: number): boolean {
    const pos = posToAlgebraic(r, c);
    return legalTargets.has(pos);
  }

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

  function isSelectable(r: number, c: number): boolean {
    if (disabled) return false;
    const sq = squares[r][c];
    if (!sq.piece) return false;
    if (!chess) return true;
    const isWhitePiece = sq.piece === sq.piece.toUpperCase();
    const turn = chess.turn();
    return isWhitePiece ? turn === 'w' : turn === 'b';
  }

  function tryMove(from: [number, number], to: [number, number]) {
    const fromSq = posToAlgebraic(from[0], from[1]);
    const toSq = posToAlgebraic(to[0], to[1]);
    if (fromSq === toSq) return;
    if (!isLegalTarget(to[0], to[1])) return;
    onMove?.(fromSq, toSq);
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
      if (isLegalTarget(ar, ac)) {
        tryMove([sr, sc], [ar, ac]);
        selected = null;
      } else if (isSelectable(ar, ac)) {
        selected = [ar, ac];
      } else {
        selected = null;
      }
    } else if (isSelectable(ar, ac)) {
      selected = [ar, ac];
    }
  }

  function handleDragStart(r: number, c: number, e: DragEvent) {
    const [ar, ac] = actualCoords(r, c);
    if (!isSelectable(ar, ac)) {
      e.preventDefault();
      return;
    }
    dragFrom = [ar, ac];
    selected = [ar, ac];
    e.dataTransfer!.effectAllowed = 'move';
    e.dataTransfer!.setData('text/plain', '');
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
    if (dragFrom[0] !== ar || dragFrom[1] !== ac) {
      tryMove(dragFrom, [ar, ac]);
    }
    dragFrom = null;
    selected = null;
  }

  function handleDragEnd() {
    dragFrom = null;
    dragOverPos = null;
    selected = null;
  }

  function cellClass(r: number, c: number): string {
    const [ar, ac] = actualCoords(r, c);
    const sq = squares[ar][ac];
    const isLight = sq.isLight;

    let bg = isLight ? 'bg-board-light' : 'bg-board-dark';

    if (lastMove) {
      const pos = posToAlgebraic(ar, ac);
      if (lastMove.from === pos || lastMove.to === pos) {
        bg = isLight ? 'bg-[#b6da95]/60' : 'bg-[#6a9f4b]/60';
      }
    }

    let overlay = '';
    if (selected && selected[0] === ar && selected[1] === ac) {
      overlay = 'ring-2 ring-inset ring-primary-green';
    }
    if (dragFrom && dragFrom[0] === ar && dragFrom[1] === ac) {
      overlay = 'opacity-60';
    }
    if (
      dragOverPos &&
      dragOverPos[0] === ar &&
      dragOverPos[1] === ac &&
      dragFrom &&
      !(dragFrom[0] === ar && dragFrom[1] === ac)
    ) {
      overlay = 'ring-2 ring-inset ring-primary-green';
    }

    let dot = '';
    if (selected && isLegalTarget(ar, ac)) {
      if (sq.piece) {
        dot = 'ring-2 ring-inset ring-primary-green/70';
      } else {
        dot =
          'after:content-[""] after:absolute after:w-[30%] after:h-[30%] after:rounded-full after:bg-black/20';
      }
    }

    return `${bg} ${overlay} ${dot}`;
  }

  const ranks = ['8', '7', '6', '5', '4', '3', '2', '1'];
  const files = ['a', 'b', 'c', 'd', 'e', 'f', 'g', 'h'];
</script>

<!-- svelte-ignore a11y_no_static_element_interactions -->
<div
  class="inline-grid grid-cols-8"
  style="width: min(100%, 30rem); aspect-ratio: 1; user-select: none; -webkit-user-select: none; touch-action: none;"
  ondragover={e => e.preventDefault()}
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
