import { getDb } from './db.js';
import { handleEvent, type RawEvent } from './handlers.js';

const BATCH_SIZE = 500;

async function backfill() {
  const databaseUrl = process.env.DATABASE_URL;
  if (!databaseUrl) {
    console.error('DATABASE_URL environment variable is required');
    process.exit(1);
  }

  const db = getDb(databaseUrl);

  try {
    console.log('truncating derived tables...');
    await db`TRUNCATE games, game_moves, challenges, account_finished_games, bets CASCADE`;
    await db`UPDATE chess_events SET processed = false`;

    const totalResult =
      await db`SELECT COUNT(*) AS total FROM chess_events WHERE processed = false`;
    const total = Number((totalResult[0] as Record<string, string>).total);
    console.log(`reprocessing ${total} events`);

    let processed = 0;
    while (true) {
      const events = (await db`
        SELECT id, trigger_block_height, trigger_block_timestamp, event_type, event_data
        FROM chess_events
        WHERE processed = false
        ORDER BY trigger_block_height ASC, trigger_block_timestamp ASC, id ASC
        LIMIT ${BATCH_SIZE}
      `) as RawEvent[];

      if (events.length === 0) break;

      for (const event of events) {
        await db.begin(async sql => {
          await handleEvent(sql, event);
          await sql`UPDATE chess_events SET processed = true WHERE id = ${event.id}`;
        });
      }

      processed += events.length;
      console.log(`processed ${processed}/${total}`);
    }

    console.log('backfill complete');
  } finally {
    await db.end();
  }
}

backfill();
