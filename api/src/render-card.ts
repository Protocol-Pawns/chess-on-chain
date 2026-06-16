import { initWasm, Resvg } from '@resvg/resvg-wasm';
import { Chess } from 'chess.js';

import { FONT_BASE64 } from './generated/font';
import { PIECES } from './generated/pieces';
import wasmModule from './resvg.wasm';

let wasmReady = false;
let wasmInitPromise: Promise<void> | null = null;

async function ensureWasm(): Promise<void> {
  if (wasmReady) return;
  if (!wasmInitPromise) {
    wasmInitPromise = initWasm(wasmModule).then(() => {
      wasmReady = true;
    });
  }
  return wasmInitPromise;
}

const FONT_BUFFER = Uint8Array.from(atob(FONT_BASE64), c => c.charCodeAt(0));

const CARD_W = 600;
const SQ = 60;
const BOARD = SQ * 8;
const BOARD_L = 60;
const BOARD_T = 100;

const MARGIN_X = 50;
const DOT_R = 9;
const DOT_GAP = 12;

const LIGHT = '#f0d9b5';
const DARK = '#b58863';
const BG = '#1a1a2e';
const LM_L = '#b6da95';
const LM_D = '#6a9f4b';
const LABEL_LIGHT = '#5c4033';
const LABEL_DARK = '#ffffff';
const CHECK_LIGHT = 'rgba(224,107,107,0.85)';
const CHECK_DARK = 'rgba(204,51,51,0.85)';

const UNICODE: Record<string, string> = {
  K: '\u2654',
  Q: '\u2655',
  R: '\u2656',
  B: '\u2657',
  N: '\u2658',
  P: '\u2659',
  k: '\u265A',
  q: '\u265B',
  r: '\u265C',
  b: '\u265D',
  n: '\u265E',
  p: '\u265F'
};

interface Square {
  piece: string | null;
  isLight: boolean;
}

function boardFromInput(board?: string[], fen?: string): Square[][] {
  if (fen) return parseFEN(fen);
  if (board) return parseBoard(board);
  return parseBoard([
    'RNBQKBNR',
    'PPPPPPPP',
    '        ',
    '        ',
    '        ',
    '        ',
    'pppppppp',
    'rnbqkbnr'
  ]);
}

function parseBoard(board: string[]): Square[][] {
  return [...board].reverse().map((row, r) =>
    [...row].map((piece, c) => ({
      piece: piece === ' ' ? null : piece,
      isLight: (r + c) % 2 === 0
    }))
  );
}

function parseFEN(fen: string): Square[][] {
  const placement = fen.split(' ')[0];
  const rows = placement.split('/');
  const board: Square[][] = [];
  for (let r = 0; r < rows.length; r++) {
    const row: Square[] = [];
    let c = 0;
    for (const ch of rows[r]) {
      if (/\d/.test(ch)) {
        for (let i = 0; i < parseInt(ch); i++) {
          row.push({ piece: null, isLight: (r + c) % 2 === 0 });
          c++;
        }
      } else {
        row.push({ piece: ch, isLight: (r + c) % 2 === 0 });
        c++;
      }
    }
    board.push(row);
  }
  return board;
}

export interface RenderCardOpts {
  board?: string[];
  fen?: string;
  whiteName: string;
  blackName: string;
  whiteElo?: number | null;
  blackElo?: number | null;
  result: string;
  lastMove?: { from: string; to: string } | null;
  inCheck?: { row: number; col: number } | null;
}

