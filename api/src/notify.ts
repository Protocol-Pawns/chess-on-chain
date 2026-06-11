import type { PushSubscriptionRow } from './db';
import { sendPush } from './push';

function gameUrlPath(gameId: unknown): string {
  return `/game/${encodeURIComponent(typeof gameId === 'string' ? gameId : JSON.stringify(gameId))}`;
}

interface Notification {
  accountId: string;
  payload: {
    type: string;
    title: string;
    body: string;
    url?: string;
    data: Record<string, unknown>;
  };
}

const QUEST_COOLDOWNS_MS: Record<string, number> = {
  DailyPlayMove: 1_000 * 60 * 60 * 24 - 1_000 * 60 * 60 * 8,
  DailyGame: 1_000 * 60 * 60 * 24 - 1_000 * 60 * 60 * 8,
  WeeklyWin: 1_000 * 60 * 60 * 24 * 7 - 1_000 * 60 * 60 * 8,
  WeeklyBettor: 1_000 * 60 * 60 * 24 * 7 - 1_000 * 60 * 60 * 8,
  WeeklyChallenger: 1_000 * 60 * 60 * 24 * 7 - 1_000 * 60 * 60 * 8
};

const QUEST_LABELS: Record<string, string> = {
  DailyPlayMove: 'Play a Move',
  DailyGame: 'Complete a Game',
  WeeklyWin: 'Win vs Human',
  WeeklyBettor: 'Place a Bet',
  WeeklyChallenger: 'Challenge a Player'
};

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
            url: '/challenges',
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
            url: gameUrlPath(data.game_id),
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
            url: '/challenges',
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
            url: gameUrlPath(data.game_id),
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
            url: gameUrlPath(data.game_id),
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
            url: gameUrlPath(data.game_id),
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
          url: '/',
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
  const rawEvents = (await db`
    SELECT id, event_type, event_data FROM chess_events
    WHERE processed = true AND notified = false
    ORDER BY trigger_block_timestamp::bigint ASC
    LIMIT 100
  `) as Array<{
    id: string;
    event_type: string;
    event_data: string | Record<string, unknown>;
  }>;

  const events = rawEvents.map(e => ({
    ...e,
    event_data:
      typeof e.event_data === 'string'
        ? (JSON.parse(e.event_data) as Record<string, unknown>)
        : e.event_data
  }));

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
      SELECT challenge_id, challenger, challenged FROM challenges WHERE challenge_id = ANY(${challengeIds})
    `) as Array<{
      challenge_id: string;
      challenger: string;
      challenged: string;
    }>;
    for (const r of rows) challenges.set(r.challenge_id, r);
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

async function fetchQuestCooldowns(
  rpcUrl: string,
  contractId: string,
  accountId: string
): Promise<Array<[number, string]>> {
  const args = btoa(JSON.stringify({ account_id: accountId }));
  const res = await fetch(rpcUrl, {
    method: 'POST',
    headers: { 'Content-Type': 'application/json' },
    body: JSON.stringify({
      jsonrpc: '2.0',
      id: 1,
      method: 'query',
      params: {
        request_type: 'call_function',
        account_id: contractId,
        method_name: 'get_quest_cooldowns',
        args_base64: args,
        finality: 'final'
      }
    })
  });

  const json = await res.json<{
    result?: { result: number[] };
    error?: { message: string };
  }>();

  if (json.error) {
    console.error(
      `RPC error fetching quest cooldowns for ${accountId}: ${json.error.message}`
    );
    return [];
  }

  const resultBytes = json.result?.result;
  if (!resultBytes || resultBytes.length === 0) return [];

  const decoded = new TextDecoder().decode(new Uint8Array(resultBytes));
  return JSON.parse(decoded);
}

export async function processQuestCooldownNotifications(
  db: Db,
  vapidPrivateKey: CryptoKey,
  vapidPublicKeyB64: string,
  vapidSubject: string,
  rpcUrl: string,
  contractId: string
): Promise<number> {
  const activeSubs = (await db`
    SELECT DISTINCT account_id FROM push_subscriptions
  `) as Array<{ account_id: string }>;

  if (activeSubs.length === 0) return 0;

  const nowMs = Date.now();

  for (const { account_id } of activeSubs) {
    try {
      const cooldowns = await fetchQuestCooldowns(
        rpcUrl,
        contractId,
        account_id
      );

      if (cooldowns.length === 0) {
        await db`DELETE FROM quest_cooldowns WHERE account_id = ${account_id}`;
        continue;
      }

      for (const [triggeredAt, quest] of cooldowns) {
        const duration = QUEST_COOLDOWNS_MS[quest];
        if (!duration) continue;
        const expiresAt = triggeredAt + duration;

        await db`
          INSERT INTO quest_cooldowns (account_id, quest, expires_at, notified)
          VALUES (${account_id}, ${quest}, ${expiresAt}, false)
          ON CONFLICT (account_id, quest) DO UPDATE SET
            expires_at = ${expiresAt},
            notified = CASE WHEN quest_cooldowns.expires_at = ${expiresAt} THEN quest_cooldowns.notified ELSE false END
        `;
      }
    } catch (err) {
      console.error(`Error fetching cooldowns for ${account_id}:`, err);
    }
  }

  const expired = (await db`
    SELECT account_id, quest FROM quest_cooldowns
    WHERE expires_at < ${nowMs} AND notified = false
  `) as Array<{ account_id: string; quest: string }>;

  if (expired.length === 0) return 0;

  const accountIds = [...new Set(expired.map(e => e.account_id))];

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

  for (const { account_id, quest } of expired) {
    const accountSubs = subsByAccount.get(account_id) || [];
    const label = QUEST_LABELS[quest] ?? quest;
    const payload = {
      type: 'quest_ready',
      title: 'Quest Ready!',
      body: `Your ${label} quest is ready to complete again`,
      data: { quest }
    };

    for (const sub of accountSubs) {
      const result = await sendPush(
        sub,
        payload,
        vapidPrivateKey,
        vapidPublicKeyB64,
        vapidSubject
      );
      if (result.subscriptionExpired) {
        expiredEndpoints.push(sub.endpoint);
      }
    }
  }

  await db`UPDATE quest_cooldowns SET notified = true WHERE expires_at < ${nowMs} AND notified = false`;

  if (expiredEndpoints.length > 0) {
    await db`DELETE FROM push_subscriptions WHERE endpoint = ANY(${expiredEndpoints})`;
  }

  return expired.length;
}
