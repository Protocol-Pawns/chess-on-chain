import postgres from 'postgres';

import type {
  Account,
  AccountStats,
  Bet,
  BetLeaderboardEntry,
  BetStats,
  Challenge,
  Color,
  Game,
  GameMove,
  GameOutcome,
  GlobalStats,
  Player
} from './events';

const MAX_LIMIT = 100;
const DEFAULT_LIMIT = 25;

export function getDb(connectionString: string) {
  return postgres(connectionString, {
    ssl: false,
    max: 1,
    idle_timeout: 10,
    connect_timeout: 15
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
  fen: string | null;
  moves: Array<{ color: string; mv: string; board: string[]; fen?: string }>;
  outcome: GameOutcome | null;
  resigner: Color | null;
  status: string;
  created_at: string;
  finished_at: string | null;
}

interface GameMoveRow {
  move_number: number;
  color: string;
  move_notation: string;
  fen: string;
  outcome: GameOutcome | null;
}

interface ChallengeRow {
  id: string;
  challenger: string;
  challenged: string;
  wager_token: string | null;
  wager_amount: string | null;
  status: string;
  game_id: string | null;
  created_at: string;
  resolved_at: string | null;
}

function parsePlayer(type: string, value: string | null): Player {
  if (type === 'Ai')
    return { type: 'Ai', value: value as 'Easy' | 'Medium' | 'Hard' };
  return { type: 'Human', value: value! };
}

function parseJson<T>(value: T): T {
  return typeof value === 'string' ? JSON.parse(value) : value;
}

function rowToGame(row: GameRow): Game {
  const board = parseJson<string[]>(row.board);
  const moves = parseJson<Array<{ color: string; mv: string; board: string[]; fen?: string }>>(row.moves);
  const outcome = parseJson<GameOutcome | null>(row.outcome);
  return {
    game_id: JSON.parse(row.game_id),
    white: parsePlayer(row.white_type, row.white_value),
    black: parsePlayer(row.black_type, row.black_value),
    board,
    fen: row.fen,
    moves: moves.map(m => ({
      color: m.color as Color,
      mv: m.mv,
      board: parseJson<string[]>(m.board),
      fen: m.fen
    })),
    status: row.status as Game['status'],
    outcome,
    resigner: row.resigner,
    created_at: row.created_at,
    finished_at: row.finished_at
  };
}

function rowToGameOverview(row: GameRow, includeMoves: boolean) {
  const game: Game = rowToGame(row);
  const { moves, ...overview } = game;
  return includeMoves ? { ...overview, moves } : overview;
}

function rowToGameMove(row: GameMoveRow): GameMove {
  return {
    move_number: row.move_number,
    color: row.color as Color,
    move_notation: row.move_notation,
    fen: row.fen,
    outcome: parseJson<GameOutcome | null>(row.outcome)
  };
}

function rowToChallenge(row: ChallengeRow): Challenge {
  return {
    id: row.id,
    challenger: row.challenger,
    challenged: row.challenged,
    wager_token: row.wager_token,
    wager_amount: row.wager_amount,
    status: row.status as Challenge['status'],
    game_id: row.game_id,
    created_at: row.created_at,
    resolved_at: row.resolved_at
  };
}

function clampLimit(
  limit: unknown,
  max = MAX_LIMIT,
  fallback = DEFAULT_LIMIT
): number {
  const n = Number(limit);
  if (!Number.isFinite(n) || n <= 0) return fallback;
  return Math.min(n, max);
}

export async function getInfo(db: Db): Promise<{ lastBlockHeight: number }> {
  const rows =
    await db`SELECT COALESCE(MAX(trigger_block_height), 0) AS last_block_height FROM chess_events`;
  return { lastBlockHeight: Number(rows[0].last_block_height) };
}

export async function getGlobalStats(db: Db): Promise<GlobalStats> {
  const [games, moves] = await Promise.all([
    db`
      SELECT
        COUNT(*) AS total,
        COUNT(*) FILTER (WHERE status = 'in_progress') AS active,
        COUNT(*) FILTER (WHERE status = 'finished') AS finished,
        COUNT(*) FILTER (WHERE status = 'cancelled') AS cancelled
      FROM games
    `,
    db`SELECT COUNT(*) AS total FROM game_moves`
  ]);
  return {
    total_games: Number(games[0].total),
    active_games: Number(games[0].active),
    finished_games: Number(games[0].finished),
    cancelled_games: Number(games[0].cancelled),
    total_moves: Number(moves[0].total)
  };
}

export async function getGame(db: Db, gameId: string): Promise<Game | null> {
  const rows = await db`SELECT * FROM games WHERE game_id = ${gameId}`;
  if (rows.length === 0) return null;
  return rowToGame(rows[0] as unknown as GameRow);
}

export async function getGameMoves(
  db: Db,
  gameId: string
): Promise<GameMove[]> {
  const rows = await db`
    SELECT move_number, color, move_notation, fen, outcome
    FROM game_moves WHERE game_id = ${gameId}
    ORDER BY move_number ASC
  `;
  return rows.map((r: unknown) => rowToGameMove(r as GameMoveRow));
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

export async function getGames(
  db: Db,
  status: 'active' | 'finished',
  cursor: string | null,
  limit: number,
  includeMoves: boolean
) {
  const actualLimit = clampLimit(limit);
  const statusFilter = status === 'active' ? 'in_progress' : 'finished';
  const orderBy = status === 'active' ? 'created_at' : 'finished_at';

  let rows;
  if (cursor) {
    rows = await db`
      SELECT * FROM games
      WHERE status = ${statusFilter} AND ${db(orderBy)} < ${cursor}
      ORDER BY ${db(orderBy)} DESC
      LIMIT ${actualLimit + 1}
    `;
  } else {
    rows = await db`
      SELECT * FROM games
      WHERE status = ${statusFilter}
      ORDER BY ${db(orderBy)} DESC
      LIMIT ${actualLimit + 1}
    `;
  }

  const hasMore = rows.length > actualLimit;
  const items = (hasMore ? rows.slice(0, -1) : rows).map((r: unknown) =>
    rowToGameOverview(r as GameRow, includeMoves)
  );
  const lastItem = items[items.length - 1] as
    | { created_at?: string; finished_at?: string }
    | undefined;
  const nextCursor =
    hasMore && lastItem
      ? ((status === 'active' ? lastItem.created_at : lastItem.finished_at) ??
        null)
      : null;

  return { items, next_cursor: nextCursor };
}

export async function getActiveGame(
  db: Db,
  accountId: string
): Promise<Game | null> {
  const rows = await db`
    SELECT g.* FROM games g
    WHERE g.status = 'in_progress'
      AND (g.white_value = ${accountId} OR g.black_value = ${accountId})
    ORDER BY g.created_at DESC
    LIMIT 1
  `;
  if (rows.length === 0) return null;
  return rowToGame(rows[0] as unknown as GameRow);
}

export async function getAccount(db: Db, accountId: string): Promise<Account> {
  const rows = await db`
    SELECT game_id FROM account_finished_games
    WHERE account_id = ${accountId}
  `;
  return {
    finishedGameIds: rows.map(
      (r: unknown) =>
        JSON.parse((r as { game_id: string }).game_id) as [
          number,
          string,
          string | null
        ]
    )
  };
}

export async function getAccountStats(
  db: Db,
  accountId: string
): Promise<AccountStats> {
  const rows = await db`
    SELECT
      COUNT(*) FILTER (
        WHERE g.outcome IS NOT NULL
          AND g.status = 'finished'
          AND (
            (g.white_value = ${accountId} AND g.outcome->>'result' = 'Victory' AND g.outcome->>'color' = 'White')
            OR (g.black_value = ${accountId} AND g.outcome->>'result' = 'Victory' AND g.outcome->>'color' = 'Black')
          )
      ) AS wins,
      COUNT(*) FILTER (
        WHERE g.outcome IS NOT NULL
          AND g.status = 'finished'
          AND (
            (g.white_value = ${accountId} AND g.outcome->>'result' = 'Victory' AND g.outcome->>'color' = 'Black')
            OR (g.black_value = ${accountId} AND g.outcome->>'result' = 'Victory' AND g.outcome->>'color' = 'White')
          )
      ) AS losses,
      COUNT(*) FILTER (
        WHERE g.outcome IS NOT NULL
          AND g.status = 'finished'
          AND g.outcome->>'result' = 'Stalemate'
      ) AS draws,
      COUNT(*) FILTER (WHERE g.status = 'finished') AS total_games
    FROM games g
    WHERE g.white_value = ${accountId} OR g.black_value = ${accountId}
  `;
  const r = rows[0] as unknown as Record<string, string>;
  return {
    account_id: accountId,
    wins: Number(r.wins),
    losses: Number(r.losses),
    draws: Number(r.draws),
    total_games: Number(r.total_games)
  };
}

export async function getChallenges(
  db: Db,
  accountId: string
): Promise<Challenge[]> {
  const rows = await db`
    SELECT * FROM challenges
    WHERE challenger = ${accountId} OR challenged = ${accountId}
    ORDER BY created_at DESC
  `;
  return rows.map((r: unknown) => rowToChallenge(r as ChallengeRow));
}

export interface PushSubscriptionRow {
  endpoint: string;
  p256dh: string;
  auth: string;
}

export async function addPushSubscription(
  db: Db,
  accountId: string,
  endpoint: string,
  p256dh: string,
  auth: string
): Promise<void> {
  await db`
    INSERT INTO push_subscriptions (endpoint, account_id, p256dh, auth)
    VALUES (${endpoint}, ${accountId}, ${p256dh}, ${auth})
    ON CONFLICT (endpoint) DO UPDATE SET
      account_id = ${accountId},
      p256dh = ${p256dh},
      auth = ${auth}
  `;
}

export async function removePushSubscription(
  db: Db,
  accountId: string,
  endpoint: string
): Promise<boolean> {
  const result = await db`
    DELETE FROM push_subscriptions
    WHERE endpoint = ${endpoint} AND account_id = ${accountId}
  `;
  return result.count > 0;
}

export async function getPushSubscriptions(
  db: Db,
  accountIds: string[]
): Promise<Map<string, PushSubscriptionRow[]>> {
  if (accountIds.length === 0) return new Map();
  const rows = await db`
    SELECT account_id, endpoint, p256dh, auth FROM push_subscriptions
    WHERE account_id = ANY(${accountIds})
  `;
  const map = new Map<string, PushSubscriptionRow[]>();
  for (const r of rows as unknown as Array<{
    account_id: string;
    endpoint: string;
    p256dh: string;
    auth: string;
  }>) {
    const list = map.get(r.account_id) || [];
    list.push({ endpoint: r.endpoint, p256dh: r.p256dh, auth: r.auth });
    map.set(r.account_id, list);
  }
  return map;
}

interface UnnotifiedEvent {
  id: string;
  event_type: string;
  event_data: Record<string, unknown>;
}

export async function getUnnotifiedEvents(db: Db): Promise<UnnotifiedEvent[]> {
  return db`
    SELECT id, event_type, event_data FROM chess_events
    WHERE processed = true AND notified = false
    ORDER BY trigger_block_timestamp::bigint ASC
    LIMIT 100
  ` as Promise<UnnotifiedEvent[]>;
}

export async function markEventsNotified(db: Db, ids: string[]): Promise<void> {
  if (ids.length === 0) return;
  await db`UPDATE chess_events SET notified = true WHERE id = ANY(${ids})`;
}

interface BetRow {
  id: string;
  bettor: string;
  player_0: string;
  player_1: string;
  game_id: string | null;
  token_id: string;
  amount: string;
  winner: string;
  status: string;
  payout: string | null;
  created_at: string;
  locked_at: string | null;
  resolved_at: string | null;
}

function rowToBet(row: BetRow): Bet {
  return {
    id: row.id,
    bettor: row.bettor,
    player_0: row.player_0,
    player_1: row.player_1,
    game_id: row.game_id,
    token_id: row.token_id,
    amount: row.amount,
    winner: row.winner,
    status: row.status as Bet['status'],
    payout: row.payout,
    created_at: row.created_at,
    locked_at: row.locked_at,
    resolved_at: row.resolved_at
  };
}

export async function getBets(
  db: Db,
  accountId: string,
  status: string | null,
  cursor: string | null,
  limit: number
) {
  const actualLimit = clampLimit(limit);
  let rows;
  if (cursor && status) {
    rows = await db`
      SELECT * FROM bets
      WHERE bettor = ${accountId} AND status = ${status} AND created_at < ${cursor}
      ORDER BY created_at DESC
      LIMIT ${actualLimit + 1}
    `;
  } else if (cursor) {
    rows = await db`
      SELECT * FROM bets
      WHERE bettor = ${accountId} AND created_at < ${cursor}
      ORDER BY created_at DESC
      LIMIT ${actualLimit + 1}
    `;
  } else if (status) {
    rows = await db`
      SELECT * FROM bets
      WHERE bettor = ${accountId} AND status = ${status}
      ORDER BY created_at DESC
      LIMIT ${actualLimit + 1}
    `;
  } else {
    rows = await db`
      SELECT * FROM bets
      WHERE bettor = ${accountId}
      ORDER BY created_at DESC
      LIMIT ${actualLimit + 1}
    `;
  }

  const hasMore = rows.length > actualLimit;
  const items = (hasMore ? rows.slice(0, -1) : rows).map((r: unknown) =>
    rowToBet(r as BetRow)
  );
  const lastItem = items[items.length - 1] as { created_at?: string } | undefined;
  const nextCursor = hasMore && lastItem ? lastItem.created_at ?? null : null;

  return { items, next_cursor: nextCursor };
}

export async function getGameBets(db: Db, gameId: string): Promise<Bet[]> {
  const rows = await db`
    SELECT * FROM bets
    WHERE game_id = ${gameId}
    ORDER BY created_at ASC
  `;
  return rows.map((r: unknown) => rowToBet(r as BetRow));
}

export async function getBetStats(
  db: Db,
  accountId: string
): Promise<BetStats> {
  const rows = await db`
    SELECT
      COALESCE(SUM(amount::numeric), 0) AS total_wagered,
      COALESCE(SUM(payout::numeric), 0) AS total_won,
      COUNT(*) AS total_bets,
      COUNT(*) FILTER (WHERE payout IS NOT NULL AND payout::numeric > 0) AS won_bets
    FROM bets
    WHERE bettor = ${accountId}
  `;
  const r = rows[0] as unknown as Record<string, string>;
  return {
    account_id: accountId,
    total_wagered: r.total_wagered,
    total_won: r.total_won,
    total_bets: Number(r.total_bets),
    won_bets: Number(r.won_bets)
  };
}

export async function getBetLeaderboard(
  db: Db,
  cursor: string | null,
  limit: number
) {
  const actualLimit = clampLimit(limit);
  let rows;
  if (cursor) {
    rows = await db`
      SELECT
        bettor AS account_id,
        SUM(amount::numeric) AS total_wagered,
        COALESCE(SUM(payout::numeric), 0) AS total_won,
        COUNT(*) AS total_bets,
        COUNT(*) FILTER (WHERE payout IS NOT NULL AND payout::numeric > 0) AS won_bets
      FROM bets
      WHERE bettor < ${cursor}
      GROUP BY bettor
      ORDER BY total_won DESC, total_wagered ASC
      LIMIT ${actualLimit + 1}
    `;
  } else {
    rows = await db`
      SELECT
        bettor AS account_id,
        SUM(amount::numeric) AS total_wagered,
        COALESCE(SUM(payout::numeric), 0) AS total_won,
        COUNT(*) AS total_bets,
        COUNT(*) FILTER (WHERE payout IS NOT NULL AND payout::numeric > 0) AS won_bets
      FROM bets
      GROUP BY bettor
      ORDER BY total_won DESC, total_wagered ASC
      LIMIT ${actualLimit + 1}
    `;
  }

  const hasMore = rows.length > actualLimit;
  const items = (hasMore ? rows.slice(0, -1) : rows).map((r: unknown) => {
    const row = r as Record<string, string>;
    return {
      account_id: row.account_id,
      total_wagered: row.total_wagered,
      total_won: row.total_won,
      total_bets: Number(row.total_bets),
      won_bets: Number(row.won_bets)
    } satisfies BetLeaderboardEntry;
  });
  const lastItem = items[items.length - 1] as { account_id: string } | undefined;
  const nextCursor = hasMore && lastItem ? lastItem.account_id : null;

  return { items, next_cursor: nextCursor };
}

export async function getOpenChallenges(
  db: Db,
  cursor: string | null,
  limit: number
) {
  const actualLimit = clampLimit(limit);
  let rows;
  if (cursor) {
    rows = await db`
      SELECT * FROM challenges
      WHERE status = 'pending' AND created_at < ${cursor}
      ORDER BY created_at DESC
      LIMIT ${actualLimit + 1}
    `;
  } else {
    rows = await db`
      SELECT * FROM challenges
      WHERE status = 'pending'
      ORDER BY created_at DESC
      LIMIT ${actualLimit + 1}
    `;
  }

  const hasMore = rows.length > actualLimit;
  const items = (hasMore ? rows.slice(0, -1) : rows).map((r: unknown) =>
    rowToChallenge(r as ChallengeRow)
  );
  const lastItem = items[items.length - 1] as { created_at?: string } | undefined;
  const nextCursor = hasMore && lastItem ? lastItem.created_at ?? null : null;

  return { items, next_cursor: nextCursor };
}

export async function getGlobalBets(
  db: Db,
  status: string | null,
  cursor: string | null,
  limit: number
) {
  const actualLimit = clampLimit(limit);
  let rows;
  if (cursor && status) {
    rows = await db`
      SELECT * FROM bets
      WHERE status = ${status} AND created_at < ${cursor}
      ORDER BY created_at DESC
      LIMIT ${actualLimit + 1}
    `;
  } else if (cursor) {
    rows = await db`
      SELECT * FROM bets
      WHERE created_at < ${cursor}
      ORDER BY created_at DESC
      LIMIT ${actualLimit + 1}
    `;
  } else if (status) {
    rows = await db`
      SELECT * FROM bets
      WHERE status = ${status}
      ORDER BY created_at DESC
      LIMIT ${actualLimit + 1}
    `;
  } else {
    rows = await db`
      SELECT * FROM bets
      ORDER BY created_at DESC
      LIMIT ${actualLimit + 1}
    `;
  }

  const hasMore = rows.length > actualLimit;
  const items = (hasMore ? rows.slice(0, -1) : rows).map((r: unknown) =>
    rowToBet(r as BetRow)
  );
  const lastItem = items[items.length - 1] as { created_at?: string } | undefined;
  const nextCursor = hasMore && lastItem ? lastItem.created_at ?? null : null;

  return { items, next_cursor: nextCursor };
}

export async function deleteExpiredSubscriptions(
  db: Db,
  endpoints: string[]
): Promise<void> {
  if (endpoints.length === 0) return;
  await db`DELETE FROM push_subscriptions WHERE endpoint = ANY(${endpoints})`;
}
