import { OpenAPIHono } from '@hono/zod-openapi';
import { apiReference } from '@scalar/hono-api-reference';
import { cors } from 'hono/cors';
import { poweredBy } from 'hono/powered-by';

import { getAccountInfo } from './account';
import {
  addPushSubscription,
  getAccount,
  getAccountStats,
  getAccountStatsBatch,
  getActiveGame,
  getBetLeaderboard,
  getBets,
  getBetStats,
  getChallenges,
  getDb,
  getGame,
  getGameBets,
  getGameMoves,
  getGames,
  getGlobalBets,
  getGlobalStats,
  getInfo,
  getOpenChallenges,
  queryGames,
  removePushSubscription,
  searchAccounts
} from './db';
import type { Db } from './db';
import { registerSSERoutes } from './events-stream';
import {
  fetchEloRatingsByIds,
  getEloRankingPage,
  getPppRankingPage,
  fetchAndCacheLeaderboard
} from './leaderboard';
import {
  processNotifications,
  processQuestCooldownNotifications
} from './notify';
import { importVapidKey } from './push';
import { renderGameCard, renderProfileCard } from './render-card';
export { SSEHub } from './do-sse';
import {
  getAccountPreviewRoute,
  getAccountRoute,
  getAccountStatsRoute,
  batchAccountStatsRoute,
  getActiveGameRoute,
  getBetsRoute,
  getBetLeaderboardRoute,
  getBetStatsRoute,
  getChallengesRoute,
  getGameBetsRoute,
  getGameMovesRoute,
  getGamePreviewRoute,
  getGameRoute,
  getGamesRoute,
  queryGamesRoute,
  getGlobalBetsRoute,
  getGlobalStatsRoute,
  getInfoRoute,
  getLeaderboardEloRoute,
  getLeaderboardPppRoute,
  getOpenChallengesRoute,
  getVapidPublicKeyRoute,
  searchAccountsRoute,
  subscribePushRoute,
  unsubscribePushRoute
} from './routes';
import type { AppEnv } from './types';

const app = new OpenAPIHono<AppEnv>();

function initDb(env: AppEnv['Bindings']): Db {
  return getDb(env.DATABASE_URL);
}

app.use('*', poweredBy());
app.use('*', cors());

app.use('*', async (c, next) => {
  try {
    c.set('DB', initDb(c.env));
  } catch {
    return c.json({ error: 'Database not configured' }, 500);
  }
  await next();
});

app.onError((err, c) => {
  console.error('Unhandled error:', err);
  return c.json({ error: 'Internal Server Error', message: err.message }, 500);
});

registerSSERoutes(app);

app.doc('/doc', {
  openapi: '3.1.0',
  info: {
    title: 'Protocol Pawns API',
    version: '2.0.0',
    description: 'API for the Protocol Pawns on-chain chess game'
  }
});

app.get('/', apiReference({ url: '/doc' } as never));

app.openapi(getInfoRoute, async c => {
  const db = c.get('DB');
  const result = await getInfo(db);
  return c.json(result, 200);
});

app.openapi(getGlobalStatsRoute, async c => {
  const db = c.get('DB');
  const result = await getGlobalStats(db);
  return c.json(result, 200);
});

app.openapi(getGameRoute, async c => {
  const gameIdJson = decodeURIComponent(c.req.param('game_id'));
  const db = c.get('DB');
  const game = await getGame(db, gameIdJson);
  if (!game) return c.json({ error: 'Not found' } as const, 404);
  return c.json(game, 200);
});

app.openapi(getGameMovesRoute, async c => {
  const gameIdJson = decodeURIComponent(c.req.param('game_id'));
  const db = c.get('DB');
  const moves = await getGameMoves(db, gameIdJson);
  return c.json(moves, 200);
});

