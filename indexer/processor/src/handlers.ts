import { asciiBoardToFen } from './fen.js';

export interface RawEvent {
  id: string;
  trigger_block_height: string;
  trigger_block_timestamp: string;
  event_type: string;
  // eslint-disable-next-line @typescript-eslint/no-explicit-any
  event_data: Record<string, any>;
}

type Queryable = {
  // eslint-disable-next-line @typescript-eslint/no-explicit-any
  (strings: TemplateStringsArray, ...values: unknown[]): any;
};

type EventHandler = (sql: Queryable, event: RawEvent) => Promise<void>;

// eslint-disable-next-line @typescript-eslint/no-explicit-any
function gameId(data: Record<string, any>): string {
  return JSON.stringify(data.game_id);
}

function toDate(ts: string): Date {
  return new Date(Number(ts));
}

async function insertAccountFinishedGames(sql: Queryable, gameId: string) {
  const rows = await sql`
    SELECT white_type, white_value, black_type, black_value
    FROM games WHERE game_id = ${gameId}
  `;
  if (rows.length === 0) return;
  const game = rows[0] as Record<string, string>;

  if (game.white_type === 'Human') {
    await sql`
      INSERT INTO account_finished_games (account_id, game_id)
      VALUES (${game.white_value}, ${gameId})
      ON CONFLICT DO NOTHING
    `;
  }
  if (game.black_type === 'Human') {
    await sql`
      INSERT INTO account_finished_games (account_id, game_id)
      VALUES (${game.black_value}, ${gameId})
      ON CONFLICT DO NOTHING
    `;
  }
}

