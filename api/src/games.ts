import { Hono } from 'hono';

import {
  Account,
  CreateGame,
  Game,
  GameId,
  GameOverview,
  PlayMove,
  ResignGame
} from './events';

const MAX_RECENT_LIMIT = 25;

export const games = new Hono()
  .get('/game/:game_id', async c => {
    const gameIdJson = c.req.param('game_id');
    const addr = c.env.GAMES.idFromName('');
    const obj = c.env.GAMES.get(addr);
    const res = await obj.fetch(
      `${new URL(c.req.url).origin}/game/${encodeURI(gameIdJson)}`
    );
    const game = await res.json<Game>();
    return c.json(game);
  })
  .post('/query', async c => {
    const addr = c.env.GAMES.idFromName('');
    const obj = c.env.GAMES.get(addr);
    const {
      gameIds,
      includeMoves
    }: { gameIds: GameId[]; includeMoves?: boolean } = await c.req.json();
    const res = await obj.fetch(`${new URL(c.req.url).origin}/query`, {
      method: 'POST',
      headers: {
        'Content-Type': 'application/json'
      },
      body: JSON.stringify({ gameIds, includeMoves: includeMoves ?? false })
    });
    const info = await res.json<Game[]>();
    return c.json(info);
  })
  .get('/recent/new', async c => {
    const { limit, include_moves } = c.req.query();
    const actualLimit = Math.min(
      limit ? Number(limit) : MAX_RECENT_LIMIT,
      MAX_RECENT_LIMIT
    );
    const includeMoves = Number(include_moves);
    const addr = c.env.GAMES.idFromName('');
    const obj = c.env.GAMES.get(addr);
    const res = await obj.fetch(
      `${new URL(c.req.url).origin}/recent/new?limit=${actualLimit}&include_moves=${includeMoves}`
    );
    const info = await res.json<Game[]>();
    return c.json(info);
  })
  .get('/recent/finished', async c => {
    const { limit, include_moves } = c.req.query();
    const actualLimit = Math.min(
      limit ? Number(limit) : MAX_RECENT_LIMIT,
      MAX_RECENT_LIMIT
    );
    const includeMoves = Number(include_moves);
    const addr = c.env.GAMES.idFromName('');
    const obj = c.env.GAMES.get(addr);
    const res = await obj.fetch(
      `${new URL(c.req.url).origin}/recent/finished?limit=${actualLimit}&include_moves=${includeMoves}`
    );
    const game = await res.json<Game[]>();
    return c.json(game);
  });

export class Games {
  private state: DurableObjectState;
  private app: Hono;
  private newGameIds: string[];
  private finishedGameIds: string[];
  private games: Record<string, Game>;
  private accounts: Record<string, Account>;