app.openapi(getGamePreviewRoute, async c => {
  const gameIdJson = decodeURIComponent(c.req.param('game_id'));
  const db = c.get('DB');
  const [game, moves] = await Promise.all([
    getGame(db, gameIdJson),
    getGameMoves(db, gameIdJson)
  ]);
  if (!game) return c.json({ error: 'Not found' } as const, 404);

  const lastMove = moves[moves.length - 1];
  const lastMoveNotation = lastMove
    ? parseMoveNotation(lastMove.move_notation, lastMove.color)
    : undefined;

  const whiteName =
    game.white.type === 'Human'
      ? truncateAddr(game.white.value)
      : `AI (${game.white.value})`;
  const blackName = game.black
    ? game.black.type === 'Human'
      ? truncateAddr(game.black.value)
      : `AI (${game.black.value})`
    : 'Unknown';

  let whiteElo: number | null = null;
  let blackElo: number | null = null;
  try {
    const eloIds: string[] = [];
    if (game.white.type === 'Human') eloIds.push(game.white.value);
    if (game.black?.type === 'Human' && game.black.value)
      eloIds.push(game.black.value);
    if (eloIds.length > 0) {
      const eloPairs = await fetchEloRatingsByIds(
        c.env.RPC_URL,
        c.env.CONTRACT_ID,
        eloIds
      );
      const eloMap = new Map<string, number>(eloPairs);
      if (game.white.type === 'Human')
        whiteElo = eloMap.get(game.white.value) ?? null;
      if (game.black?.type === 'Human' && game.black.value)
        blackElo = eloMap.get(game.black.value) ?? null;
    }
  } catch {
    // ELOs are optional for the preview
  }

  const png = await renderGameCard({
    board: game.board,
    fen: game.fen ?? undefined,
    whiteName,
    blackName,
    whiteElo,
    blackElo,
    result: resultText(game),
    lastMove: lastMoveNotation
  });

  return c.body(new Uint8Array(png), 200, {
    'Content-Type': 'image/png',
    'Cache-Control': 'public, max-age=300'
  });
});

app.openapi(getAccountPreviewRoute, async c => {
  const accountId = decodeURIComponent(c.req.param('account_id'));
  const db = c.get('DB');

  const [stats, accountInfo, eloPairs] = await Promise.all([
    getAccountStats(db, accountId),
    getAccountInfo(c.env.RPC_URL, c.env.CONTRACT_ID, accountId).catch(
      () => null
    ),
    fetchEloRatingsByIds(c.env.RPC_URL, c.env.CONTRACT_ID, [accountId]).catch(
      () => [] as [string, number][]
    )
  ]);

  const eloMap = new Map<string, number>(eloPairs);
  const elo = eloMap.get(accountId) ?? accountInfo?.elo ?? null;

  const winRate =
    stats.total_games > 0
      ? Math.round((stats.wins / stats.total_games) * 1000) / 10
      : 0;

  const png = await renderProfileCard({
    accountId: truncateAddr(accountId, 24),
    elo,
    points: formatPoints(accountInfo?.points),
    wins: stats.wins,
    losses: stats.losses,
    draws: stats.draws,
    totalGames: stats.total_games,
    winRate,
    extras: accountInfo
      ? {
          winStreak: accountInfo.win_streak,
          maxWinStreak: accountInfo.max_win_streak,
          betsPlaced: accountInfo.bets_placed,
          betsWon: accountInfo.bets_won,
          wagersPlayed: accountInfo.wagers_played,
          wagerWins: accountInfo.wager_wins,
          challengesSent: accountInfo.challenges_sent
        }
      : undefined
  });

  return c.body(new Uint8Array(png), 200, {
    'Content-Type': 'image/png',
    'Cache-Control': 'public, max-age=300'
  });
});

function parseMoveNotation(
  notation: string,
  color?: string
): { from: string; to: string } | null {
  if (/^O-O-O$/i.test(notation) || /^0-0-0$/.test(notation)) {
    const rank = color === 'Black' ? '8' : '1';
    return { from: `e${rank}`, to: `c${rank}` };
  }
  if (/^O-O$/i.test(notation) || /^0-0$/.test(notation)) {
    const rank = color === 'Black' ? '8' : '1';
    return { from: `e${rank}`, to: `g${rank}` };
  }
  const parts = notation.split(' to ');
  if (parts.length >= 2) {
    const from = parts[0].trim();
    const to = parts[1].trim().split(/\s/)[0];
    if (/^[a-h][1-8]$/.test(from) && /^[a-h][1-8]$/.test(to)) {
      return { from, to };
    }
  }
  return null;
}

function truncateAddr(id: string, max = 20): string {
  if (id.length <= max) return id;
  return `${id.slice(0, 8)}...${id.slice(-4)}`;
}

