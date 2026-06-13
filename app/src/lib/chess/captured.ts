const SORT_ORDER = 'qrbnp';

const STARTING_PIECES: Record<string, number> = {
  P: 8,
  N: 2,
  B: 2,
  R: 2,
  Q: 1,
  K: 1,
  p: 8,
  n: 2,
  b: 2,
  r: 2,
  q: 1,
  k: 1
};

function countBoardPieces(board: string[]): Record<string, number> {
  const counts: Record<string, number> = {};
  for (const row of board) {
    for (const ch of row) {
      if (ch !== ' ' && ch !== undefined) {
        counts[ch] = (counts[ch] ?? 0) + 1;
      }
    }
  }
  return counts;
}

function countFenPieces(fen: string): Record<string, number> {
  const placement = fen.split(' ')[0];
  const counts: Record<string, number> = {};
  for (const ch of placement) {
    if (/[a-zA-Z]/.test(ch)) {
      counts[ch] = (counts[ch] ?? 0) + 1;
    }
  }
  return counts;
}

function sortPieces(pieces: string[]): string[] {
  return [...pieces].sort((a, b) => {
    const va = SORT_ORDER.indexOf(a.toLowerCase());
    const vb = SORT_ORDER.indexOf(b.toLowerCase());
    return va - vb;
  });
}

export function getCapturedPieces(boardOrFen: string[] | string): {
  whiteCaptured: string[];
  blackCaptured: string[];
} {
  const current =
    typeof boardOrFen === 'string'
      ? countFenPieces(boardOrFen)
      : countBoardPieces(boardOrFen);

  const whiteMissing: string[] = [];
  const blackMissing: string[] = [];

  for (const [piece, startCount] of Object.entries(STARTING_PIECES)) {
    const now = current[piece] ?? 0;
    const missing = startCount - now;
    if (missing <= 0) continue;
    for (let i = 0; i < missing; i++) {
      const isWhite = piece === piece.toUpperCase();
      (isWhite ? whiteMissing : blackMissing).push(piece);
    }
  }

  return {
    whiteCaptured: sortPieces(blackMissing),
    blackCaptured: sortPieces(whiteMissing)
  };
}
