import type { AppEnv } from './types';
type DOStub = ReturnType<AppEnv['Bindings']['SSE_HUB']['get']>;

function forwardResponse(res: Awaited<ReturnType<DOStub['fetch']>>): Response {
  const headers = new Headers();
  res.headers.forEach((v, k) => headers.set(k, v));
  return new Response(res.body as unknown as BodyInit, {
    status: res.status,
    statusText: res.statusText,
    headers
  });
}

interface GameLookup {
  white_type: string;
  white_value: string;
  black_type: string;
  black_value: string | null;
}

interface ChallengeLookup {
  challenger: string;
  challenged: string;
}

function gameIdFromData(data: Record<string, unknown>): string | null {
  if (!data.game_id) return null;
  return typeof data.game_id === 'string'
    ? data.game_id
    : JSON.stringify(data.game_id);
}

function resolveTargets(
  eventType: string,
  data: Record<string, unknown>,
  games: Map<string, GameLookup>,
  challenges: Map<string, ChallengeLookup>
): string[] {
  switch (eventType) {
    case 'challenge': {
      return [data.challenged as string];
    }
    case 'accept_challenge': {
      const challengeId = data.challenge_id as string;
      const challenge = challenges.get(challengeId);
      if (!challenge) return [];
      const targets = [challenge.challenger];
      if (challenge.challenged) targets.push(challenge.challenged);
      return targets;
    }
    case 'reject_challenge': {
      const challengeId = data.challenge_id as string;
      const challenge = challenges.get(challengeId);
      if (!challenge) return [];
      return [challenge.challenger];
    }
    case 'play_move': {
      const gid = gameIdFromData(data);
      if (!gid) return [];
      const game = games.get(gid);
      if (!game) return [];
      const outcome = data.outcome as Record<string, unknown> | null;
      const accounts: string[] = [];
      if (game.white_type === 'Human') accounts.push(game.white_value);
      if (game.black_type === 'Human' && game.black_value)
        accounts.push(game.black_value);
      if (outcome) return accounts;
      const color = data.color as string;
      if (color === 'White' && game.black_type === 'Human' && game.black_value)
        return [game.black_value];
      if (color === 'Black' && game.white_type === 'Human')
        return [game.white_value];
      return accounts;
    }
    case 'resign_game': {
      const gid = gameIdFromData(data);
      if (!gid) return [];
      const game = games.get(gid);
      if (!game) return [];
      const resigner = data.resigner as string;
      const accounts: string[] = [];
      if (game.white_type === 'Human' && game.white_value !== resigner)
        accounts.push(game.white_value);
      if (
        game.black_type === 'Human' &&
        game.black_value &&
        game.black_value !== resigner
      )
        accounts.push(game.black_value);
      return accounts;
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
      return accounts;
    }
    case 'create_game': {
      const gid = gameIdFromData(data);
      if (!gid) return [];
      const game = games.get(gid);
      if (!game) return [];
      const accounts: string[] = [];
      if (game.white_type === 'Human') accounts.push(game.white_value);
      if (game.black_type === 'Human' && game.black_value)
        accounts.push(game.black_value);
      return accounts;
    }
    default:
      return [];
  }
}

export function registerSSERoutes(
  app: import('@hono/zod-openapi').OpenAPIHono<AppEnv>
) {
  app.get('/events', async c => {
    const id = c.env.SSE_HUB.idFromName('global');
    const accounts = c.req.query('account');
    if (!accounts) {
      return c.json({ error: 'Missing account param' }, 400);
    }
    const params = new URLSearchParams();
    for (const a of Array.isArray(accounts) ? accounts : [accounts]) {
      params.append('account', a);
    }
    const doRes = await c.env.SSE_HUB.get(id).fetch(
      `https://do/sse-hub/subscribe?${params.toString()}`
    );
    return forwardResponse(doRes);
  });

  app.post('/events/publish', async c => {
    const auth = c.req.header('Authorization');
    if (!auth || auth !== `Bearer ${c.env.PROCESSOR_SECRET}`) {
      return c.json({ error: 'Unauthorized' }, 401);
    }

    const body = await c.req.json<{
      events: Array<{
        event_type: string;
        trigger_block_height: string | number;
        trigger_block_timestamp: string | number;
        event_data: Record<string, unknown>;
      }>;
    }>();

    if (!body.events || body.events.length === 0) {
      return c.json({ ok: true, delivered: 0 });
    }

    const db = c.get('DB');

    const challengeIds: string[] = [];
    const gameIds: string[] = [];

    for (const e of body.events) {
      const data = e.event_data;
      if (
        e.event_type === 'accept_challenge' ||
        e.event_type === 'reject_challenge'
      ) {
        if (data.challenge_id) challengeIds.push(data.challenge_id as string);
      }
      if (
        ['play_move', 'resign_game', 'cancel_game', 'create_game'].includes(
          e.event_type
        )
      ) {
        const gid = gameIdFromData(data);
        if (gid) gameIds.push(gid);
      }
    }

    const games = new Map<string, GameLookup>();
    if (gameIds.length > 0) {
      const rows = (await db`
        SELECT game_id, white_type, white_value, black_type, black_value
        FROM games WHERE game_id = ANY(${gameIds})
      `) as Array<GameLookup & { game_id: string }>;
      for (const r of rows) games.set(r.game_id, r);
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

    const publishEvents: Array<{
      event_type: string;
      trigger_block_height: number;
      trigger_block_timestamp: number;
      event_data: Record<string, unknown>;
      target_accounts: string[];
    }> = [];

    for (const e of body.events) {
      const targets = resolveTargets(
        e.event_type,
        e.event_data,
        games,
        challenges
      );
      if (targets.length === 0) continue;
      publishEvents.push({
        event_type: e.event_type,
        trigger_block_height: Number(e.trigger_block_height),
        trigger_block_timestamp: Number(e.trigger_block_timestamp),
        event_data: e.event_data,
        target_accounts: targets
      });
    }

    if (publishEvents.length === 0) {
      return c.json({ ok: true, delivered: 0 });
    }

    const id = c.env.SSE_HUB.idFromName('global');
    const stub = c.env.SSE_HUB.get(id);
    const doRes = await stub.fetch('https://do/sse-hub/publish', {
      method: 'POST',
      headers: { 'Content-Type': 'application/json' },
      body: JSON.stringify({ events: publishEvents })
    });
    return forwardResponse(doRes);
  });
}
