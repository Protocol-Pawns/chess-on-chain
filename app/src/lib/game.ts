import type { GameOverview } from '$lib/api/client';

export interface ChessMoveEvent {
  color: string;
  from: string;
  to: string;
}

export function parseChessLogs(logs: string[]): ChessMoveEvent[] {
  const moves: ChessMoveEvent[] = [];
  for (const log of logs) {
    try {
      const json = log.startsWith('EVENT_JSON:') ? log.slice(11) : log;
      const event = JSON.parse(json);
      if (event.standard === 'chess-game' && event.event === 'play_move') {
        const mv: string = event.data.mv;
        const parts = mv.split(' to ');
        if (parts.length === 2) {
          moves.push({
            color: event.data.color,
            from: parts[0],
            to: parts[1]
          });
        }
      }
    } catch {
      // not a JSON log, skip
    }
  }
  return moves;
}

export function parseChessEvents(txResult: unknown): ChessMoveEvent[] {
  const result = txResult as {
    receipts_outcome?: { outcome: { logs: string[] } }[];
  };
  const logs: string[] = [];
  if (result?.receipts_outcome) {
    for (const receipt of result.receipts_outcome) {
      logs.push(...(receipt.outcome?.logs ?? []));
    }
  }
  return parseChessLogs(logs);
}

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
    status: 'in_progress',
    turn_color: info.turn_color
  };
}
