import { Hono } from 'hono';

import { CreateGame, Game, PlayMove, ResignGame } from './events';
import type { Env } from './global';

export const games = new Hono<{ Bindings: Env }>().get('/:game_id', async c => {
  const gameIdUri = c.req.param('game_id');
  console.log('GAMEID', gameIdUri, typeof gameIdUri, encodeURI(gameIdUri));
  const addr = c.env.GAMES.idFromName('');
  const obj = c.env.GAMES.get(addr);
  const res = await obj.fetch(
    `${new URL(c.req.url).origin}/${encodeURI(gameIdUri)}`
  );
  const info = await res.json<Game>();
  return c.json(info);
});

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
        console.log('GAMEID DUR', gameIdUri);
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
        this.gameIds.unshift(gameIdUri);

        await this.state.storage.put(`game:${gameIdUri}`, createGame, {
          allowUnconfirmed: true
        });
        await this.state.storage.put('gameIds', this.gameIds, {
          allowUnconfirmed: true
        });

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
          board: playMove.board
        });
        if (playMove.outcome != null) {
          game.outcome = playMove.outcome;
        }
        game.board = playMove.board;
        await this.state.storage.put(`game:${gameIdUri}`, game, {
          allowUnconfirmed: true
        });

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
        await this.state.storage.put(`game:${gameIdUri}`, game, {
          allowUnconfirmed: true
        });

        return new Response(null, { status: 204 });
      })
      .post('/:game_id/cancel_game', async c => {
        const gameIdUri = c.req.param('game_id');
        const game = await this.loadGame(gameIdUri);
        if (game instanceof Response) {
          return game;
        }

        delete this.games[gameIdUri];
        const index = this.gameIds.findIndex(gameId => gameId === gameIdUri);
        if (index >= 0) {
          this.gameIds.splice(index, 1);
        }

        await this.state.storage.delete(`game:${gameIdUri}`);
        await this.state.storage.put('gameIds', this.gameIds, {
          allowUnconfirmed: true
        });

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
        // returns 204, because it might be from old version of contract
        return new Response(null, { status: 204 });
      }
      game = loadGame;
    } else {
      game = this.games[gameId];
    }
    return game;
  }
}
