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
  GameOverview,
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
  outcome: GameOutcome | null;
  resigner: Color | null;
  status: string;
  created_at: string | number;
  finished_at: string | number | null;
}

interface GameMoveRow {
  move_number: number;
  color: string;
  move_notation: string;
  fen: string;
  outcome: GameOutcome | null;
}

interface ChallengeRow {
  id: number;
  challenge_id: string;
  challenger: string;
  challenged: string;
  wager_token: string | null;
  wager_amount: string | null;
  status: string;
  game_id: string | null;
  created_at: string | number;
  resolved_at: string | number | null;
}

function parsePlayer(type: string, value: string | null): Player {
  if (type === 'Ai')
    return { type: 'Ai', value: value as 'Easy' | 'Medium' | 'Hard' };
  return { type: 'Human', value: value! };
}

function parseJson<T>(value: T): T {
  return typeof value === 'string' ? JSON.parse(value) : value;
}

function nsToIso(ts: string | number | null): string | null {
  if (ts == null) return null;
  const n = Number(ts);
  return new Date(n > 1e15 ? n / 1_000_000 : n).toISOString();
}

function rowToGame(row: GameRow): Game {
  const board = parseJson<string[]>(row.board);
  const outcome = parseJson<GameOutcome | null>(row.outcome);
  return {
    game_id: JSON.parse(row.game_id),
    white: parsePlayer(row.white_type, row.white_value),
    black: parsePlayer(row.black_type, row.black_value),
    board,
    fen: row.fen,
    status: row.status as Game['status'],
    outcome,
    resigner: row.resigner,
    created_at: nsToIso(row.created_at) ?? '',
    finished_at: nsToIso(row.finished_at)
  };
}

