import { boardFromInput } from './board';

import { fmtDecimals } from '$lib/format';

const CARD_W = 600;
const SQ = 60;
const BOARD = SQ * 8;
const BOARD_L = 60;
const BOARD_T = 100;

const LIGHT = '#f0d9b5';
const DARK = '#b58863';
const BG = '#1a1a2e';
const LM_L = '#b6da95';
const LM_D = '#6a9f4b';

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

const imgCache = new Map<string, HTMLImageElement>();

function imgPath(piece: string): string {
  return `/pieces/${piece === piece.toUpperCase() ? 'w' : 'b'}${piece.toUpperCase()}.webp`;
}

function loadImg(piece: string): Promise<HTMLImageElement | null> {
  const cached = imgCache.get(piece);
  if (cached) return Promise.resolve(cached);
  return new Promise(resolve => {
    const img = new Image();
    img.crossOrigin = 'anonymous';
    img.onload = () => {
      imgCache.set(piece, img);
      resolve(img);
    };
    img.onerror = () => resolve(null);
    img.src = imgPath(piece);
  });
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

export async function renderGameCard(opts: RenderCardOpts): Promise<Blob> {
  const squares = boardFromInput(opts.board, opts.fen);

  const pieces = new Set<string>();
  for (const row of squares)
    for (const sq of row) if (sq.piece) pieces.add(sq.piece);
  const imgs = new Map<string, HTMLImageElement | null>();
  await Promise.all([...pieces].map(async p => imgs.set(p, await loadImg(p))));

  const h = BOARD_T + BOARD + 60;
  const canvas = document.createElement('canvas');
  canvas.width = CARD_W;
  canvas.height = h;
  const ctx = canvas.getContext('2d')!;

  roundRect(ctx, 0, 0, CARD_W, h, 12);
  ctx.fillStyle = BG;
  ctx.fill();

  const nameY = 40;
  ctx.textBaseline = 'middle';

  ctx.font =
    '600 17px -apple-system, BlinkMacSystemFont, "Segoe UI", Roboto, sans-serif';

  function drawPlayerName(
    name: string,
    elo: number | null | undefined,
    x: number,
    align: 'left' | 'right',
    dotColor: string,
    dotStroke: string
  ) {
    ctx.font =
      '600 17px -apple-system, BlinkMacSystemFont, "Segoe UI", Roboto, sans-serif';
    ctx.textAlign = align;
    ctx.fillStyle = '#fff';
    const nameW = ctx.measureText(name).width;
    let eloText = '';
    let eloW = 0;
    if (elo != null) {
      ctx.font =
        '13px -apple-system, BlinkMacSystemFont, "Segoe UI", Roboto, sans-serif';
      eloText = ' \u00b7 ' + fmtDecimals(elo);
      eloW = ctx.measureText(eloText).width;
    }
    const dotCX = align === 'left' ? x : x - eloW - 14;
    const nameX = align === 'left' ? x + 18 : x - eloW;
    const eloX = align === 'left' ? x + 18 + nameW + 2 : x;
    ctx.beginPath();
    ctx.arc(dotCX, nameY, 9, 0, Math.PI * 2);
    ctx.fillStyle = dotColor;
    ctx.fill();
    ctx.strokeStyle = dotStroke;
    ctx.lineWidth = 1;
    ctx.stroke();
    ctx.font =
      '600 17px -apple-system, BlinkMacSystemFont, "Segoe UI", Roboto, sans-serif';
    ctx.textAlign = align;
    ctx.fillStyle = '#fff';
    ctx.fillText(name, nameX, nameY);
    if (elo != null) {
      ctx.fillStyle = 'rgba(255,255,255,0.4)';
      ctx.font =
        '13px -apple-system, BlinkMacSystemFont, "Segoe UI", Roboto, sans-serif';
      ctx.textAlign = align;
      ctx.fillText(eloText, eloX, nameY + 1);
    }
  }

  drawPlayerName(opts.whiteName, opts.whiteElo, 50, 'left', '#fff', '#999');
  drawPlayerName(
    opts.blackName,
    opts.blackElo,
    CARD_W - 50,
    'right',
    '#1a1a1a',
    '#666'
  );

  ctx.textAlign = 'center';
  ctx.fillStyle = 'rgba(255,255,255,0.4)';
  ctx.font =
    '14px -apple-system, BlinkMacSystemFont, "Segoe UI", Roboto, sans-serif';
  ctx.fillText('vs', CARD_W / 2, nameY);

  for (let r = 0; r < 8; r++) {
    for (let c = 0; c < 8; c++) {
      const x = BOARD_L + c * SQ;
      const y = BOARD_T + r * SQ;
      const sq = squares[r][c];
      const light = sq.isLight;

      ctx.fillStyle = light ? LIGHT : DARK;
      ctx.fillRect(x, y, SQ, SQ);

      if (opts.lastMove) {
        const pos = String.fromCharCode(97 + c) + (8 - r);
        if (opts.lastMove.from === pos || opts.lastMove.to === pos) {
          ctx.fillStyle = light ? LM_L : LM_D;
          ctx.fillRect(x, y, SQ, SQ);
        }
      }

      if (sq.piece) {
        const img = imgs.get(sq.piece);
        if (img) {
          const p = 6;
          ctx.drawImage(img, x + p, y + p, SQ - p * 2, SQ - p * 2);
        } else {
          ctx.fillStyle = light ? '#333' : '#fff';
          ctx.font = `${SQ * 0.7}px serif`;
          ctx.textAlign = 'center';
          ctx.textBaseline = 'middle';
          ctx.fillText(
            UNICODE[sq.piece] || sq.piece,
            x + SQ / 2,
            y + SQ / 2 + 2
          );
        }
      }

      if (c === 0) {
        ctx.fillStyle = light ? DARK : LIGHT;
        ctx.font =
          'bold 11px -apple-system, BlinkMacSystemFont, "Segoe UI", Roboto, sans-serif';
        ctx.textAlign = 'left';
        ctx.textBaseline = 'top';
        ctx.fillText(String(8 - r), x + 3, y + 2);
      }

      if (r === 7) {
        ctx.fillStyle = light ? DARK : LIGHT;
        ctx.font =
          'bold 11px -apple-system, BlinkMacSystemFont, "Segoe UI", Roboto, sans-serif';
        ctx.textAlign = 'right';
        ctx.textBaseline = 'bottom';
        ctx.fillText(String.fromCharCode(97 + c), x + SQ - 3, y + SQ - 2);
      }
    }
  }

  ctx.textAlign = 'center';
  ctx.textBaseline = 'top';
  ctx.fillStyle = '#fff';
  ctx.font =
    'bold 20px -apple-system, BlinkMacSystemFont, "Segoe UI", Roboto, sans-serif';
  const resultY = BOARD_T + BOARD + 18;
  ctx.fillText(opts.result, CARD_W / 2, resultY);

  ctx.fillStyle = 'rgba(255,255,255,0.35)';
  ctx.font =
    '13px -apple-system, BlinkMacSystemFont, "Segoe UI", Roboto, sans-serif';
  ctx.fillText('Protocol Pawns', CARD_W / 2, resultY + 32);

  return new Promise(resolve => canvas.toBlob(b => resolve(b!), 'image/png'));
}

function roundRect(
  ctx: CanvasRenderingContext2D,
  x: number,
  y: number,
  w: number,
  h: number,
  r: number
) {
  ctx.beginPath();
  ctx.moveTo(x + r, y);
  ctx.lineTo(x + w - r, y);
  ctx.quadraticCurveTo(x + w, y, x + w, y + r);
  ctx.lineTo(x + w, y + h - r);
  ctx.quadraticCurveTo(x + w, y + h, x + w - r, y + h);
  ctx.lineTo(x + r, y + h);
  ctx.quadraticCurveTo(x, y + h, x, y + h - r);
  ctx.lineTo(x, y + r);
  ctx.quadraticCurveTo(x, y, x + r, y);
  ctx.closePath();
}
