import type { PushSubscriptionRow } from './db';
import { sendPush } from './push';

interface Notification {
  accountId: string;
  payload: {
    type: string;
    title: string;
    body: string;
    data: Record<string, unknown>;
  };
}

interface GameLookup {
  white_value: string;
  black_value: string | null;
  white_type: string;
  black_type: string;
}

interface ChallengeLookup {
  challenger: string;
  challenged: string;
}

type Db = import('./db').Db;

function gameIdFromData(data: Record<string, unknown>): string | null {
  if (!data.game_id) return null;
  return typeof data.game_id === 'string'
    ? data.game_id
    : JSON.stringify(data.game_id);
}

function opponent(game: GameLookup, movedColor: string): string | null {
  if (
    movedColor === 'White' &&
    game.white_type === 'Human' &&
    game.black_type === 'Human'
  ) {
    return game.black_value;
  }
  if (
    movedColor === 'Black' &&
    game.white_type === 'Human' &&
    game.black_type === 'Human'
  ) {
    return game.white_value;
  }
  return null;
}

function buildNotifications(
  eventType: string,
  data: Record<string, unknown>,
  games: Map<string, GameLookup>,
  challenges: Map<string, ChallengeLookup>
): Notification[] {
  switch (eventType) {
    case 'challenge': {
      return [
        {
          accountId: data.challenged as string,
          payload: {
            type: 'challenge_received',
            title: 'New Challenge!',
            body: `${data.challenger as string} challenged you`,
            data: {
              challenge_id: data.id,
              challenger: data.challenger
            }
          }
        }
      ];
    }
    case 'accept_challenge': {
      const challengeId = data.challenge_id as string;
      const challenge = challenges.get(challengeId);
      if (!challenge) return [];
      return [
        {
          accountId: challenge.challenger,
          payload: {
            type: 'challenge_accepted',
            title: 'Challenge Accepted!',
            body: `${challenge.challenged} accepted your challenge`,
            data: { challenge_id: challengeId, game_id: data.game_id }
          }
        }
      ];
    }
    case 'reject_challenge': {
      const challengeId = data.challenge_id as string;
      const challenge = challenges.get(challengeId);
      if (!challenge) return [];
      return [
        {
          accountId: challenge.challenger,
          payload: {
            type: 'challenge_rejected',
            title: 'Challenge Rejected',
            body: `${challenge.challenged} rejected your challenge`,
            data: { challenge_id: challengeId }
          }
        }
      ];
    }
    case 'play_move': {
      const gid = gameIdFromData(data);
      if (!gid) return [];
      const game = games.get(gid);
      if (!game) return [];
      const color = data.color as string;
      const outcome = data.outcome as Record<string, unknown> | null;
      if (outcome) {
        const accounts: string[] = [];
        if (game.white_type === 'Human') accounts.push(game.white_value);
        if (game.black_type === 'Human') accounts.push(game.black_value!);
        return accounts.map(accountId => ({
          accountId,
          payload: {
            type: 'game_over',
            title: 'Game Over!',
            body: formatOutcome(outcome),
            data: { game_id: data.game_id, outcome }
          }
        }));
      }
      const opp = opponent(game, color);
      if (!opp) return [];
      return [
        {
          accountId: opp,
          payload: {
            type: 'your_turn',
            title: 'Your Turn!',
            body: `It's your move in your chess game`,
            data: { game_id: data.game_id }
          }
        }
      ];
    }
    case 'resign_game': {
      const gid = gameIdFromData(data);
      if (!gid) return [];
      const game = games.get(gid);
      if (!game) return [];
      const resigner = data.resigner as string;
      const opp = opponent(game, resigner);
      if (!opp) return [];
      return [
        {
          accountId: opp,
          payload: {
            type: 'opponent_resigned',
            title: 'Opponent Resigned!',
            body: 'Your opponent resigned — you win!',
            data: { game_id: data.game_id }
          }
        }
      ];
    }
    case 'cancel_game': {
      const gid = gameIdFromData(data);
      if (!gid) return [];
      const game = games.get(gid);
      if (!game) return [];
      const cancelledBy = data.cancelled_by as string;
      const accounts: string[] = [];
      if (game.white_type === 'Human' && game.white_value !== cancelledBy)
        accounts.push(game.white_value);
      if (
        game.black_type === 'Human' &&
        game.black_value &&
        game.black_value !== cancelledBy
      )
        accounts.push(game.black_value);
      return accounts.map(accountId => ({
        accountId,
        payload: {
          type: 'game_cancelled',
          title: 'Game Cancelled',
          body: 'Your chess game was cancelled',
          data: { game_id: data.game_id }
        }
      }));
    }
    default:
      return [];
  }
}