const handlers: Record<string, EventHandler> = {
  async create_game(sql, event) {
    const d = event.event_data;
    const gid = gameId(d);
    const board = d.board as string[];
    const fen = asciiBoardToFen(board) + ' w - - 0 1';

    await sql`
      INSERT INTO games (game_id, trigger_block_height, white_type, white_value, black_type, black_value, board, fen)
      VALUES (${gid}, ${event.trigger_block_height}, ${d.white.type}, ${d.white.value}, ${d.black.type}, ${d.black.value}, ${JSON.stringify(board)}::jsonb, ${fen})
      ON CONFLICT (game_id) DO NOTHING
    `;
  },

  async play_move(sql, event) {
    const d = event.event_data;
    const gid = gameId(d);
    const board = d.board as string[];
    const color = d.color as string;
    const outcome = d.outcome ?? null;
    const outcomeJson = JSON.stringify(outcome);
    const fenBase = asciiBoardToFen(board);
    const activeColor = color === 'White' ? 'b' : 'w';

    const moveNumRows = await sql`
      SELECT COALESCE(MAX(move_number), 0) + 1 AS move_num
      FROM game_moves WHERE game_id = ${gid}
    `;
    const moveNum = Number((moveNumRows[0] as Record<string, string>).move_num);
    const fullmove = Math.floor((moveNum + 1) / 2) + 1;
    const fen = `${fenBase} ${activeColor} - - 0 ${fullmove}`;

    await sql`
      INSERT INTO game_moves (id, game_id, move_number, color, move_notation, fen, outcome, trigger_block_height, trigger_block_timestamp)
      VALUES (${event.id}, ${gid}, ${moveNum}, ${color}, ${d.mv}, ${fen}, ${outcomeJson}::jsonb, ${event.trigger_block_height}, ${event.trigger_block_timestamp})
      ON CONFLICT (id) DO NOTHING
    `;

    await sql`
      UPDATE games SET
        board = COALESCE(${JSON.stringify(board)}::jsonb, board),
        fen = ${fen},
        moves = moves || ${JSON.stringify([{ color, mv: d.mv, board, fen }])}::jsonb
      WHERE game_id = ${gid}
    `;

    if (outcome != null) {
      await sql`
        UPDATE games SET
          outcome = ${outcomeJson}::jsonb,
          status = 'finished',
          finished_at = ${toDate(event.trigger_block_timestamp)}
        WHERE game_id = ${gid}
      `;
      await insertAccountFinishedGames(sql, gid);
    }
  },

  async resign_game(sql, event) {
    const d = event.event_data;
    const gid = gameId(d);

    await sql`
      UPDATE games SET
        outcome = ${JSON.stringify(d.outcome)}::jsonb,
        resigner = ${d.resigner},
        status = 'finished',
        finished_at = ${toDate(event.trigger_block_timestamp)}
      WHERE game_id = ${gid}
    `;
    await insertAccountFinishedGames(sql, gid);
  },

  async cancel_game(sql, event) {
    const d = event.event_data;
    const gid = gameId(d);

    await sql`UPDATE games SET status = 'cancelled' WHERE game_id = ${gid}`;
    await sql`DELETE FROM account_finished_games WHERE game_id = ${gid}`;
  },

  async challenge(sql, event) {
    const d = event.event_data;
    const wager = d.wager;

    await sql`
      INSERT INTO challenges (id, challenger, challenged, wager_token, wager_amount)
      VALUES (${d.id}, ${d.challenger}, ${d.challenged}, ${wager?.[0] ?? null}, ${wager?.[1] ?? null})
      ON CONFLICT (id) DO NOTHING
    `;
  },

  async accept_challenge(sql, event) {
    const d = event.event_data;

    await sql`
      UPDATE challenges SET
        status = 'accepted',
        game_id = ${gameId(d)},
        resolved_at = ${toDate(event.trigger_block_timestamp)}
      WHERE id = ${d.challenge_id}
    `;
  },

  async reject_challenge(sql, event) {
    const d = event.event_data;

    await sql`
      UPDATE challenges SET
        status = 'rejected',
        resolved_at = ${toDate(event.trigger_block_timestamp)}
      WHERE id = ${d.challenge_id}
    `;
  },

  async place_bet(sql, event) {
    const d = event.event_data;
    const betId = `${d.bettor}_${d.players[0]}_${d.players[1]}_${d.token_id}`;

    await sql`
      INSERT INTO bets (id, bettor, player_0, player_1, token_id, amount, winner)
      VALUES (${betId}, ${d.bettor}, ${d.players[0]}, ${d.players[1]}, ${d.token_id}, ${d.amount}, ${d.winner})
      ON CONFLICT (id) DO UPDATE SET
        amount = EXCLUDED.amount
    `;
  },

  async cancel_bet(sql, event) {
    const d = event.event_data;

    await sql`
      DELETE FROM bets
      WHERE bettor = ${d.bettor}
        AND player_0 = ${d.players[0]}
        AND player_1 = ${d.players[1]}
        AND token_id = ${d.token_id}
        AND status = 'pending'
    `;
  },

  async lock_bets(sql, event) {
    const d = event.event_data;
    const gid = gameId(d);

    await sql`
      UPDATE bets SET
        status = 'locked',
        game_id = ${gid},
        locked_at = ${toDate(event.trigger_block_timestamp)}
      WHERE player_0 = ${d.players[0]}
        AND player_1 = ${d.players[1]}
        AND status = 'pending'
    `;
  },

  async resolve_bets(sql, event) {
    const d = event.event_data;
    const gid = gameId(d);

    await sql`
      UPDATE bets SET
        status = 'resolved',
        resolved_at = ${toDate(event.trigger_block_timestamp)}
      WHERE game_id = ${gid}
        AND status = 'locked'
    `;

    for (const p of (d.payouts ?? []) as Array<{
      bettor: string;
      token_id: string;
      amount: string;
    }>) {
      await sql`
        UPDATE bets SET payout = ${p.amount}
        WHERE game_id = ${gid}
          AND bettor = ${p.bettor}
          AND token_id = ${p.token_id}
          AND status = 'resolved'
      `;
    }
  }
};

export async function handleEvent(
  sql: Queryable,
  event: RawEvent
): Promise<void> {
  const handler = handlers[event.event_type];
  if (!handler) {
    console.warn(`unknown event_type: ${event.event_type} (${event.id})`);
    return;
  }
  await handler(sql, event);
}
