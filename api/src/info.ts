import { Hono } from 'hono';

import type { Env } from './types';

export type InfoResult = {
  lastBlockHeight: number;
};

export const info = new Hono<{ Bindings: Env }>().get('/', async c => {
  const addr = c.env.INFO.idFromName('');
  const obj = c.env.INFO.get(addr);
  const res = await obj.fetch(`${new URL(c.req.url).origin}/info`);
  const info = await res.json<InfoResult>();
  return c.json(info);
});

export class Info {
  private state: DurableObjectState;
  private app: Hono<{ Bindings: Env }>;
  private info?: InfoResult;

  constructor(state: DurableObjectState) {
    this.state = state;
    this.state.blockConcurrencyWhile(async () => {
      const info = await this.state.storage.get<InfoResult>('info');
      this.info = info ?? { lastBlockHeight: 0 };
    });

    this.app = new Hono();
    this.app
      .get('*', c => {
        return c.json(this.info);
      })
      .post('/last_block_height', async c => {
        if (!this.info) return c.text('', 500);
        const lastBlockHeight = Number(await c.req.text());
        this.info.lastBlockHeight = lastBlockHeight;
        await this.state.storage.put('info', this.info);
        return new Response(null, { status: 204 });
      });
  }

  async fetch(request: Request): Promise<Response> {
    return this.app.fetch(request);
  }
}