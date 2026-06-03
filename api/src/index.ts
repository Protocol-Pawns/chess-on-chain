import { OpenAPIHono } from '@hono/zod-openapi';
import { apiReference } from '@scalar/hono-api-reference';
import { cors } from 'hono/cors';
import { poweredBy } from 'hono/powered-by';

import {
  getAccount,
  getAccountStats,
  getActiveGame,
  getChallenges,
  getDb,
  getGame,
  getGameMoves,
  getGames,
  getGlobalStats,
  getInfo,
  getLeaderboard,
  queryGames
} from './db';
import {
  getAccountRoute,
  getAccountStatsRoute,
  getActiveGameRoute,
  getChallengesRoute,
  getGameMovesRoute,
  getGameRoute,
  getGamesRoute,
  getGlobalStatsRoute,
  getInfoRoute,
  getLeaderboardRoute,
  queryGamesRoute
} from './routes';
import type { AppEnv } from './types';

const app = new OpenAPIHono<AppEnv>();

app.use('*', poweredBy());
app.use('*', cors());

app.use('*', async (c, next) => {
  if (!c.get('DB')) {
    const env = c.env;
    const connectionString = env.HYPERDRIVE
      ? env.HYPERDRIVE.connectionString
      : env.DATABASE_URL!;
    c.set('DB', getDb(connectionString));
  }
  await next();
});

app.onError((err, c) => {
  console.error('Unhandled error:', err);
  return c.json({ error: 'Internal Server Error', message: err.message }, 500);
});

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

app.openapi(queryGamesRoute, async c => {
  const db = c.get('DB');
  const { gameIds, includeMoves } = c.req.valid('json');
  const gameIdStrings = gameIds.map((id: unknown) => JSON.stringify(id));
  const result = await queryGames(db, gameIdStrings, includeMoves ?? false);
  return c.json(result, 200);
});

app.openapi(getGamesRoute, async c => {
  const { status, cursor, limit, include_moves } = c.req.valid('query');
  const includeMoves = include_moves === '1' || include_moves === 'true';
  const db = c.get('DB');
  const result = await getGames(
    db,
    status,
    cursor ?? null,
    Number(limit) || 25,
    includeMoves
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

app.openapi(getChallengesRoute, async c => {
  const accountId = c.req.param('account_id');
  const db = c.get('DB');
  const challenges = await getChallenges(db, accountId);
  return c.json(challenges, 200);
});

app.openapi(getLeaderboardRoute, async c => {
  const { cursor, limit } = c.req.valid('query');
  const db = c.get('DB');
  const result = await getLeaderboard(db, cursor ?? null, Number(limit) || 25);
  return c.json(result, 200);
});

app.notFound(() => {
  return new Response(null, { status: 404 });
});

export default app;

export type AppType = typeof app;