export async function renderGameCard(
  opts: RenderCardOpts
): Promise<Uint8Array> {
  await ensureWasm();

  const squares = boardFromInput(opts.board, opts.fen);
  const checkSquare = opts.inCheck ?? findCheckSquare(opts.fen);

  const h = BOARD_T + BOARD + 60;

  let svg = `<svg xmlns="http://www.w3.org/2000/svg" width="${CARD_W}" height="${h}" viewBox="0 0 ${CARD_W} ${h}">`;
  svg += `<rect width="${CARD_W}" height="${h}" rx="12" fill="${BG}"/>`;

  const nameY = 40;
  const whiteDotCX = MARGIN_X + DOT_R;
  const whiteTextStart = MARGIN_X + DOT_R * 2 + DOT_GAP;
  svg += `<circle cx="${whiteDotCX}" cy="${nameY}" r="${DOT_R}" fill="#fff" stroke="#999" stroke-width="1"/>`;
  svg += `<text x="${whiteTextStart}" y="${nameY}" text-anchor="start" dominant-baseline="middle" font-size="17" font-weight="600" fill="#fff">${escapeXml(opts.whiteName)}</text>`;
  if (opts.whiteElo != null) {
    svg += `<text x="${whiteTextStart + measureText(opts.whiteName, 17, 600) + 4}" y="${nameY + 1}" text-anchor="start" dominant-baseline="middle" font-size="13" fill="rgba(255,255,255,0.4)">· ${opts.whiteElo}</text>`;
  }

  const blackDotCX = CARD_W - MARGIN_X - DOT_R;
  const blackTextEnd = blackDotCX - DOT_R - DOT_GAP;
  const blackEloText = opts.blackElo != null ? ` · ${opts.blackElo}` : '';
  const blackNameWidth = measureText(opts.blackName, 17, 600);
  const blackNameStart = blackTextEnd - blackNameWidth;
  svg += `<circle cx="${blackDotCX}" cy="${nameY}" r="${DOT_R}" fill="#1a1a1a" stroke="#666" stroke-width="1"/>`;
  if (blackEloText) {
    svg += `<text x="${blackNameStart - 4}" y="${nameY + 1}" text-anchor="end" dominant-baseline="middle" font-size="13" fill="rgba(255,255,255,0.4)">${escapeXml(blackEloText)}</text>`;
  }
  svg += `<text x="${blackTextEnd}" y="${nameY}" text-anchor="end" dominant-baseline="middle" font-size="17" font-weight="600" fill="#fff">${escapeXml(opts.blackName)}</text>`;

  svg += `<text x="${CARD_W / 2}" y="${nameY}" text-anchor="middle" dominant-baseline="middle" font-size="14" fill="rgba(255,255,255,0.4)">vs</text>`;

  for (let r = 0; r < 8; r++) {
    for (let c = 0; c < 8; c++) {
      const x = BOARD_L + c * SQ;
      const y = BOARD_T + r * SQ;
      const sq = squares[r][c];
      const light = sq.isLight;

      let fill = light ? LIGHT : DARK;
      if (opts.lastMove) {
        const pos = String.fromCharCode(97 + c) + (8 - r);
        if (opts.lastMove.from === pos || opts.lastMove.to === pos) {
          fill = light ? LM_L : LM_D;
        }
      }

      svg += `<rect x="${x}" y="${y}" width="${SQ}" height="${SQ}" fill="${fill}"/>`;

      if (checkSquare && checkSquare.row === r && checkSquare.col === c) {
        svg += `<rect x="${x}" y="${y}" width="${SQ}" height="${SQ}" fill="${light ? CHECK_LIGHT : CHECK_DARK}"/>`;
      }

      if (sq.piece) {
        const b64 = PIECES[pieceKey(sq.piece)];
        if (b64) {
          const p = 6;
          svg += `<image href="data:image/png;base64,${b64}" x="${x + p}" y="${y + p}" width="${SQ - p * 2}" height="${SQ - p * 2}"/>`;
        } else {
          svg += `<text x="${x + SQ / 2}" y="${y + SQ / 2 + 2}" text-anchor="middle" dominant-baseline="middle" font-size="${SQ * 0.7}" fill="${light ? '#333' : '#fff'}">${UNICODE[sq.piece] || sq.piece}</text>`;
        }
      }

      if (c === 0) {
        svg += `<text x="${x + 4}" y="${y + 3}" dominant-baseline="hanging" font-size="13" font-weight="bold" fill="${light ? LABEL_LIGHT : LABEL_DARK}">${8 - r}</text>`;
      }
      if (r === 7) {
        svg += `<text x="${x + SQ - 4}" y="${y + SQ - 4}" text-anchor="end" font-size="13" font-weight="bold" fill="${light ? LABEL_LIGHT : LABEL_DARK}">${String.fromCharCode(97 + c)}</text>`;
      }
    }
  }

  const resultY = BOARD_T + BOARD + 18;
  svg += `<text x="${CARD_W / 2}" y="${resultY}" text-anchor="middle" font-size="20" font-weight="bold" fill="#fff">${escapeXml(opts.result)}</text>`;
  svg += `<text x="${CARD_W / 2}" y="${resultY + 32}" text-anchor="middle" font-size="13" fill="rgba(255,255,255,0.35)">Protocol Pawns</text>`;

  svg += '</svg>';

  const resvg = new Resvg(svg, {
    background: BG,
    fitTo: { mode: 'original' },
    font: {
      fontBuffers: [FONT_BUFFER],
      defaultFontFamily: 'DejaVu Sans'
    }
  });
  const rendered = resvg.render();
  return rendered.asPng();
}

