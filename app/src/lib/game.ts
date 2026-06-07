import type { GameOverview } from '$lib/api/client';

export type GameId = [number, string, string | null];

export function normalizePlayer(
  p: { type: string; value: string | null } | null
): { type: string; value: string | null } {
  if (!p) return { type: 'AI', value: null };
  if (p.type === 'Ai') return { type: 'AI', value: p.value };
  return p;
}

export function gameUrl(gameId: GameId): string {
  return `/game/${encodeURIComponent(JSON.stringify(gameId))}`;
}

export function boardToFen(board: string[], turnColor: string): string {
  const rows: string[] = [];
  for (let i = 0; i < 8; i++) {
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
  turn_color: string;
}

export async function loadGameFromContract(
  gameId: GameId
): Promise<ContractGameData> {
  const { contract } = await import('$lib/near/connector');
  const [info, rawBoard] = await Promise.all([
    contract.getGameInfo(gameId),
    contract.getBoard(gameId)
  ]);
  const board = [...rawBoard].reverse();
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
