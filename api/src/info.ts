import { Hono } from 'hono';

export type InfoResult = {
  lastBlockHeight: number;
};

export const info = new Hono().get('/', async c => {
  const addr = c.env.INFO.idFromName('');
  const obj = c.env.INFO.get(addr);
  const res = await obj.fetch(`${new URL(c.req.url).origin}/info`);
  const info = await res.json<InfoResult>();
  return c.json(info);
});

export class Info {
  private state: DurableObjectState;
  private app: Hono;
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