function rowToGameOverview(row: GameRow) {
  return rowToGame(row);
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
    id: row.challenge_id,
    challenger: row.challenger,
    challenged: row.challenged,
    wager_token: row.wager_token,
    wager_amount: row.wager_amount,
    status: row.status as Challenge['status'],
    game_id: row.game_id,
    created_at: nsToIso(row.created_at) ?? '',
    resolved_at: nsToIso(row.resolved_at)
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

export async function queryGames(db: Db, gameIds: string[]) {
  if (gameIds.length === 0) return [];
  const rows = await db`SELECT * FROM games WHERE game_id = ANY(${gameIds})`;
  return rows.map((r: unknown) => rowToGameOverview(r as GameRow));
}

export interface OffsetPaginatedResult<T> {
  items: T[];
  next_cursor: string | null;
  total_count?: number;
  total_pages?: number;
  page?: number;
  per_page?: number;
}

export async function getGames(
  db: Db,
  status: 'active' | 'finished',
  cursor: string | null,
  limit: number,
  page?: number,
  excludeAi?: boolean
): Promise<OffsetPaginatedResult<GameOverview>> {
  const actualLimit = clampLimit(limit);
  const statusFilter = status === 'active' ? 'in_progress' : 'finished';
  const orderBy = status === 'active' ? 'created_at' : 'finished_at';

  const aiCondition = excludeAi
    ? db`AND white_type != 'Ai' AND black_type != 'Ai'`
    : db``;

  if (page != null && page > 0) {
    const offset = (page - 1) * actualLimit;

    const countRows = await db`
      SELECT COUNT(*) AS total FROM games
      WHERE status = ${statusFilter} ${aiCondition}
    `;
    const totalCount = Number(
      (countRows[0] as unknown as Record<string, string>).total
    );
    const totalPages = Math.max(1, Math.ceil(totalCount / actualLimit));

    const rows = await db`
      SELECT * FROM games
      WHERE status = ${statusFilter} ${aiCondition}
      ORDER BY ${db(orderBy)} DESC
      LIMIT ${actualLimit} OFFSET ${offset}
    `;

    const items = rows.map((r: unknown) => rowToGameOverview(r as GameRow));
    const hasMore = page < totalPages;
    const lastRaw = rows[rows.length - 1] as GameRow | undefined;
    const nextCursor =
      hasMore && lastRaw
        ? String(status === 'active' ? lastRaw.created_at : lastRaw.finished_at)
        : null;

    return {
      items,
      next_cursor: nextCursor,
      total_count: totalCount,
      total_pages: totalPages,
      page,
      per_page: actualLimit
    };
  }

  let rows;
  if (cursor) {
    rows = await db`
      SELECT * FROM games
      WHERE status = ${statusFilter} ${aiCondition} AND ${db(orderBy)} < ${cursor}
      ORDER BY ${db(orderBy)} DESC
      LIMIT ${actualLimit + 1}
    `;
  } else {
    rows = await db`
      SELECT * FROM games
      WHERE status = ${statusFilter} ${aiCondition}
      ORDER BY ${db(orderBy)} DESC
      LIMIT ${actualLimit + 1}
    `;
  }

  const hasMore = rows.length > actualLimit;
  const rawItems = hasMore ? rows.slice(0, -1) : rows;
  const items = rawItems.map((r: unknown) => rowToGameOverview(r as GameRow));
  const lastRaw = rawItems[rawItems.length - 1] as GameRow | undefined;
  const nextCursor =
    hasMore && lastRaw
      ? String(status === 'active' ? lastRaw.created_at : lastRaw.finished_at)
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

type OutcomeRow = {
  outcome: Record<string, string> | string | null;
  status: string;
  white_type: string;
  black_type: string | null;
  white_value: string;
  black_value: string | null;
};

function parseOutcome(
  raw: Record<string, string> | string | null
): Record<string, string> | null {
  if (!raw) return null;
  if (typeof raw === 'string') {
    try {
      return JSON.parse(raw);
    } catch {
      return null;
    }
  }
  return raw;
}

function tallyGame(
  accountId: string,
  row: OutcomeRow
): { win: boolean; loss: boolean; draw: boolean } | null {
  if (row.status !== 'finished') return null;
  if (row.white_type !== 'Human' || row.black_type !== 'Human') return null;
  const outcome = parseOutcome(row.outcome);
  if (!outcome) return { win: false, loss: false, draw: false };

  if (outcome.result === 'Stalemate')
    return { win: false, loss: false, draw: true };

  const wonAsWhite =
    outcome.result === 'Victory' &&
    outcome.color === 'White' &&
    row.white_value === accountId;
  const wonAsBlack =
    outcome.result === 'Victory' &&
    outcome.color === 'Black' &&
    row.black_value === accountId;

  if (wonAsWhite || wonAsBlack) return { win: true, loss: false, draw: false };
  return { win: false, loss: true, draw: false };
}

export async function getAccountStats(
  db: Db,
  accountId: string
): Promise<AccountStats> {
  const rows = await db`
    SELECT outcome, status, white_type, black_type, white_value, black_value
    FROM games
    WHERE white_value = ${accountId} OR black_value = ${accountId}
  `;

  let wins = 0;
  let losses = 0;
  let draws = 0;
  let total_games = 0;

  for (const row of rows as unknown as OutcomeRow[]) {
    const result = tallyGame(accountId, row);
    if (!result) continue;
    total_games++;
    if (result.win) wins++;
    if (result.loss) losses++;
    if (result.draw) draws++;
  }

  return { account_id: accountId, wins, losses, draws, total_games };
}

export async function getAccountStatsBatch(
  db: Db,
  accountIds: string[]
): Promise<AccountStats[]> {
  if (accountIds.length === 0) return [];

  const idSet = new Set(accountIds);
  const rows = await db`
    SELECT outcome, status, white_type, black_type, white_value, black_value
    FROM games
    WHERE white_value = ANY(${accountIds}::text[]) OR black_value = ANY(${accountIds}::text[])
  `;

  const statsMap = new Map<
    string,
    { wins: number; losses: number; draws: number; total_games: number }
  >();
  for (const id of accountIds) {
    statsMap.set(id, { wins: 0, losses: 0, draws: 0, total_games: 0 });
  }

  for (const row of rows as unknown as OutcomeRow[]) {
    for (const accountId of [row.white_value, row.black_value]) {
      if (!accountId || !idSet.has(accountId)) continue;
      const result = tallyGame(accountId, row);
      if (!result) continue;
      const stats = statsMap.get(accountId)!;
      stats.total_games++;
      if (result.win) stats.wins++;
      if (result.loss) stats.losses++;
      if (result.draw) stats.draws++;
    }
  }

  return accountIds.map(id => ({
    account_id: id,
    ...statsMap.get(id)!
  }));
}

export async function searchAccounts(
  db: Db,
  query: string
): Promise<AccountStats[]> {
  const pattern = '%' + query.toLowerCase() + '%';
  const rows = await db`
    SELECT DISTINCT account_id FROM (
      SELECT white_value AS account_id FROM games WHERE LOWER(white_value) LIKE ${pattern}
      UNION
      SELECT black_value AS account_id FROM games WHERE LOWER(black_value) LIKE ${pattern}
      UNION
      SELECT challenger AS account_id FROM challenges WHERE LOWER(challenger) LIKE ${pattern}
      UNION
      SELECT challenged AS account_id FROM challenges WHERE LOWER(challenged) LIKE ${pattern}
      UNION
      SELECT account_id FROM account_finished_games WHERE LOWER(account_id) LIKE ${pattern}
    ) matching
    LIMIT 20
  `;
  const accountIds = rows.map(
    r => (r as unknown as { account_id: string }).account_id
  );
  if (accountIds.length === 0) return [];
  return getAccountStatsBatch(db, accountIds);
}

export async function getChallenges(
  db: Db,
  accountId: string,
  page?: number,
  perPage?: number,
  excludeRejected?: boolean
): Promise<Challenge[] | OffsetPaginatedResult<Challenge>> {
  const notRejected = excludeRejected ? db`AND status != 'rejected'` : db``;

  if (page != null && page > 0) {
    const limit = clampLimit(perPage, 100, 25);
    const offset = (page - 1) * limit;

    const countRows = await db`
      SELECT COUNT(*) AS total FROM challenges
      WHERE (challenger = ${accountId} OR challenged = ${accountId}) ${notRejected}
    `;
    const totalCount = Number(
      (countRows[0] as unknown as Record<string, string>).total
    );
    const totalPages = Math.max(1, Math.ceil(totalCount / limit));

    const rows = await db`
      SELECT * FROM challenges
      WHERE (challenger = ${accountId} OR challenged = ${accountId}) ${notRejected}
      ORDER BY created_at DESC
      LIMIT ${limit} OFFSET ${offset}
    `;

    return {
      items: rows.map((r: unknown) => rowToChallenge(r as ChallengeRow)),
      next_cursor: page < totalPages ? String(page + 1) : null,
      total_count: totalCount,
      total_pages: totalPages,
      page,
      per_page: limit
    };
  }

  const rows = await db`
    SELECT * FROM challenges
    WHERE (challenger = ${accountId} OR challenged = ${accountId}) ${notRejected}
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
  id: number;
  bet_key: string;
  bettor: string;
  player_0: string;
  player_1: string;
  game_id: string | null;
  token_id: string;
  amount: string;
  winner: string;
  status: string;
  payout: string | null;
  created_at: string | number;
  locked_at: string | number | null;
  resolved_at: string | number | null;
}

function rowToBet(row: BetRow): Bet {
  return {
    id: row.bet_key,
    bettor: row.bettor,
    player_0: row.player_0,
    player_1: row.player_1,
    game_id: row.game_id,
    token_id: row.token_id,
    amount: row.amount,
    winner: row.winner,
    status: row.status as Bet['status'],
    payout: row.payout,
    created_at: nsToIso(row.created_at) ?? '',
    locked_at: nsToIso(row.locked_at),
    resolved_at: nsToIso(row.resolved_at)
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
  const rawItems = hasMore ? rows.slice(0, -1) : rows;
  const items = rawItems.map((r: unknown) => rowToBet(r as BetRow));
  const lastRaw = rawItems[rawItems.length - 1] as BetRow | undefined;
  const nextCursor = hasMore && lastRaw ? String(lastRaw.created_at) : null;

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
  const lastItem = items[items.length - 1] as
    | { account_id: string }
    | undefined;
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
  const rawItems = hasMore ? rows.slice(0, -1) : rows;
  const items = rawItems.map((r: unknown) => rowToChallenge(r as ChallengeRow));
  const lastRaw = rawItems[rawItems.length - 1] as ChallengeRow | undefined;
  const nextCursor = hasMore && lastRaw ? String(lastRaw.created_at) : null;

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
  const rawItems = hasMore ? rows.slice(0, -1) : rows;
  const items = rawItems.map((r: unknown) => rowToBet(r as BetRow));
  const lastRaw = rawItems[rawItems.length - 1] as BetRow | undefined;
  const nextCursor = hasMore && lastRaw ? String(lastRaw.created_at) : null;

  return { items, next_cursor: nextCursor };
}

export async function deleteExpiredSubscriptions(
  db: Db,
  endpoints: string[]
): Promise<void> {
  if (endpoints.length === 0) return;
  await db`DELETE FROM push_subscriptions WHERE endpoint = ANY(${endpoints})`;
}
