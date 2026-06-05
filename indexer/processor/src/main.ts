import { getDb } from './db.js';
import { handleEvent, type RawEvent } from './handlers.js';

const BATCH_SIZE = 100;
const POLL_INTERVAL_MS = 2000;

async function processBatch(db: ReturnType<typeof getDb>): Promise<number> {
  const events = (await db`
    SELECT id, trigger_block_height, trigger_block_timestamp, event_type, event_data
    FROM chess_events
    WHERE processed = false
    ORDER BY trigger_block_height ASC, trigger_block_timestamp ASC
    LIMIT ${BATCH_SIZE}
  `) as RawEvent[];

  if (events.length === 0) return 0;

  for (const event of events) {
    await db.begin(async sql => {
      await handleEvent(sql, event);
      await sql`UPDATE chess_events SET processed = true WHERE id = ${event.id}`;
    });
  }

  return events.length;
}

async function main() {
  const databaseUrl = process.env.DATABASE_URL;
  if (!databaseUrl) {
    console.error('DATABASE_URL environment variable is required');
    process.exit(1);
  }

  const db = getDb(databaseUrl);

  console.log('processor started');

  try {
    while (true) {
      try {
        const count = await processBatch(db);
        if (count > 0) {
          console.log(`processed ${count} events`);
        } else {
          await new Promise(r => setTimeout(r, POLL_INTERVAL_MS));
        }
      } catch (err) {
        console.error('error processing batch:', err);
        await new Promise(r => setTimeout(r, POLL_INTERVAL_MS));
      }
    }
  } finally {
    await db.end();
  }
}

main();
