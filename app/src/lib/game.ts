import type { GameOverview } from '$lib/api/client';
import type { GameId, Player, Color } from '$lib/near/contract-types';

export const MAX_OPEN_GAMES = 5;

export type { GameId };

export function normalizePlayer(p: Player | null): {
  type: string;
  value: string | null;
} {
  if (!p) return { type: 'AI', value: null };
  if (p.type === 'Ai') return { type: 'AI', value: p.value };
  return p;
}

export function gameUrl(gameId: GameId): string {
  const [num, p1, p2] = gameId;
  return p2 ? `/game/${num}/${p1}/${p2}` : `/game/${num}/${p1}`;
}

export function parseGamePath(path: string): GameId {
  const parts = path.split('/');
  const num = Number(parts[0]);
  const p1 = parts[1];
  const p2 = parts[2] ?? null;
  return [num, p1, p2];
}

export function boardToFen(board: string[], turnColor: Color): string {
  const rows: string[] = [];
  for (let i = 7; i >= 0; i--) {
    let fenRow = '';
    let emptyCount = 0;
    for (let j = 0; j < 8; j++) {
      const ch = board[i][j];
      if (ch === ' ' || ch === undefined) {
        emptyCount++;
      } else {
        if (emptyCount > 0) {
          fenRow += emptyCount;
          emptyCount = 0;
        }
        fenRow += ch;
      }
    }
    if (emptyCount > 0) {
      fenRow += emptyCount;
    }
    rows.push(fenRow);
  }
  const turn = turnColor === 'White' ? 'w' : 'b';
  return `${rows.join('/')} ${turn} - - 0 1`;
}

export interface ContractGameData extends GameOverview {
  turn_color: Color;
}

export async function loadGameFromContract(
  gameId: GameId
): Promise<ContractGameData> {
  const { contract } = await import('$lib/near/connector');
  const [info, rawBoard] = await Promise.all([
    contract.getGameInfo(gameId),
    contract.getBoard(gameId)
  ]);
  const board = rawBoard;
  const fen = boardToFen(board, info.turn_color);
  return {
    game_id: gameId,
    white: normalizePlayer(info.white) as GameOverview['white'],
    black: normalizePlayer(info.black),
    board,
    fen,
    status: 'in_progress' as const,
    turn_color: info.turn_color,
    outcome: null
  };
}
