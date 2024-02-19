import { Hono } from 'hono';

import type { Env } from './global';

export type Game = {
  accountId: string;
  games: [];
};

export type GameId = [number, string, string?];

export const games = new Hono<{ Bindings: Env }>().get(
  '/:account_id',
  async c => {
    const addr = c.env.GAMES.idFromName('');
    const obj = c.env.GAMES.get(addr);
    const res = await obj.fetch(c.req.url);
    const info = await res.json<Game>();
    return c.json(info);
  }
);

export class Games {
  private state: DurableObjectState;
  private app: Hono<{ Bindings: Env }>;
  private gameIds: string[];
  private games: Record<string, Game>;

  constructor(state: DurableObjectState) {
    this.state = state;
    this.gameIds = [];
    this.games = {};
    this.state.blockConcurrencyWhile(async () => {
      const gameIds = await this.state.storage.get<string[]>('gameIds');
      this.gameIds = gameIds ?? [];
    });

    this.app = new Hono();
    this.app
      .get('/:game_id', async c => {
        const gameId = c.req.param('game_id');
        if (this.games[gameId]) {
          return c.json(this.games[gameId]);
        }
        if (this.gameIds.includes(gameId)) {
          const account = await this.state.storage.get<Game>(`game:${gameId}`);
          if (!account) {
            return new Response(null, { status: 500 });
          }
          this.games[gameId] = account;
          return c.json(account);
        }
        return new Response(null, { status: 404 });
      })
      .post('/:game_id/add_game', async () => {
        // TODO
        return new Response(null, { status: 501 });
      });
  }

  async fetch(request: Request): Promise<Response> {
    return this.app.fetch(request);
  }
}
