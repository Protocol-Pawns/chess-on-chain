<script lang="ts">
  import { Chess, type Square as ChessSquare } from 'chess.js';
  import {
    boardFromInput,
    posToAlgebraic,
    type Square
  } from '$lib/chess/board';
  import { castlingRights } from '$lib/game';
  import Modal from './Modal.svelte';

  let {
    board,
    fen,
    onMove,
    flipped = false,
    lastMove,
    disabled = false,
    loading = false
  }: {
    board?: string[];
    fen?: string;
    onMove?: (from: string, to: string, promotion?: string) => void;
    flipped?: boolean;
    lastMove?: { from: string; to: string } | null;
    disabled?: boolean;
    loading?: boolean;
  } = $props();

  let selected: [number, number] | null = $state(null);
  let dragFrom: [number, number] | null = $state(null);
  let dragOverPos: [number, number] | null = $state(null);
  let pendingPromotion = $state<{
    from: [number, number];
    to: [number, number];
  } | null>(null);

  function isPromotion(from: [number, number], to: [number, number]): boolean {
    if (!chess) return false;
    const fromSq = posToAlgebraic(from[0], from[1]) as ChessSquare;
    const toSq = posToAlgebraic(to[0], to[1]);
    try {
      const moves = chess.moves({ square: fromSq, verbose: true });
      return moves.some(m => m.to === toSq && m.promotion);
    } catch {
      return false;
    }
  }

  const PROMOTION_PIECES = ['q', 'r', 'b', 'n'] as const;
  const PROMOTION_NAMES: Record<string, string> = {
    q: 'queen',
    r: 'rook',
    b: 'bishop',
    n: 'knight'
  };
  const PROMOTION_LABELS: Record<string, string> = {
    q: 'Q',
    r: 'R',
    b: 'B',
    n: 'N'
  };

  function selectPromotion(piece: string) {
    if (!pendingPromotion) return;
    const { from, to } = pendingPromotion;
    const fromSq = posToAlgebraic(from[0], from[1]);
    const toSq = posToAlgebraic(to[0], to[1]);
    onMove?.(fromSq, toSq, PROMOTION_NAMES[piece]);
    pendingPromotion = null;
  }

  let squares = $derived(boardFromInput(board, fen));

  let boardArr = $derived.by(() => {
    if (board) return board;
    if (!fen) return null;
    const placement = fen.split(' ')[0];
    const parts = placement.split('/').reverse();
    return parts.map(row => {
      let s = '';
      for (const ch of row) {
        if (/\d/.test(ch)) s += ' '.repeat(parseInt(ch));
        else s += ch;
      }
      return s;
    });
  });

  let chessFen = $derived.by(() => {
    if (!fen) return null;
    if (!boardArr) return fen;
    const parts = fen.split(' ');
    parts[2] = castlingRights(boardArr);
    return parts.join(' ');
  });

  let chess = $derived.by(() => {
    if (!chessFen) return null;
    try {
      const c = new Chess(chessFen);
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

  let inCheck = $derived(chess?.inCheck() ?? false);

  let checkSquare: [number, number] | null = $derived.by(() => {
    if (!inCheck || !chess) return null;
    const b = chess.board();
    const turn = chess.turn();
    for (let r = 0; r < 8; r++) {
      for (let c = 0; c < 8; c++) {
        const piece = b[r][c];
        if (piece && piece.type === 'k' && piece.color === turn) {
          return [r, c];
        }
      }
    }
    return null;
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
    if (isPromotion(from, to)) {
      pendingPromotion = { from, to };
      return;
    }
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

    if (checkSquare) {
      const pos = posToAlgebraic(checkSquare[0], checkSquare[1]);
      const cellPos = posToAlgebraic(ar, ac);
      if (pos === cellPos) {
        bg = isLight ? 'bg-[#e06b6b]/80' : 'bg-[#cc3333]/80';
      }
    }

    let overlay = '';
    if (selected && selected[0] === ar && selected[1] === ac) {
      overlay = 'bg-yellow-400/40 ring-2 ring-inset ring-yellow-400';
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
      overlay = 'bg-yellow-400/40 ring-2 ring-inset ring-yellow-400';
    }

    let dot = '';
    if (selected && isLegalTarget(ar, ac)) {
      if (sq.piece) {
        dot = 'ring-3 ring-inset ring-red-500/80';
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

<div class="relative max-lg:max-w-[26rem]" style="width: 100%;">
  {#if loading}
    <div
      class="absolute inset-0 flex items-center justify-center bg-black/40 z-10 rounded"
    >
      <div
        class="w-10 h-10 border-4 border-white/30 border-t-white rounded-full animate-spin"
      ></div>
    </div>
  {/if}
  <!-- svelte-ignore a11y_no_static_element_interactions -->
  <div
    class="inline-grid grid-cols-8"
    style="width: 100%; aspect-ratio: 1; user-select: none; -webkit-user-select: none; touch-action: none;"
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
</div>
<Modal open={!!pendingPromotion} onclose={() => (pendingPromotion = null)}>
  <div
    class="grid grid-cols-2 gap-3 p-4 rounded-xl bg-gray-900/95 shadow-2xl border border-white/20"
  >
    <p class="text-center text-sm font-semibold text-white/80 mb-1">
      Promote to
    </p>
    {#each PROMOTION_PIECES as piece}
      {@const color = chess?.turn() === 'w' ? 'w' : 'b'}
      <button
        class="w-16 h-16 flex items-center justify-center rounded-lg bg-white/10 hover:bg-white/20 transition-colors border border-white/20 hover:border-white/50 cursor-pointer"
        onclick={() => selectPromotion(piece)}
      >
        <img
          src="/pieces/{color}{PROMOTION_LABELS[piece]}.webp"
          alt={piece}
          class="w-12 h-12 object-contain pointer-events-none"
          draggable="false"
        />
      </button>
    {/each}
  </div>
</Modal>
