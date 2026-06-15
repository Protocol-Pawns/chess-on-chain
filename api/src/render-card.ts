import { initWasm, Resvg } from '@resvg/resvg-wasm';

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
}

export async function renderGameCard(
  opts: RenderCardOpts
): Promise<Uint8Array> {
  await ensureWasm();

  const squares = boardFromInput(opts.board, opts.fen);

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
  const blackEloWidth = blackEloText ? measureText(blackEloText, 13, 400) : 0;
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
        svg += `<text x="${x + 4}" y="${y + 4}" font-size="13" font-weight="bold" fill="${light ? LABEL_LIGHT : LABEL_DARK}">${8 - r}</text>`;
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

function pieceKey(piece: string): string {
  return `${piece === piece.toUpperCase() ? 'w' : 'b'}${piece.toUpperCase()}`;
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
