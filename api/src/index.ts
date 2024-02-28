import { Hono } from 'hono';
import { cors } from 'hono/cors';
import { poweredBy } from 'hono/powered-by';
import { match } from 'ts-pattern';

import { accounts } from './acounts';
import { batch } from './batch';
import { games } from './games';
import { info } from './info';

const app = new Hono();

app.use('*', poweredBy());
app.use('*', cors());

const infoRoute = app.route('/info', info);
const gamesRoute = app.route('/games', games);
const accountsRoute = app.route('/accounts', accounts);
app.route('/batch', batch);

const testRoute = app.get('/test', c => {
  return c.json({
    test: '',
    test1: 0
  });
});

app.onError(
  err =>
    new Response(null, {
      status: match(err.message)
        .with('Unauthorized', () => 401 as const)
        .with('Bad Request', () => 400 as const)
        .otherwise(() => {
          throw err;
        })
    })
);

app.notFound(() => {
  return new Response(null, { status: 404 });
});

export default app;

export { Games } from './games';
export { Info } from './info';

export type AppType =
  | typeof infoRoute
  | typeof gamesRoute
  | typeof accountsRoute
  | typeof testRoute;
