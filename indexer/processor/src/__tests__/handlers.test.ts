import { afterAll, beforeAll, beforeEach, describe, expect, it } from 'vitest';

import { getDb, type Db } from '../db.js';
import { handleEvent, type RawEvent } from '../handlers.js';

import {
  makeAcceptChallenge,
  makeCancelBet,
  makeCancelGame,
  makeChallenge,
  makeCreateGame,
  makeLockBets,
  makePlaceBet,
  makePlayMove,
  makeRejectChallenge,
  makeResignGame,
  makeResolveBets,
  STARTING_BOARD
} from './fixtures.js';

const GAME_ID = [123, 'alice.near', 'bob.near'] as [
  number,
  string,
  string | null
];
const AI_GAME_ID = [456, 'alice.near', null] as [number, string, string | null];

describe('handlers', () => {
  let db: Db;

  beforeAll(async () => {
    db = getDb(process.env.TEST_DATABASE_URL!);
  });

  afterAll(async () => {
    await db.end();
  });

  beforeEach(async () => {
    await db`TRUNCATE games, game_moves, challenges, account_finished_games, bets CASCADE`;
  });

  async function processEvent(event: RawEvent) {
    await db.begin(async sql => {
      await handleEvent(sql, event);
    });
  }

  function parseJson<T>(val: unknown): T {
    return typeof val === 'string' ? JSON.parse(val) : (val as T);
  }

  async function getGame(gid: typeof GAME_ID) {
    const rows =
      await db`SELECT * FROM games WHERE game_id = ${JSON.stringify(gid)}`;
    const row = rows[0] as Record<string, unknown> | undefined;
    if (!row) return undefined;
    row.outcome = parseJson(row.outcome);
    row.moves = parseJson(row.moves);
    row.board = parseJson(row.board);
    return row;
  }

  async function getMoves(gid: typeof GAME_ID) {
    return db`SELECT * FROM game_moves WHERE game_id = ${JSON.stringify(gid)} ORDER BY move_number`;
  }

  async function getFinishedGames(accountId: string) {
    return db`SELECT game_id FROM account_finished_games WHERE account_id = ${accountId}`;
  }

  describe('create_game', () => {
    it('inserts a game with computed FEN', async () => {
      await processEvent(makeCreateGame(GAME_ID));

      const game = await getGame(GAME_ID);
      expect(game).toBeDefined();
      expect(game!.status).toBe('in_progress');
      expect(game!.fen).toBe(
        'rnbqkbnr/pppppppp/8/8/8/8/PPPPPPPP/RNBQKBNR w - - 0 1'
      );
      expect(game!.white_type).toBe('Human');
      expect(game!.white_value).toBe('alice.near');
      expect(game!.black_type).toBe('Human');
      expect(game!.black_value).toBe('bob.near');
    });

    it('handles AI games', async () => {
      await processEvent(
        makeCreateGame(
          AI_GAME_ID,
          { type: 'Human', value: 'alice.near' },
          { type: 'Ai', value: 'Easy' }
        )
      );

      const game = await getGame(AI_GAME_ID);
      expect(game).toBeDefined();
      expect(game!.white_type).toBe('Human');
      expect(game!.black_type).toBe('Ai');
      expect(game!.black_value).toBe('Easy');
    });

    it('is idempotent on conflict', async () => {
      await processEvent(makeCreateGame(GAME_ID));
      await processEvent(makeCreateGame(GAME_ID));

      const rows = await db`SELECT COUNT(*) as count FROM games`;
      expect(Number((rows[0] as Record<string, string>).count)).toBe(1);
    });
  });

  describe('play_move', () => {
    it('appends a move and updates the game', async () => {
      await processEvent(makeCreateGame(GAME_ID));
      await processEvent(makePlayMove(GAME_ID));

      const game = await getGame(GAME_ID);
      expect(game!.fen).toContain('4P3');
      const moves = game!.moves as unknown[];
      expect(moves).toHaveLength(1);

      const moveRows = await getMoves(GAME_ID);
      expect(moveRows).toHaveLength(1);
      expect((moveRows[0] as Record<string, unknown>).move_number).toBe(1);
      expect((moveRows[0] as Record<string, unknown>).color).toBe('White');
      expect((moveRows[0] as Record<string, unknown>).move_notation).toBe('e4');
    });

    it('finishes game when outcome is present', async () => {
      await processEvent(makeCreateGame(GAME_ID));
      const outcome = { result: 'Victory', color: 'White' };
      await processEvent(
        makePlayMove(GAME_ID, 'White', 'Qf7', STARTING_BOARD, outcome)
      );

      const game = await getGame(GAME_ID);
      expect(game!.status).toBe('finished');
      expect(game!.outcome).toEqual(outcome);
      expect(game!.finished_at).toBeDefined();

      const finishedWhite = await getFinishedGames('alice.near');
      expect(finishedWhite).toHaveLength(1);
      const finishedBlack = await getFinishedGames('bob.near');
      expect(finishedBlack).toHaveLength(1);
    });

    it('does not add AI accounts to account_finished_games', async () => {
      await processEvent(
        makeCreateGame(
          AI_GAME_ID,
          { type: 'Human', value: 'alice.near' },
          { type: 'Ai', value: 'Easy' }
        )
      );
      const outcome = { result: 'Victory', color: 'White' };
      await processEvent(
        makePlayMove(AI_GAME_ID, 'White', 'Qf7', STARTING_BOARD, outcome)
      );

      const finished = await getFinishedGames('alice.near');
      expect(finished).toHaveLength(1);
    });
  });

  describe('resign_game', () => {
    it('marks game as finished with resigner', async () => {
      await processEvent(makeCreateGame(GAME_ID));
      await processEvent(makeResignGame(GAME_ID));

      const game = await getGame(GAME_ID);
      expect(game!.status).toBe('finished');
      expect(game!.resigner).toBe('White');
      expect(game!.outcome).toEqual({ result: 'Victory', color: 'Black' });
      expect(game!.finished_at).toBeDefined();

      const finished = await getFinishedGames('alice.near');
      expect(finished).toHaveLength(1);
    });
  });

  describe('cancel_game', () => {
    it('marks game as cancelled', async () => {
      await processEvent(makeCreateGame(GAME_ID));
      const outcome = { result: 'Victory', color: 'White' };
      await processEvent(
        makePlayMove(GAME_ID, 'White', 'Qf7', STARTING_BOARD, outcome)
      );
      expect(await getFinishedGames('alice.near')).toHaveLength(1);

      await processEvent(makeCancelGame(GAME_ID));

      const game = await getGame(GAME_ID);
      expect(game!.status).toBe('cancelled');
      expect(await getFinishedGames('alice.near')).toHaveLength(0);
    });
  });

  describe('challenge flow', () => {
    it('creates, accepts, and rejects challenges', async () => {
      await processEvent(makeChallenge('c1'));
      await processEvent(makeChallenge('c2'));

      let rows = await db`SELECT * FROM challenges ORDER BY id`;
      expect(rows).toHaveLength(2);
      expect((rows[0] as Record<string, unknown>).status).toBe('pending');

      await processEvent(makeAcceptChallenge('c1', GAME_ID));
      rows = await db`SELECT * FROM challenges WHERE id = 'c1'`;
      expect((rows[0] as Record<string, unknown>).status).toBe('accepted');
      expect((rows[0] as Record<string, unknown>).game_id).toBe(
        JSON.stringify(GAME_ID)
      );

      await processEvent(makeRejectChallenge('c2'));
      rows = await db`SELECT * FROM challenges WHERE id = 'c2'`;
      expect((rows[0] as Record<string, unknown>).status).toBe('rejected');
    });

    it('is idempotent on conflict', async () => {
      await processEvent(makeChallenge('c1'));
      await processEvent(makeChallenge('c1'));

      const rows = await db`SELECT COUNT(*) as count FROM challenges`;
      expect(Number((rows[0] as Record<string, string>).count)).toBe(1);
    });
  });

  describe('bet lifecycle', () => {
    const players = ['alice.near', 'bob.near'] as [string, string];

    it('places, locks, and resolves bets with payouts', async () => {
      await processEvent(makeCreateGame(GAME_ID));
      await processEvent(makePlaceBet('carol.near', players));
      await processEvent(makePlaceBet('dave.near', players, 'usdc.testnet', '1000000', 'bob.near'));

      let rows = await db`SELECT * FROM bets ORDER BY bettor`;
      expect(rows).toHaveLength(2);
      expect((rows[0] as Record<string, unknown>).status).toBe('pending');

      await processEvent(makeLockBets(players, GAME_ID));
      rows = await db`SELECT * FROM bets WHERE status = 'locked'`;
      expect(rows).toHaveLength(2);
      for (const r of rows) {
        expect((r as Record<string, unknown>).game_id).toBe(
          JSON.stringify(GAME_ID)
        );
      }

      await processEvent(
        makeResolveBets(players, GAME_ID)
      );
      rows = await db`SELECT * FROM bets ORDER BY bettor`;
      for (const r of rows) {
        expect((r as Record<string, unknown>).status).toBe('resolved');
      }
      const carol = rows.find(
        r => (r as Record<string, unknown>).bettor === 'carol.near'
      )!;
      expect((carol as Record<string, unknown>).payout).toBe('2000000');
      const dave = rows.find(
        r => (r as Record<string, unknown>).bettor === 'dave.near'
      )!;
      expect((dave as Record<string, unknown>).payout).toBeNull();
    });

    it('cancels a pending bet', async () => {
      await processEvent(makePlaceBet('carol.near', players));
      await processEvent(makeCancelBet('carol.near', players));

      const rows = await db`SELECT COUNT(*) as count FROM bets`;
      expect(Number((rows[0] as Record<string, string>).count)).toBe(0);
    });

    it('upserts duplicate place_bet', async () => {
      await processEvent(
        makePlaceBet('carol.near', players, 'usdc.testnet', '1000')
      );
      await processEvent(
        makePlaceBet('carol.near', players, 'usdc.testnet', '2000')
      );

      const rows = await db`SELECT amount FROM bets`;
      expect(rows).toHaveLength(1);
      expect((rows[0] as Record<string, unknown>).amount).toBe('2000');
    });
  });

  describe('full game flow', () => {
    it('challenge → create → play → finish', async () => {
      await processEvent(makeChallenge('c1'));
      await processEvent(makeCreateGame(GAME_ID));
      await processEvent(makeAcceptChallenge('c1', GAME_ID));

      await processEvent(makePlayMove(GAME_ID));
      await processEvent(
        makePlayMove(GAME_ID, 'Black', 'e5', STARTING_BOARD, null)
      );

      const outcome = { result: 'Victory', color: 'White' };
      await processEvent(
        makePlayMove(GAME_ID, 'White', 'Qf7', STARTING_BOARD, outcome)
      );

      const game = await getGame(GAME_ID);
      expect(game!.status).toBe('finished');
      expect(game!.outcome).toEqual(outcome);

      const moves = await getMoves(GAME_ID);
      expect(moves).toHaveLength(3);

      expect(await getFinishedGames('alice.near')).toHaveLength(1);
      expect(await getFinishedGames('bob.near')).toHaveLength(1);

      const challenge = await db`SELECT * FROM challenges WHERE id = 'c1'`;
      expect((challenge[0] as Record<string, unknown>).status).toBe('accepted');
    });
  });
});