function formatPoints(raw: string | undefined | null): string | null {
  if (!raw || raw === '0') return null;
  try {
    const val = BigInt(raw);
    const whole = val / BigInt(1000000);
    const frac = val % BigInt(1000000);
    const fracStr = frac.toString().padStart(6, '0').slice(0, 2);
    return `${whole}.${fracStr}`;
  } catch {
    return null;
  }
}

function resultText(game: Awaited<ReturnType<typeof getGame>>): string {
  if (!game?.outcome) {
    if (game?.status === 'cancelled') return 'Game cancelled';
    return game?.status === 'in_progress' ? 'In progress' : 'Waiting';
  }
  if (game.outcome.result === 'Stalemate') return 'Draw — Stalemate';
  if (game.resigner) return `${game.outcome.color} wins by resignation!`;
  return `${game.outcome.color} wins by checkmate!`;
}

app.openapi(queryGamesRoute, async c => {
  const db = c.get('DB');
  const { gameIds } = c.req.valid('json');
  const gameIdStrings = gameIds.map((id: unknown) => JSON.stringify(id));
  const result = await queryGames(db, gameIdStrings);
  return c.json(result, 200);
});

app.openapi(getGamesRoute, async c => {
  const { status, cursor, limit, page, exclude_ai } = c.req.valid('query');
  const db = c.get('DB');
  const result = await getGames(
    db,
    status,
    cursor ?? null,
    Number(limit) || 25,
    page ? Number(page) : undefined,
    exclude_ai
  );
  return c.json(result, 200);
});

app.openapi(getActiveGameRoute, async c => {
  const accountId = c.req.param('account_id');
  const db = c.get('DB');
  const game = await getActiveGame(db, accountId);
  if (!game) return c.json({ error: 'Not found' } as const, 404);
  return c.json(game, 200);
});

app.openapi(getAccountRoute, async c => {
  const accountId = c.req.param('account_id');
  const db = c.get('DB');
  const account = await getAccount(db, accountId);
  return c.json(account, 200);
});

app.openapi(getAccountStatsRoute, async c => {
  const accountId = c.req.param('account_id');
  const db = c.get('DB');
  const stats = await getAccountStats(db, accountId);
  return c.json(stats, 200);
});

app.openapi(batchAccountStatsRoute, async c => {
  const { account_ids } = c.req.valid('json');
  const db = c.get('DB');
  const stats = await getAccountStatsBatch(db, account_ids);
  return c.json(stats, 200);
});

app.openapi(getChallengesRoute, async c => {
  const accountId = c.req.param('account_id');
  const { page, per_page, exclude_rejected } = c.req.valid('query');
  const db = c.get('DB');
  const challenges = await getChallenges(
    db,
    accountId,
    page ? Number(page) : undefined,
    per_page ? Number(per_page) : undefined,
    exclude_rejected
  );
  return c.json(challenges, 200);
});

app.openapi(getLeaderboardEloRoute, async c => {
  const { page, per_page, dir } = c.req.valid('query');
  const db = c.get('DB');
  const result = await getEloRankingPage(
    c.env.LEADERBOARD_CACHE,
    c.env.RPC_URL,
    c.env.CONTRACT_ID,
    db,
    Number(page) || 1,
    Number(per_page) || 25,
    dir
  );
  return c.json(result, 200);
});

app.openapi(getLeaderboardPppRoute, async c => {
  const { page, per_page } = c.req.valid('query');
  const db = c.get('DB');
  const result = await getPppRankingPage(
    c.env.LEADERBOARD_CACHE,
    c.env.RPC_URL,
    c.env.CONTRACT_ID,
    db,
    Number(page) || 1,
    Number(per_page) || 25
  );
  return c.json(result, 200);
});

app.openapi(getVapidPublicKeyRoute, async c => {
  return c.json({ publicKey: c.env.VAPID_PUBLIC_KEY }, 200);
});

app.openapi(subscribePushRoute, async c => {
  const accountId = c.req.param('account_id');
  const { endpoint, keys } = c.req.valid('json');
  const db = c.get('DB');
  await addPushSubscription(db, accountId, endpoint, keys.p256dh, keys.auth);
  return c.json({ ok: true }, 200);
});

