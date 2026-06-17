import { beforeEach, describe, expect, it, vi } from 'vitest';

import type { Db } from './db';
import {
  processNotifications,
  processQuestCooldownNotifications
} from './notify';
import type { SendPushFn } from './push';

type DbCall = { query: string; values: unknown[] };

function createMockDb(options: {
  results: Array<{ match: RegExp; rows: unknown[] }>;
  onCall?: (call: DbCall) => void;
}): Db {
  const fn = async (
    strings: TemplateStringsArray,
    ...values: unknown[]
  ): Promise<unknown[]> => {
    const query = strings.join('?');
    options.onCall?.({ query, values });
    for (const r of options.results) {
      if (r.match.test(query)) return r.rows;
    }
    return [];
  };
  return fn as unknown as Db;
}

const fakeKey = 'fake-private-key';
const fakeSendPush = vi.fn<SendPushFn>(async () => ({
  ok: true,
  subscriptionExpired: false,
  status: 200
}));

beforeEach(() => {
  fakeSendPush.mockClear();
});

function makePlayMoveEvent(id: string, gameIdTuple: [number, string, string]) {
  return {
    id,
    event_type: 'play_move',
    event_data: {
      game_id: gameIdTuple,
      color: 'White'
    }
  };
}

function makeGameRow(gameIdTuple: [number, string, string]) {
  return {
    game_id: JSON.stringify(gameIdTuple),
    white_type: 'Human',
    white_value: gameIdTuple[1],
    black_type: 'Human',
    black_value: gameIdTuple[2]
  };
}