function formatOutcome(outcome: Record<string, unknown>): string {
  if (outcome.result === 'Stalemate') return 'Draw — stalemate';
  const color = outcome.color as string;
  return `${color} wins!`;
}

export async function processNotifications(
  db: Db,
  vapidPrivateKey: CryptoKey,
  vapidPublicKeyB64: string,
  vapidSubject: string
): Promise<number> {
  const events = (await db`
    SELECT id, event_type, event_data FROM chess_events
    WHERE processed = true AND notified = false
    ORDER BY trigger_block_timestamp ASC
    LIMIT 100
  `) as Array<{
    id: string;
    event_type: string;
    event_data: Record<string, unknown>;
  }>;

  if (events.length === 0) return 0;

  const challengeIds: string[] = [];
  const gameIds: string[] = [];

  for (const e of events) {
    const data = e.event_data;
    if (
      e.event_type === 'accept_challenge' ||
      e.event_type === 'reject_challenge'
    ) {
      if (data.challenge_id) challengeIds.push(data.challenge_id as string);
    }
    if (['play_move', 'resign_game', 'cancel_game'].includes(e.event_type)) {
      const gid = gameIdFromData(data);
      if (gid) gameIds.push(gid);
    }
  }

  const challenges = new Map<string, ChallengeLookup>();
  if (challengeIds.length > 0) {
    const rows = (await db`
      SELECT id, challenger, challenged FROM challenges WHERE id = ANY(${challengeIds})
    `) as Array<{ id: string; challenger: string; challenged: string }>;
    for (const r of rows) challenges.set(r.id, r);
  }

  const games = new Map<string, GameLookup>();
  if (gameIds.length > 0) {
    const rows = (await db`
      SELECT game_id, white_type, white_value, black_type, black_value
      FROM games WHERE game_id = ANY(${gameIds})
    `) as Array<GameLookup & { game_id: string }>;
    for (const r of rows) games.set(r.game_id, r);
  }

  const allNotifications: Notification[] = [];
  for (const e of events) {
    allNotifications.push(
      ...buildNotifications(e.event_type, e.event_data, games, challenges)
    );
  }

  if (allNotifications.length === 0) {
    await db`UPDATE chess_events SET notified = true WHERE id = ANY(${events.map(e => e.id)})`;
    return events.length;
  }

  const accountIds = [...new Set(allNotifications.map(n => n.accountId))];

  const subs = (await db`
    SELECT account_id, endpoint, p256dh, auth FROM push_subscriptions
    WHERE account_id = ANY(${accountIds})
  `) as Array<{
    account_id: string;
    endpoint: string;
    p256dh: string;
    auth: string;
  }>;

  const subsByAccount = new Map<string, PushSubscriptionRow[]>();
  for (const s of subs) {
    const list = subsByAccount.get(s.account_id) || [];
    list.push({ endpoint: s.endpoint, p256dh: s.p256dh, auth: s.auth });
    subsByAccount.set(s.account_id, list);
  }

  const expiredEndpoints: string[] = [];
  const MAX_PUSH_SENDS = 40;
  let sends = 0;

  for (const notif of allNotifications) {
    if (sends >= MAX_PUSH_SENDS) break;
    const accountSubs = subsByAccount.get(notif.accountId) || [];
    for (const sub of accountSubs) {
      if (sends >= MAX_PUSH_SENDS) break;
      sends++;
      const result = await sendPush(
        sub,
        notif.payload,
        vapidPrivateKey,
        vapidPublicKeyB64,
        vapidSubject
      );
      if (result.subscriptionExpired) {
        expiredEndpoints.push(sub.endpoint);
      }
    }
  }

  await db`UPDATE chess_events SET notified = true WHERE id = ANY(${events.map(e => e.id)})`;

  if (expiredEndpoints.length > 0) {
    await db`DELETE FROM push_subscriptions WHERE endpoint = ANY(${expiredEndpoints})`;
  }

  return events.length;
}