export interface ProfileCardOpts {
  accountId: string;
  elo: number | null;
  points: string | null;
  wins: number;
  losses: number;
  draws: number;
  totalGames: number;
  winRate: number;
  extras?: {
    winStreak: number;
    maxWinStreak: number;
    betsPlaced: number;
    betsWon: number;
    wagersPlayed: number;
    wagerWins: number;
    challengesSent: number;
  };
}

export async function renderProfileCard(
  opts: ProfileCardOpts
): Promise<Uint8Array> {
  await ensureWasm();

  const W = 1200;
  const H = 630;

  const BG = '#1a1a2e';
  const PRIMARY = '#aed581';
  const WARN = '#eea14a';
  const GREEN = '#2aa876';
  const ERR = '#f36262';
  const CARD_FILL = 'rgba(255,255,255,0.03)';
  const CARD_STROKE = 'rgba(255,255,255,0.1)';
  const BOX_FILL = 'rgba(174,213,129,0.1)';
  const EXTRA_FILL = 'rgba(255,255,255,0.05)';

  let svg = `<svg xmlns="http://www.w3.org/2000/svg" width="${W}" height="${H}" viewBox="0 0 ${W} ${H}">`;
  svg += `<rect width="${W}" height="${H}" fill="${BG}"/>`;
  svg += `<rect x="80" y="45" width="1040" height="540" rx="28" fill="${CARD_FILL}" stroke="${CARD_STROKE}" stroke-width="2"/>`;

  // Name
  svg += `<text x="${W / 2}" y="120" text-anchor="middle" dominant-baseline="middle" font-size="44" font-weight="bold" fill="${PRIMARY}">${escapeXml(opts.accountId)}</text>`;

  // ELO / Points boxes
  const boxW = 300;
  const boxH = 120;
  const boxGap = 50;
  const boxY = 170;
  const boxStartX = (W - (boxW * 2 + boxGap)) / 2;

  const fmtElo = (n: number | null) =>
    n != null
      ? new Intl.NumberFormat('en', {
          minimumFractionDigits: 0,
          maximumFractionDigits: 2
        }).format(n)
      : '—';

  // ELO box
  svg += `<rect x="${boxStartX}" y="${boxY}" width="${boxW}" height="${boxH}" rx="16" fill="${BOX_FILL}"/>`;
  svg += `<text x="${boxStartX + boxW / 2}" y="${boxY + 32}" text-anchor="middle" dominant-baseline="middle" font-size="16" fill="rgba(255,255,255,0.5)">ELO</text>`;
  svg += `<text x="${boxStartX + boxW / 2}" y="${boxY + 76}" text-anchor="middle" dominant-baseline="middle" font-size="42" font-weight="bold" fill="${WARN}">${fmtElo(opts.elo)}</text>`;

  // Points box
  svg += `<rect x="${boxStartX + boxW + boxGap}" y="${boxY}" width="${boxW}" height="${boxH}" rx="16" fill="${BOX_FILL}"/>`;
  svg += `<text x="${boxStartX + boxW + boxGap + boxW / 2}" y="${boxY + 32}" text-anchor="middle" dominant-baseline="middle" font-size="16" fill="rgba(255,255,255,0.5)">Points</text>`;
  svg += `<text x="${boxStartX + boxW + boxGap + boxW / 2}" y="${boxY + 76}" text-anchor="middle" dominant-baseline="middle" font-size="42" font-weight="bold" fill="${PRIMARY}">${opts.points ?? '—'} PPP</text>`;

  // Stats row
  const stats = [
    { label: 'Wins', value: opts.wins, color: GREEN },
    { label: 'Losses', value: opts.losses, color: ERR },
    { label: 'Draws', value: opts.draws, color: '#ffffff' },
    { label: 'Win Rate', value: `${opts.winRate}%`, color: PRIMARY }
  ];
  const statW = 220;
  const statGap = 20;
  const statY = 360;
  const statStartX = (W - (statW * 4 + statGap * 3)) / 2;
  stats.forEach((s, i) => {
    const x = statStartX + i * (statW + statGap);
    svg += `<text x="${x + statW / 2}" y="${statY}" text-anchor="middle" dominant-baseline="middle" font-size="40" font-weight="bold" fill="${s.color}">${s.value}</text>`;
    svg += `<text x="${x + statW / 2}" y="${statY + 42}" text-anchor="middle" dominant-baseline="middle" font-size="17" fill="rgba(255,255,255,0.5)">${s.label}</text>`;
  });

  // Optional extras row
  const extras: { label: string; value: string; color: string }[] = [];
  if (opts.extras) {
    const e = opts.extras;
    if (e.winStreak > 0 || e.maxWinStreak > 0) {
      extras.push({
        label: 'Streak / Best',
        value: `${e.winStreak}/${e.maxWinStreak}`,
        color: WARN
      });
    }
    if (e.betsPlaced > 0) {
      extras.push({
        label: 'Bets Won',
        value: `${e.betsWon}/${e.betsPlaced}`,
        color: GREEN
      });
    }
    if (e.wagersPlayed > 0) {
      extras.push({
        label: 'Wagers Won',
        value: `${e.wagerWins}/${e.wagersPlayed}`,
        color: PRIMARY
      });
    }
    if (e.challengesSent > 0) {
      extras.push({
        label: 'Challenges',
        value: String(e.challengesSent),
        color: '#ffffff'
      });
    }
  }

  if (extras.length > 0) {
    const extraW = 220;
    const extraGap = 20;
    const extraY = 480;
    const shown = extras.slice(0, 4);
    const extraStartX =
      (W - (extraW * shown.length + extraGap * (shown.length - 1))) / 2;
    shown.forEach((e, i) => {
      const x = extraStartX + i * (extraW + extraGap);
      svg += `<rect x="${x}" y="${extraY}" width="${extraW}" height="64" rx="12" fill="${EXTRA_FILL}"/>`;
      svg += `<text x="${x + extraW / 2}" y="${extraY + 26}" text-anchor="middle" dominant-baseline="middle" font-size="22" font-weight="bold" fill="${e.color}">${e.value}</text>`;
      svg += `<text x="${x + extraW / 2}" y="${extraY + 50}" text-anchor="middle" dominant-baseline="middle" font-size="14" fill="rgba(255,255,255,0.5)">${e.label}</text>`;
    });
  }

  svg += `<text x="${W / 2}" y="${H - 22}" text-anchor="middle" dominant-baseline="middle" font-size="14" fill="rgba(255,255,255,0.3)">protocol-pawns.com</text>`;
  svg += '</svg>';

  const resvg = new Resvg(svg, {
    background: BG,
    fitTo: { mode: 'original' },
    font: {
      fontBuffers: [FONT_BUFFER],
      defaultFontFamily: 'DejaVu Sans'
    }
  });
  return resvg.render().asPng();
}

function pieceKey(piece: string): string {
  return `${piece === piece.toUpperCase() ? 'w' : 'b'}${piece.toUpperCase()}`;
}

function findCheckSquare(fen?: string): { row: number; col: number } | null {
  if (!fen) return null;
  try {
    const c = new Chess(fen);
    if (!c.inCheck()) return null;
    const board = c.board();
    const turn = c.turn();
    for (let r = 0; r < 8; r++) {
      for (let col = 0; col < 8; col++) {
        const p = board[r][col];
        if (p && p.type === 'k' && p.color === turn) {
          return { row: r, col };
        }
      }
    }
  } catch {
    // ignore
  }
  return null;
}

function escapeXml(str: string): string {
  return str
    .replace(/&/g, '&amp;')
    .replace(/</g, '&lt;')
    .replace(/>/g, '&gt;')
    .replace(/"/g, '&quot;')
    .replace(/'/g, '&apos;');
}

function measureText(
  text: string,
  fontSize: number,
  fontWeight: number
): number {
  // Rough heuristic: avg width ~ 0.55 * fontSize for sans-serif.
  const weightFactor = fontWeight >= 600 ? 1.05 : 1;
  return text.length * fontSize * 0.55 * weightFactor;
}