describe('processNotifications', () => {
  it('sends notifications and marks events notified', async () => {
    const event = makePlayMoveEvent('evt-1', [1, 'alice', 'bob']);
    const gameRow = makeGameRow([1, 'alice', 'bob']);
    const notifiedIds: string[] = [];

    const db = createMockDb({
      results: [
        {
          match: /SELECT id, event_type, event_data FROM chess_events/,
          rows: [event]
        },
        {
          match:
            /SELECT game_id, white_type, white_value, black_type, black_value/,
          rows: [gameRow]
        },
        {
          match:
            /SELECT account_id, endpoint, p256dh, auth FROM push_subscriptions/,
          rows: [
            {
              account_id: 'bob',
              endpoint: 'https://push.example/bob',
              p256dh: 'p256dh-bob',
              auth: 'auth-bob'
            }
          ]
        },
        { match: /UPDATE chess_events SET notified = true/, rows: [] },
        { match: /DELETE FROM push_subscriptions/, rows: [] }
      ],
      onCall: call => {
        if (/UPDATE chess_events SET notified = true/.test(call.query)) {
          notifiedIds.push(...(call.values[0] as string[]));
        }
      }
    });

    const count = await processNotifications(db, fakeKey, 'pub', 'subj', {
      sendPush: fakeSendPush
    });

    expect(count).toBe(1);
    expect(fakeSendPush).toHaveBeenCalledTimes(1);
    expect(notifiedIds).toEqual(['evt-1']);
  });

  it('only marks fully-sent events as notified when the send cap is reached', async () => {
    const events = [
      makePlayMoveEvent('evt-1', [1, 'alice', 'bob']),
      makePlayMoveEvent('evt-2', [2, 'alice', 'bob']),
      makePlayMoveEvent('evt-3', [3, 'alice', 'bob'])
    ];
    const games = events.map((e, i) => makeGameRow([i + 1, 'alice', 'bob']));
    const notifiedIds: string[] = [];

    const db = createMockDb({
      results: [
        {
          match: /SELECT id, event_type, event_data FROM chess_events/,
          rows: events
        },
        {
          match:
            /SELECT game_id, white_type, white_value, black_type, black_value/,
          rows: games
        },
        {
          match:
            /SELECT account_id, endpoint, p256dh, auth FROM push_subscriptions/,
          rows: [
            {
              account_id: 'bob',
              endpoint: 'https://push.example/bob',
              p256dh: 'p256dh-bob',
              auth: 'auth-bob'
            }
          ]
        },
        { match: /UPDATE chess_events SET notified = true/, rows: [] },
        { match: /DELETE FROM push_subscriptions/, rows: [] }
      ],
      onCall: call => {
        if (/UPDATE chess_events SET notified = true/.test(call.query)) {
          notifiedIds.push(...(call.values[0] as string[]));
        }
      }
    });

    const count = await processNotifications(db, fakeKey, 'pub', 'subj', {
      sendPush: fakeSendPush,
      maxSends: 2
    });

    expect(count).toBe(3);
    expect(fakeSendPush).toHaveBeenCalledTimes(2);
    expect(notifiedIds).toEqual(['evt-1', 'evt-2']);
  });

  it('marks events notified when no subscriptions exist', async () => {
    const event = makePlayMoveEvent('evt-1', [1, 'alice', 'bob']);
    const gameRow = makeGameRow([1, 'alice', 'bob']);
    const notifiedIds: string[] = [];

    const db = createMockDb({
      results: [
        {
          match: /SELECT id, event_type, event_data FROM chess_events/,
          rows: [event]
        },
        {
          match:
            /SELECT game_id, white_type, white_value, black_type, black_value/,
          rows: [gameRow]
        },
        {
          match:
            /SELECT account_id, endpoint, p256dh, auth FROM push_subscriptions/,
          rows: []
        },
        { match: /UPDATE chess_events SET notified = true/, rows: [] },
        { match: /DELETE FROM push_subscriptions/, rows: [] }
      ],
      onCall: call => {
        if (/UPDATE chess_events SET notified = true/.test(call.query)) {
          notifiedIds.push(...(call.values[0] as string[]));
        }
      }
    });

    const count = await processNotifications(db, fakeKey, 'pub', 'subj', {
      sendPush: fakeSendPush
    });

    expect(count).toBe(1);
    expect(fakeSendPush).not.toHaveBeenCalled();
    expect(notifiedIds).toEqual(['evt-1']);
  });

  it('deletes expired subscriptions after sending', async () => {
    const event = makePlayMoveEvent('evt-1', [1, 'alice', 'bob']);
    const gameRow = makeGameRow([1, 'alice', 'bob']);
    const expiredEndpoints: string[] = [];

    fakeSendPush.mockResolvedValueOnce({
      ok: false,
      subscriptionExpired: true,
      status: 410
    });

    const db = createMockDb({
      results: [
        {
          match: /SELECT id, event_type, event_data FROM chess_events/,
          rows: [event]
        },
        {
          match:
            /SELECT game_id, white_type, white_value, black_type, black_value/,
          rows: [gameRow]
        },
        {
          match:
            /SELECT account_id, endpoint, p256dh, auth FROM push_subscriptions/,
          rows: [
            {
              account_id: 'bob',
              endpoint: 'https://push.example/bob',
              p256dh: 'p256dh-bob',
              auth: 'auth-bob'
            }
          ]
        },
        { match: /UPDATE chess_events SET notified = true/, rows: [] },
        { match: /DELETE FROM push_subscriptions/, rows: [] }
      ],
      onCall: call => {
        if (/DELETE FROM push_subscriptions/.test(call.query)) {
          expiredEndpoints.push(...(call.values[0] as string[]));
        }
      }
    });

    await processNotifications(db, fakeKey, 'pub', 'subj', {
      sendPush: fakeSendPush
    });

    expect(expiredEndpoints).toEqual(['https://push.example/bob']);
  });

  it('sends game_started notification to challenged player on create_game', async () => {
    const event = {
      id: 'evt-create-1',
      event_type: 'create_game',
      event_data: {
        game_id: [1, 'alice', 'bob'],
        white: { Human: 'alice' },
        black: { Human: 'bob' },
        board: []
      }
    };
    const notifiedIds: string[] = [];

    const db = createMockDb({
      results: [
        {
          match: /SELECT id, event_type, event_data FROM chess_events/,
          rows: [event]
        },
        {
          match:
            /SELECT account_id, endpoint, p256dh, auth FROM push_subscriptions/,
          rows: [
            {
              account_id: 'bob',
              endpoint: 'https://push.example/bob',
              p256dh: 'p256dh-bob',
              auth: 'auth-bob'
            }
          ]
        },
        { match: /UPDATE chess_events SET notified = true/, rows: [] },
        { match: /DELETE FROM push_subscriptions/, rows: [] }
      ],
      onCall: call => {
        if (/UPDATE chess_events SET notified = true/.test(call.query)) {
          notifiedIds.push(...(call.values[0] as string[]));
        }
      }
    });

    const count = await processNotifications(db, fakeKey, 'pub', 'subj', {
      sendPush: fakeSendPush
    });

    expect(count).toBe(1);
    expect(fakeSendPush).toHaveBeenCalledTimes(1);
    expect(notifiedIds).toEqual(['evt-create-1']);
  });
});

