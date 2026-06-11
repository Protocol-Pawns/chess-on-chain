import { getDb } from './db.js';
import { handleEvent, type RawEvent } from './handlers.js';

const BATCH_SIZE = 100;
const POLL_INTERVAL_MS = 2000;

const API_EVENTS_URL = process.env.API_EVENTS_URL;
const PROCESSOR_SECRET = process.env.PROCESSOR_SECRET;

interface ProcessedEvent {
  event_type: string;
  trigger_block_height: string;
  trigger_block_timestamp: string;
  event_data: Record<string, unknown>;
}

async function notifyApi(events: ProcessedEvent[]): Promise<void> {
  if (!API_EVENTS_URL || !PROCESSOR_SECRET || events.length === 0) return;
  try {
    await fetch(`${API_EVENTS_URL}/events/publish`, {
      method: 'POST',
      headers: {
        'Content-Type': 'application/json',
        Authorization: `Bearer ${PROCESSOR_SECRET}`
      },
      body: JSON.stringify({ events }),
      signal: AbortSignal.timeout(5000)
    });
  } catch (err) {
    console.error('failed to notify API:', err);
  }
}

async function processBatch(
  db: ReturnType<typeof getDb>
): Promise<{ count: number; events: ProcessedEvent[] }> {
  const rows = await db`
    SELECT id, trigger_block_height, trigger_block_timestamp, event_type, event_data
    FROM chess_events
    WHERE processed = false
    ORDER BY trigger_block_height::bigint ASC, trigger_block_timestamp::bigint ASC, id ASC
    LIMIT ${BATCH_SIZE}
  `;
  const events = rows.map((r: Record<string, unknown>) => ({
    ...r,
    trigger_block_height: String(r.trigger_block_height),
    trigger_block_timestamp: String(r.trigger_block_timestamp),
    event_data:
      typeof r.event_data === 'string' ? JSON.parse(r.event_data) : r.event_data
  })) as RawEvent[];

  if (events.length === 0) return { count: 0, events: [] };

  const processed: ProcessedEvent[] = [];

  for (const event of events) {
    await db.begin(async sql => {
      await handleEvent(sql, event);
      await sql`UPDATE chess_events SET processed = true WHERE id = ${event.id}`;
    });
    processed.push({
      event_type: event.event_type,
      trigger_block_height: event.trigger_block_height,
      trigger_block_timestamp: event.trigger_block_timestamp,
      event_data: event.event_data
    });
  }

  return { count: events.length, events: processed };
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
        const { count, events } = await processBatch(db);
        if (count > 0) {
          console.log(`processed ${count} events`);
          await notifyApi(events);
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
