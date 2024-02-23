import { Hono } from 'hono';

import { Account } from './events';
import type { Env } from './types';

export const accounts = new Hono<{ Bindings: Env }>().get(
  '/account/:account_id',
  async c => {
    const accountId = c.req.param('account_id');
    const addr = c.env.GAMES.idFromName('');
    const obj = c.env.GAMES.get(addr);
    const res = await obj.fetch(
      `${new URL(c.req.url).origin}/account/${accountId}`
    );
    if (!res.ok) {
      return new Response(res.body, res);
    }
    const info = await res.json<Account>();
    return c.json(info);
  }
);
