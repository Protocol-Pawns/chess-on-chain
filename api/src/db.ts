import postgres from 'postgres';

import type {
  Color,
  Game,
  GameId,
  GameOutcome,
  Player,
  Account
} from './events';

const MAX_RECENT_LIMIT = 25;

export function getDb(connectionString: string) {
  return postgres(connectionString, {
    ssl: connectionString.includes('localhost')
      ? { rejectUnauthorized: false }
      : true
  });
}

export type Db = ReturnType<typeof getDb>;

interface GameRow {
  game_id: string;
  white_type: string;
  white_value: string;
  black_type: string;
  black_value: string | null;
  board: string[];
  moves: Array<{ color: string; mv: string; board: string[] }>;
  outcome: GameOutcome | null;
  resigner: Color | null;
}

function parsePlayer(type: string, value: string | null): Player {
  if (type === 'Ai')
    return { type: 'Ai', value: value as 'Easy' | 'Medium' | 'Hard' };
  return { type: 'Human', value: value! };
}

function rowToGame(row: GameRow): Game {
  return {
    game_id: JSON.parse(row.game_id),
    white: parsePlayer(row.white_type, row.white_value),
    black: parsePlayer(row.black_type, row.black_value),
    board: row.board,
    moves: row.moves.map(m => ({
      color: m.color as Color,
      mv: m.mv,
      board: m.board
    })),
    outcome: row.outcome,
    resigner: row.resigner
  };
}

function rowToGameOverview(row: GameRow, includeMoves: boolean) {
  const game: Game = rowToGame(row);
  const { moves, ...overview } = game;
  return includeMoves ? { ...overview, moves } : overview;
}

export async function getInfo(db: Db): Promise<{ lastBlockHeight: number }> {
  const rows =
    await db`SELECT COALESCE(MAX(trigger_block_height), 0) AS last_block_height FROM chess_events`;
  return { lastBlockHeight: Number(rows[0].last_block_height) };
}

export async function getGame(db: Db, gameId: string): Promise<Game | null> {
  const rows = await db`SELECT * FROM games WHERE game_id = ${gameId}`;
  if (rows.length === 0) return null;
  return rowToGame(rows[0] as unknown as GameRow);
}

export async function queryGames(
  db: Db,
  gameIds: string[],
  includeMoves: boolean
) {
  if (gameIds.length === 0) return [];
  const rows = await db`SELECT * FROM games WHERE game_id = ANY(${gameIds})`;
  return rows.map((r: unknown) =>
    rowToGameOverview(r as GameRow, includeMoves)
  );
}

export async function getRecentNewGames(
  db: Db,
  limit: number,
  includeMoves: boolean
) {
  const actualLimit = Math.min(limit, MAX_RECENT_LIMIT);
  const rows = await db`
    SELECT * FROM games
    WHERE finished_at IS NULL
    ORDER BY created_at DESC
    LIMIT ${actualLimit}
  `;
  return rows.map((r: unknown) =>
    rowToGameOverview(r as GameRow, includeMoves)
  );
}

export async function getRecentFinishedGames(
  db: Db,
  limit: number,
  includeMoves: boolean
) {
  const actualLimit = Math.min(limit, MAX_RECENT_LIMIT);
  const rows = await db`
    SELECT * FROM games
    WHERE finished_at IS NOT NULL
    ORDER BY finished_at DESC
    LIMIT ${actualLimit}
  `;
  return rows.map((r: unknown) =>
    rowToGameOverview(r as GameRow, includeMoves)
  );
}

export async function getAccount(db: Db, accountId: string): Promise<Account> {
  const rows = await db`
    SELECT game_id FROM account_finished_games
    WHERE account_id = ${accountId}
  `;
  return {
    finishedGameIds: rows.map(
      (r: unknown) => JSON.parse((r as { game_id: string }).game_id) as GameId
    )
  };
}