describe('processQuestCooldownNotifications', () => {
  it('sends quest ready notifications and marks them notified', async () => {
    const notifiedQuests: Array<{ account_id: string; quest: string }> = [];

    const db = createMockDb({
      results: [
        {
          match: /SELECT DISTINCT account_id FROM push_subscriptions/,
          rows: [{ account_id: 'bob' }]
        },
        { match: /DELETE FROM quest_cooldowns WHERE account_id/, rows: [] },
        { match: /INSERT INTO quest_cooldowns/, rows: [] },
        {
          match: /SELECT account_id, quest FROM quest_cooldowns/,
          rows: [{ account_id: 'bob', quest: 'DailyPlayMove' }]
        },
        {
          match:
            /SELECT account_id, endpoint, p256dh, auth FROM push_subscriptions/,
          rows: [
            {
              account_id: 'bob',
              endpoint: 'https://push.example/bob',
              p256dh: 'p256dh-bob',
              auth: 'auth-bob'
            }
          ]
        },
        { match: /UPDATE quest_cooldowns SET notified = true/, rows: [] },
        { match: /DELETE FROM push_subscriptions/, rows: [] }
      ],
      onCall: call => {
        if (/UPDATE quest_cooldowns SET notified = true/.test(call.query)) {
          notifiedQuests.push({
            account_id: call.values[0] as string,
            quest: call.values[1] as string
          });
        }
      }
    });

    const fetchCooldowns = vi.fn(async () => [
      [Date.now(), 'DailyPlayMove'] as [number, string]
    ]);

    const count = await processQuestCooldownNotifications(
      db,
      fakeKey,
      'pub',
      'subj',
      'https://rpc.example',
      'contract',
      { sendPush: fakeSendPush, fetchCooldowns }
    );

    expect(count).toBe(1);
    expect(fakeSendPush).toHaveBeenCalledTimes(1);
    expect(notifiedQuests).toEqual([
      { account_id: 'bob', quest: 'DailyPlayMove' }
    ]);
  });

  it('only marks fully-sent quests as notified when the send cap is reached', async () => {
    const notifiedQuests: Array<{ account_id: string; quest: string }> = [];

    const db = createMockDb({
      results: [
        {
          match: /SELECT DISTINCT account_id FROM push_subscriptions/,
          rows: [{ account_id: 'bob' }]
        },
        { match: /DELETE FROM quest_cooldowns WHERE account_id/, rows: [] },
        { match: /INSERT INTO quest_cooldowns/, rows: [] },
        {
          match: /SELECT account_id, quest FROM quest_cooldowns/,
          rows: [
            { account_id: 'bob', quest: 'DailyPlayMove' },
            { account_id: 'bob', quest: 'DailyGame' },
            { account_id: 'bob', quest: 'WeeklyWin' }
          ]
        },
        {
          match:
            /SELECT account_id, endpoint, p256dh, auth FROM push_subscriptions/,
          rows: [
            {
              account_id: 'bob',
              endpoint: 'https://push.example/bob',
              p256dh: 'p256dh-bob',
              auth: 'auth-bob'
            }
          ]
        },
        { match: /UPDATE quest_cooldowns SET notified = true/, rows: [] },
        { match: /DELETE FROM push_subscriptions/, rows: [] }
      ],
      onCall: call => {
        if (/UPDATE quest_cooldowns SET notified = true/.test(call.query)) {
          notifiedQuests.push({
            account_id: call.values[0] as string,
            quest: call.values[1] as string
          });
        }
      }
    });

    const fetchCooldowns = vi.fn(async () => [
      [Date.now(), 'DailyPlayMove'] as [number, string],
      [Date.now(), 'DailyGame'] as [number, string],
      [Date.now(), 'WeeklyWin'] as [number, string]
    ]);

    const count = await processQuestCooldownNotifications(
      db,
      fakeKey,
      'pub',
      'subj',
      'https://rpc.example',
      'contract',
      { sendPush: fakeSendPush, fetchCooldowns, maxSends: 2 }
    );

    expect(count).toBe(3);
    expect(fakeSendPush).toHaveBeenCalledTimes(2);
    expect(notifiedQuests).toEqual([
      { account_id: 'bob', quest: 'DailyPlayMove' },
      { account_id: 'bob', quest: 'DailyGame' }
    ]);
  });
});