  constructor(state: DurableObjectState) {
    this.state = state;
    this.newGameIds = [];
    this.finishedGameIds = [];
    this.games = {};
    this.accounts = {};
    this.state.blockConcurrencyWhile(async () => {
      const newGameIds = await this.state.storage.get<string[]>('newGameIds');
      this.newGameIds = newGameIds ?? [];
      const finishedGameIds =
        await this.state.storage.get<string[]>('finishedGameIds');
      this.finishedGameIds = finishedGameIds ?? [];
    });

    this.app = new Hono();
    this.app
      .get('/game/:game_id', async c => {
        const gameIdJson = c.req.param('game_id');
        const game = await this.loadGame(gameIdJson);
        if (game instanceof Response) {
          return new Response('', { status: 404 });
        }
        return c.json(game);
      })
      .post('/query', async c => {
        const {
          gameIds,
          includeMoves
        }: { gameIds: GameId[]; includeMoves?: boolean } = await c.req.json();

        const games: GameOverview[] = [];
        for (const gameId of gameIds) {
          const gameIdJson = JSON.stringify(gameId);
          const game = await this.loadGame(gameIdJson);
          if (game instanceof Response) {
            continue;
          }
          games.push({
            game_id: game.game_id,
            white: game.white,
            black: game.black,
            board: game.board,
            outcome: game.outcome,
            resigner: game.resigner,
            moves: includeMoves ? game.moves : undefined
          });
        }

        return c.json(games);
      })
      .get('/recent/new', async c => {
        const { limit, include_moves } = c.req.query();
        const end = Number(limit);
        const includeMoves = Number(include_moves);
        let index = 0;
        const games: GameOverview[] = [];
        for (const gameId of this.newGameIds) {
          const game = await this.loadGame(gameId);
          if (game instanceof Response) {
            continue;
          }
          games.push({
            game_id: game.game_id,
            white: game.white,
            black: game.black,
            board: game.board,
            outcome: game.outcome,
            resigner: game.resigner,
            moves: includeMoves ? game.moves : undefined
          });

          index++;
          if (index >= end) {
            break;
          }
        }

        return c.json(games);
      })
      .get('/recent/finished', async c => {
        const { limit, include_moves } = c.req.query();
        const end = Number(limit);
        const includeMoves = Number(include_moves);
        let index = 0;
        const games: GameOverview[] = [];
        for (const gameId of this.finishedGameIds) {
          const game = await this.loadGame(gameId);
          if (game instanceof Response) {
            continue;
          }
          games.push({
            game_id: game.game_id,
            white: game.white,
            black: game.black,
            board: game.board,
            outcome: game.outcome,
            resigner: game.resigner,
            moves: includeMoves ? game.moves : undefined
          });

          index++;
          if (index >= end) {
            break;
          }
        }

        return c.json(games);
      })
      .get('/account/:account_id', async c => {
        const accountId = c.req.param('account_id');
        const account = await this.loadAccount(accountId);

        return c.json(account);
      })
      .post('/:game_id/create_game', async c => {
        const gameIdJson = c.req.param('game_id');
        const createGame = await c.req.json<CreateGame>();

        this.games[gameIdJson] = { moves: [], ...createGame };
        await this.state.storage.put(`game:${gameIdJson}`, createGame, {
          allowUnconfirmed: false
        });

        this.newGameIds.unshift(gameIdJson);
        if (this.newGameIds.length > MAX_RECENT_LIMIT) {
          this.newGameIds.pop();
        }
        await this.state.storage.put('newGameIds', this.newGameIds, {
          allowUnconfirmed: false
        });

        return new Response(null, { status: 204 });
      })
      .post('/:game_id/play_move', async c => {
        const gameIdJson = c.req.param('game_id');
        const game = await this.loadGame(gameIdJson);
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
          this.finishedGameIds.unshift(gameIdJson);
          if (this.finishedGameIds.length > MAX_RECENT_LIMIT) {
            this.finishedGameIds.pop();
          }
          await this.state.storage.put(
            'finishedGameIds',
            this.finishedGameIds,
            {
              allowUnconfirmed: false
            }
          );

          await this.storeFinishedGame(game);
        }
        game.board = playMove.board;
        await this.state.storage.put(`game:${gameIdJson}`, game, {
          allowUnconfirmed: false
        });

        return new Response(null, { status: 204 });
      })
      .post('/:game_id/resign_game', async c => {
        const gameIdJson = c.req.param('game_id');
        const game = await this.loadGame(gameIdJson);
        if (game instanceof Response) {
          return game;
        }
        const resignGame = await c.req.json<ResignGame>();

        game.outcome = resignGame.outcome;
        game.resigner = resignGame.resigner;
        await this.state.storage.put(`game:${gameIdJson}`, game, {
          allowUnconfirmed: false
        });

        this.finishedGameIds.unshift(gameIdJson);
        if (this.finishedGameIds.length > MAX_RECENT_LIMIT) {
          this.finishedGameIds.pop();
        }
        await this.state.storage.put('finishedGameIds', this.finishedGameIds, {
          allowUnconfirmed: false
        });

        await this.storeFinishedGame(game);

        return new Response(null, { status: 204 });
      })
      .post('/:game_id/cancel_game', async c => {
        const gameIdJson = c.req.param('game_id');
        const game = await this.loadGame(gameIdJson);
        if (game instanceof Response) {
          return game;
        }

        delete this.games[gameIdJson];
        await this.state.storage.delete(`game:${gameIdJson}`, {
          allowUnconfirmed: false
        });

        const index = this.newGameIds.findIndex(
          gameId => gameId === gameIdJson
        );
        if (index >= 0) {
          this.newGameIds.splice(index, 1);
          await this.state.storage.put('newGameIds', this.newGameIds, {
            allowUnconfirmed: false
          });
        }

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

  private async loadAccount(accountId: string): Promise<Account> {
    if (!this.accounts[accountId]) {
      const loadedAccount = await this.state.storage.get<Account>(
        `account:${accountId}`
      );
      const account = loadedAccount ?? {
        finishedGameIds: []
      };
      this.accounts[accountId] = account;
      return account;
    } else {
      return this.accounts[accountId];
    }
  }

  private async storeFinishedGame(game: Game) {
    if (game.white.type === 'Human') {
      const account = await this.loadAccount(game.white.value);
      account.finishedGameIds.push(game.game_id);
      await this.state.storage.put(`account:${game.white.value}`, account);
    }
    if (game.black.type === 'Human') {
      const account = await this.loadAccount(game.black.value);
      account.finishedGameIds.push(game.game_id);
      await this.state.storage.put(`account:${game.black.value}`, account);
    }
  }
}
