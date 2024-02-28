import { Hono } from 'hono';

import { Account } from './events';

export const accounts = new Hono().get('/account/:account_id', async c => {
  const accountId = c.req.param('account_id');
  const addr = c.env.GAMES.idFromName('');
  const obj = c.env.GAMES.get(addr);
  const res = await obj.fetch(
    `${new URL(c.req.url).origin}/account/${accountId}`
  );
  const info = await res.json<Account>();
  return c.json(info);
});
