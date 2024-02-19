import { Hono } from 'hono';
import { cors } from 'hono/cors';
import { poweredBy } from 'hono/powered-by';
import { match } from 'ts-pattern';

import { games } from './games';
import { Env } from './global';

const app = new Hono<{ Bindings: Env }>();

app.use('*', poweredBy());
app.use('*', cors());

app.route('/games', games);

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