app.openapi(unsubscribePushRoute, async c => {
  const accountId = c.req.param('account_id');
  const { endpoint } = c.req.valid('json');
  const db = c.get('DB');
  const ok = await removePushSubscription(db, accountId, endpoint);
  return c.json({ ok }, 200);
});

app.openapi(searchAccountsRoute, async c => {
  const { query } = c.req.valid('json');
  const db = c.get('DB');
  const stats = await searchAccounts(db, query);
  const accountIds = stats.map(s => s.account_id);
  const eloPairs = await fetchEloRatingsByIds(
    c.env.RPC_URL,
    c.env.CONTRACT_ID,
    accountIds
  );
  const eloMap = new Map<string, number>(eloPairs);
  const results = stats.map(s => ({
    ...s,
    elo: eloMap.get(s.account_id) ?? null
  }));
  return c.json(results, 200);
});

app.openapi(getBetsRoute, async c => {
  const accountId = c.req.param('account_id');
  const { status, cursor, limit } = c.req.valid('query');
  const db = c.get('DB');
  const result = await getBets(
    db,
    accountId,
    status ?? null,
    cursor ?? null,
    Number(limit) || 25
  );
  return c.json(result, 200);
});

app.openapi(getGameBetsRoute, async c => {
  const gameId = decodeURIComponent(c.req.param('game_id'));
  const db = c.get('DB');
  const bets = await getGameBets(db, gameId);
  return c.json(bets, 200);
});

app.openapi(getBetStatsRoute, async c => {
  const accountId = c.req.param('account_id');
  const db = c.get('DB');
  const stats = await getBetStats(db, accountId);
  return c.json(stats, 200);
});

app.openapi(getBetLeaderboardRoute, async c => {
  const { cursor, limit } = c.req.valid('query');
  const db = c.get('DB');
  const result = await getBetLeaderboard(
    db,
    cursor ?? null,
    Number(limit) || 25
  );
  return c.json(result, 200);
});

app.openapi(getOpenChallengesRoute, async c => {
  const { cursor, limit } = c.req.valid('query');
  const db = c.get('DB');
  const result = await getOpenChallenges(
    db,
    cursor ?? null,
    Number(limit) || 25
  );
  return c.json(result, 200);
});

app.openapi(getGlobalBetsRoute, async c => {
  const { status, cursor, limit } = c.req.valid('query');
  const db = c.get('DB');
  const result = await getGlobalBets(
    db,
    status ?? null,
    cursor ?? null,
    Number(limit) || 25
  );
  return c.json(result, 200);
});

app.notFound(() => {
  return new Response(null, { status: 404 });
});

export default {
  fetch: app.fetch,
  scheduled: async (_event: ScheduledEvent, env: AppEnv['Bindings']) => {
    const promises: Promise<void>[] = [];

    promises.push(
      (async () => {
        try {
          const db = initDb(env);
          const vapidPrivateKey = importVapidKey(env.VAPID_PRIVATE_KEY);
          const processed = await processNotifications(
            db,
            vapidPrivateKey,
            env.VAPID_PUBLIC_KEY,
            env.VAPID_SUBJECT
          );
          console.log(`Processed ${processed} notifications`);
        } catch (err) {
          console.error('Notification cron error:', err);
        }
      })()
    );

    promises.push(
      (async () => {
        try {
          const db = initDb(env);
          const vapidPrivateKey = importVapidKey(env.VAPID_PRIVATE_KEY);
          const processed = await processQuestCooldownNotifications(
            db,
            vapidPrivateKey,
            env.VAPID_PUBLIC_KEY,
            env.VAPID_SUBJECT,
            env.RPC_URL,
            env.CONTRACT_ID
          );
          console.log(`Processed ${processed} quest cooldown notifications`);
        } catch (err) {
          console.error('Quest cooldown notification cron error:', err);
        }
      })()
    );

    promises.push(
      (async () => {
        try {
          await fetchAndCacheLeaderboard(
            env.LEADERBOARD_CACHE,
            env.RPC_URL,
            env.CONTRACT_ID
          );
          console.log('Leaderboard cache refreshed');
        } catch (err) {
          console.error('Leaderboard cache error:', err);
        }
      })()
    );

    await Promise.all(promises);
  }
};

export type AppType = typeof app;
