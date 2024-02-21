import { Hono } from 'hono';

import { CreateGame, Game, PlayMove, ResignGame } from './events';
import type { Env } from './global';

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
        const gameIdUri = c.req.param('game_id');
        if (!this.gameIds.includes(gameIdUri)) {
          return new Response(null, { status: 404 });
        }
        const game = await this.loadGame(gameIdUri);
        if (game instanceof Response) {
          return game;
        }
        return c.json(game);
      })
      .post('/:game_id/create_game', async c => {
        const gameIdUri = c.req.param('game_id');
        const createGame = await c.req.json<CreateGame>();

        this.games[gameIdUri] = { moves: [], ...createGame };
        await this.state.storage.put(`game:${gameIdUri}`, createGame);

        this.gameIds.push(gameIdUri);
        await this.state.storage.put('gameIds', this.gameIds);
        return new Response(null, { status: 204 });
      })
      .post('/:game_id/play_move', async c => {
        const gameIdUri = c.req.param('game_id');
        const game = await this.loadGame(gameIdUri);
        if (game instanceof Response) {
          return game;
        }
        const playMove = await c.req.json<PlayMove>();

        game.moves.push({
          color: playMove.color,
          mv: playMove.mv,
          board: playMove.board,
          outcome: playMove.outcome
        });
        await this.state.storage.put(`game:${gameIdUri}`, game);

        return new Response(null, { status: 204 });
      })
      .post('/:game_id/resign_game', async c => {
        const gameIdUri = c.req.param('game_id');
        const game = await this.loadGame(gameIdUri);
        if (game instanceof Response) {
          return game;
        }
        const resignGame = await c.req.json<ResignGame>();

        game.outcome = resignGame.outcome;
        game.resigner = resignGame.resigner;
        await this.state.storage.put(`game:${gameIdUri}`, game);

        return new Response(null, { status: 204 });
      })
      .post('/:game_id/cancel_game', async c => {
        const gameIdUri = c.req.param('game_id');
        const game = await this.loadGame(gameIdUri);
        if (game instanceof Response) {
          return game;
        }

        delete this.games[gameIdUri];
        await this.state.storage.delete(`game:${gameIdUri}`);

        return new Response(null, { status: 204 });
      });
  }

  async fetch(request: Request): Promise<Response> {
    return this.app.fetch(request);
  }

  private async loadGame(gameId: string): Promise<Game | Response> {
    let game: Game;
    if (!this.games[gameId]) {
      const loadGame = await this.state.storage.get<Game>(`game:${gameId}`);
      if (!loadGame) {
        return new Response(null, { status: 500 });
      }
      game = loadGame;
    } else {
      game = this.games[gameId];
    }
    return game;
  }
}
