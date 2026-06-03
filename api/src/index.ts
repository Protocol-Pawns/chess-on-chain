import { OpenAPIHono } from '@hono/zod-openapi';
import { apiReference } from '@scalar/hono-api-reference';
import { cors } from 'hono/cors';
import { poweredBy } from 'hono/powered-by';

import {
  getAccount,
  getDb,
  getGame,
  getInfo,
  getRecentFinishedGames,
  getRecentNewGames,
  queryGames
} from './db';
import {
  getAccountRoute,
  getGameRoute,
  getInfoRoute,
  getRecentFinishedGamesRoute,
  getRecentNewGamesRoute,
  queryGamesRoute
} from './routes';
import type { AppEnv } from './types';

const app = new OpenAPIHono<AppEnv>();

app.use('*', poweredBy());
app.use('*', cors());

app.use('*', async (c, next) => {
  const env = c.env;
  const connectionString = env.HYPERDRIVE
    ? env.HYPERDRIVE.connectionString
    : env.DATABASE_URL!;
  c.set('DB', getDb(connectionString));
  await next();
});

app.doc('/doc', {
  openapi: '3.1.0',
  info: {
    title: 'Protocol Pawns API',
    version: '2.0.0',
    description: 'API for the Protocol Pawns on-chain chess game'
  }
});

app.get('/', apiReference({ spec: { url: '/doc' } }));

const MAX_RECENT_LIMIT = 25;

app.openapi(getInfoRoute, async c => {
  const db = c.get('DB');
  const result = await getInfo(db);
  return c.json(result);
});

app.openapi(getGameRoute, async c => {
  const gameIdJson = decodeURIComponent(c.req.param('game_id'));
  const db = c.get('DB');
  const game = await getGame(db, gameIdJson);
  if (!game) {
    return c.jsonT(null, 404);
  }
  return c.jsonT(game, 200);
});

app.openapi(queryGamesRoute, async c => {
  const db = c.get('DB');
  const { gameIds, includeMoves } = c.req.valid('json');
  const gameIdStrings = gameIds.map(id => JSON.stringify(id));
  const result = await queryGames(db, gameIdStrings, includeMoves ?? false);
  return c.jsonT(result, 200);
});

app.openapi(getRecentNewGamesRoute, async c => {
  const { limit, include_moves } = c.req.valid('query');
  const actualLimit = limit ? Number(limit) : MAX_RECENT_LIMIT;
  const includeMoves = Number(include_moves) === 1;
  const db = c.get('DB');
  const result = await getRecentNewGames(db, actualLimit, includeMoves);
  return c.jsonT(result, 200);
});

app.openapi(getRecentFinishedGamesRoute, async c => {
  const { limit, include_moves } = c.req.valid('query');
  const actualLimit = limit ? Number(limit) : MAX_RECENT_LIMIT;
  const includeMoves = Number(include_moves) === 1;
  const db = c.get('DB');
  const result = await getRecentFinishedGames(db, actualLimit, includeMoves);
  return c.jsonT(result, 200);
});

app.openapi(getAccountRoute, async c => {
  const accountId = c.req.param('account_id');
  const db = c.get('DB');
  const account = await getAccount(db, accountId);
  return c.jsonT(account, 200);
});

app.get('/test', c => {
  return c.json({ test: '', test1: 0 });
});

app.notFound(() => {
  return new Response(null, { status: 404 });
});

export default app;

export type AppType = typeof app;
