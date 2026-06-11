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

function normalizePlayer(p: Record<string, unknown>): {
  type: string;
  value: string;
} {
  if ('type' in p && 'value' in p) {
    return { type: p.type as string, value: p.value as string };
  }
  const [[type, value]] = Object.entries(p);
  return { type, value: value as string };
}

function normalizeOutcome(o: Record<string, unknown>): {
  result: string;
  color?: string;
} {
  if ('result' in o)
    return { result: o.result as string, color: o.color as string | undefined };
  const [[key, val]] = Object.entries(o);
  if (key === 'Stalemate') return { result: 'Stalemate' };
  return { result: key, color: val as string };
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
    const white = normalizePlayer(d.white as Record<string, unknown>);
    const black = normalizePlayer(d.black as Record<string, unknown>);

    await sql`
      INSERT INTO games (game_id, trigger_block_height, white_type, white_value, black_type, black_value, board, fen, created_at)
      VALUES (${gid}, ${event.trigger_block_height}, ${white.type}, ${white.value}, ${black.type}, ${black.value}, ${JSON.stringify(board)}::jsonb, ${fen}, ${event.trigger_block_timestamp})
      ON CONFLICT (game_id) DO NOTHING
    `;
  },

  async play_move(sql, event) {
    const d = event.event_data;
    const gid = gameId(d);
    const board = d.board as string[] | undefined;
    if (!board) {
      console.warn(`skipping play_move without board (${event.id})`);
      return;
    }
    const color = d.color as string;
    const outcome = d.outcome
      ? normalizeOutcome(d.outcome as Record<string, unknown>)
      : null;
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
        moves = moves || jsonb_build_array(
          jsonb_build_object('color', ${color}, 'mv', ${d.mv}, 'board', ${JSON.stringify(board)}::jsonb, 'fen', ${fen})
        )
      WHERE game_id = ${gid}
    `;

    if (outcome != null) {
      await sql`
        UPDATE games SET
          outcome = ${outcomeJson}::jsonb,
          status = 'finished',
          finished_at = ${event.trigger_block_timestamp}
        WHERE game_id = ${gid}
      `;
      await insertAccountFinishedGames(sql, gid);
    }
  },

  async resign_game(sql, event) {
    const d = event.event_data;
    const gid = gameId(d);
    const outcome = d.outcome
      ? normalizeOutcome(d.outcome as Record<string, unknown>)
      : {
          result: 'Victory',
          color: d.resigner === 'White' ? 'Black' : 'White'
        };

    await sql`
      UPDATE games SET
        outcome = ${JSON.stringify(outcome)}::jsonb,
        resigner = ${d.resigner},
        status = 'finished',
        finished_at = ${event.trigger_block_timestamp}
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
      INSERT INTO challenges (challenge_id, challenger, challenged, wager_token, wager_amount, created_at)
      VALUES (${d.id}, ${d.challenger}, ${d.challenged}, ${wager?.[0] ?? null}, ${wager?.[1] ?? null}, ${event.trigger_block_timestamp})
      ON CONFLICT DO NOTHING
    `;
  },

  async accept_challenge(sql, event) {
    const d = event.event_data;

    await sql`
      UPDATE challenges SET
        status = 'accepted',
        game_id = ${gameId(d)},
        resolved_at = ${event.trigger_block_timestamp}
      WHERE challenge_id = ${d.challenge_id} AND status = 'pending'
    `;
  },

  async reject_challenge(sql, event) {
    const d = event.event_data;

    await sql`
      UPDATE challenges SET
        status = 'rejected',
        resolved_at = ${event.trigger_block_timestamp}
      WHERE challenge_id = ${d.challenge_id} AND status = 'pending'
    `;
  },

  async place_bet(sql, event) {
    const d = event.event_data;
    const betKey = `${d.bettor}_${d.players[0]}_${d.players[1]}_${d.token_id}`;

    await sql`
      INSERT INTO bets (bet_key, bettor, player_0, player_1, token_id, amount, winner, created_at)
      VALUES (${betKey}, ${d.bettor}, ${d.players[0]}, ${d.players[1]}, ${d.token_id}, ${d.amount}, ${d.winner}, ${event.trigger_block_timestamp})
      ON CONFLICT DO NOTHING
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
        locked_at = ${event.trigger_block_timestamp}
      WHERE player_0 = ${d.players[0]}
        AND player_1 = ${d.players[1]}
        AND status = 'pending'
    `;
  },

  async resolve_bets(sql, event) {
    const d = event.event_data;
    const gid = gameId(d);
    const feeBps = (d.fee_bps as number) ?? 0;

    await sql`
      UPDATE bets SET
        status = 'resolved',
        resolved_at = ${event.trigger_block_timestamp}
      WHERE game_id = ${gid}
        AND status = 'locked'
    `;

    const outcome = normalizeOutcome(d.outcome as Record<string, unknown>);
    const bets = await sql`
      SELECT id, bettor, token_id, amount, winner FROM bets
      WHERE game_id = ${gid} AND status = 'resolved'
    `;

    const byToken = new Map<
      string,
      Array<{ id: number; bettor: string; amount: string; winner: string }>
    >();
    for (const b of bets) {
      const row = b as {
        id: number;
        bettor: string;
        amount: string;
        winner: string;
        token_id: string;
      };
      let arr = byToken.get(row.token_id);
      if (!arr) {
        arr = [];
        byToken.set(row.token_id, arr);
      }
      arr.push(row);
    }

    const payouts: Array<{ id: number; amount: string }> = [];

    if (outcome.result === 'Stalemate') {
      for (const [, tokenBets] of byToken) {
        for (const b of tokenBets) {
          payouts.push({ id: b.id, amount: b.amount });
        }
      }
    } else if (outcome.result === 'Victory' && outcome.color) {
      const gameRows =
        await sql`SELECT white_type, white_value, black_type, black_value FROM games WHERE game_id = ${gid}`;
      if (gameRows.length > 0) {
        const game = gameRows[0] as Record<string, string>;
        const winnerId =
          outcome.color === 'White' ? game.white_value : game.black_value;

        for (const [, tokenBets] of byToken) {
          const winners = tokenBets.filter(b => b.winner === winnerId);
          const losers = tokenBets.filter(b => b.winner !== winnerId);
          const totalWinner = winners.reduce(
            (s, b) => s + BigInt(b.amount),
            0n
          );
          const totalLoserRaw = losers.reduce(
            (s, b) => s + BigInt(b.amount),
            0n
          );
          const fee = (totalLoserRaw * BigInt(feeBps)) / 10_000n;
          const totalLoser = totalLoserRaw - fee;

          let totalWinAmount = 0n;
          if (totalWinner > 0n) {
            for (const b of winners) {
              const amt = BigInt(b.amount);
              let winAmount = (totalLoser * amt) / totalWinner;
              if (winAmount > amt) winAmount = amt;
              totalWinAmount += winAmount;
              payouts.push({
                id: b.id,
                amount: String(winAmount + amt)
              });
            }
          }

          const totalRefund = totalLoser - totalWinAmount;
          if (totalRefund > 0n && totalLoser > 0n) {
            for (const b of losers) {
              const refundAmount =
                (totalRefund * BigInt(b.amount)) / totalLoser;
              payouts.push({
                id: b.id,
                amount: String(refundAmount)
              });
            }
          }
        }
      }
    }

    for (const p of payouts) {
      await sql`
        UPDATE bets SET payout = ${p.amount}
        WHERE id = ${p.id}
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
