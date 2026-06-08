import { getDb } from './db.js';

const ACCOUNT_ID = 'app.chess-game.near';
const API_BASE = 'https://tx.main.fastnear.com/v0';
const TX_HASH_PAGE_SIZE = 80;
const TX_DETAIL_BATCH_SIZE = 20;
const DELAY_MS = 1000;

const CHESS_EVENTS = [
  'challenge',
  'accept_challenge',
  'reject_challenge',
  'create_game',
  'play_move',
  'resign_game',
  'cancel_game',
  'place_bet',
  'cancel_bet',
  'lock_bets',
  'resolve_bets'
];

function delay(ms: number): Promise<void> {
  return new Promise(resolve => setTimeout(resolve, ms));
}

interface AccountTxPage {
  account_txs: Array<{ transaction_hash: string }>;
  resume_token: string | null;
  txs_count: number;
}

interface TxDetail {
  transactions: Array<{
    receipts: Array<{
      execution_outcome: {
        block_height: number;
        block_timestamp: string;
        id: string;
        outcome: {
          executor_id: string;
          logs: string[];
          status: Record<string, unknown>;
        };
      };
    }>;
  }>;
}

interface ExtractedEvent {
  id: string;
  trigger_block_height: string;
  trigger_block_timestamp: string;
  event_type: string;
  event_data: string;
}

// eslint-disable-next-line @typescript-eslint/no-explicit-any
async function fetchWithRetry(url: string, body: unknown): Promise<any> {
  for (let attempt = 0; attempt < 5; attempt++) {
    try {
      const res = await fetch(url, {
        method: 'POST',
        headers: { 'content-type': 'application/json' },
        body: JSON.stringify(body)
      });
      if (!res.ok) {
        if (res.status === 429) {
          const wait = Math.min(1000 * Math.pow(2, attempt), 30000);
          console.warn(`rate limited, waiting ${wait}ms...`);
          await delay(wait);
          attempt--;
          continue;
        }
        throw new Error(`HTTP ${res.status}: ${await res.text()}`);
      }
      return await res.json();
    } catch (err) {
      if (attempt === 4) throw err;
      const wait = Math.min(1000 * Math.pow(2, attempt), 30000);
      console.warn(`request failed (attempt ${attempt + 1}), retrying in ${wait}ms:`, err);
      await delay(wait);
    }
  }
}

function extractEvents(detail: TxDetail): ExtractedEvent[] {
  const events: ExtractedEvent[] = [];

  for (const tx of detail.transactions ?? []) {
    for (const receipt of tx.receipts ?? []) {
      const eo = receipt.execution_outcome;
      if (!eo?.outcome) continue;
      if (eo.outcome.executor_id !== ACCOUNT_ID) continue;
      if ('Failure' in eo.outcome.status) continue;

      const blockHeight = String(eo.block_height);
      const blockTimestamp = String(eo.block_timestamp);
      const receiptId = eo.id;

      let eventIndex = 0;
      for (const log of eo.outcome.logs ?? []) {
        const trimmed = (log || '').trim();
        if (!trimmed.startsWith('EVENT_JSON:')) continue;

        try {
          const parsed = JSON.parse(trimmed.slice('EVENT_JSON:'.length));
          if (parsed.standard !== 'chess-game') continue;
          if (!CHESS_EVENTS.includes(parsed.event)) continue;

          events.push({
            id: `${receiptId}_${parsed.event}_${eventIndex}`,
            trigger_block_height: blockHeight,
            trigger_block_timestamp: blockTimestamp,
            event_type: parsed.event,
            event_data: JSON.stringify(parsed.data ?? {})
          });
          eventIndex++;
        } catch {
          // skip malformed JSON
        }
      }
    }
  }

  return events;
}

const DRY_RUN = process.argv.includes('--dry-run');

async function fetchAllTxHashes(): Promise<string[]> {
  const allHashes: string[] = [];
  let resumeToken: string | undefined;
  let totalOnAccount = 0;

  console.log('phase 1: fetching all tx hashes...');

  while (true) {
    const pageBody: Record<string, unknown> = {
      account_id: ACCOUNT_ID,
      limit: TX_HASH_PAGE_SIZE,
      desc: false
    };
    if (resumeToken) pageBody.resume_token = resumeToken;

    await delay(DELAY_MS);
    const page = (await fetchWithRetry(`${API_BASE}/account`, pageBody)) as AccountTxPage;
    const hashes = page.account_txs?.map(t => t.transaction_hash) ?? [];

    if (hashes.length === 0) break;

    totalOnAccount = page.txs_count;
    allHashes.push(...hashes);
    console.log(`  fetched ${allHashes.length}/${totalOnAccount} tx hashes`);

    resumeToken = page.resume_token ?? undefined;
    if (!resumeToken) break;
  }

  console.log(`  total: ${allHashes.length} tx hashes`);
  return allHashes;
}

async function fetchAllTxDetails(txHashes: string[]): Promise<TxDetail[]> {
  const allDetails: TxDetail[] = [];
  const totalBatches = Math.ceil(txHashes.length / TX_DETAIL_BATCH_SIZE);

  console.log(`phase 2: fetching tx details (${totalBatches} batches)...`);

  for (let i = 0; i < txHashes.length; i += TX_DETAIL_BATCH_SIZE) {
    const batch = txHashes.slice(i, i + TX_DETAIL_BATCH_SIZE);

    await delay(DELAY_MS);
    const detail = (await fetchWithRetry(`${API_BASE}/transactions`, {
      tx_hashes: batch
    })) as TxDetail;

    allDetails.push(detail);

    const batchNum = Math.floor(i / TX_DETAIL_BATCH_SIZE) + 1;
    console.log(`  batch ${batchNum}/${totalBatches} (${batch.length} txs)`);
  }

  return allDetails;
}

function extractAllEvents(details: TxDetail[]): ExtractedEvent[] {
  console.log('phase 3: extracting events...');

  const events: ExtractedEvent[] = [];
  for (const detail of details) {
    events.push(...extractEvents(detail));
  }

  events.sort((a, b) => {
    const h = BigInt(a.trigger_block_height) - BigInt(b.trigger_block_height);
    if (h !== 0n) return Number(h);
    return Number(BigInt(a.trigger_block_timestamp) - BigInt(b.trigger_block_timestamp));
  });

  const byType = new Map<string, number>();
  for (const e of events) {
    byType.set(e.event_type, (byType.get(e.event_type) ?? 0) + 1);
  }

  console.log(`  ${events.length} events extracted:`);
  for (const [type, count] of byType) {
    console.log(`    ${type}: ${count}`);
  }

  return events;
}

async function writeEvents(db: ReturnType<typeof getDb>, events: ExtractedEvent[]): Promise<void> {
  console.log(`phase 4: writing ${events.length} events to db...`);

  const INSERT_BATCH = 100;
  for (let i = 0; i < events.length; i += INSERT_BATCH) {
    const batch = events.slice(i, i + INSERT_BATCH);
    await db.begin(async sql => {
      for (const event of batch) {
        await sql`
          INSERT INTO chess_events (id, trigger_block_height, trigger_block_timestamp, event_type, event_data)
          VALUES (${event.id}, ${event.trigger_block_height}, ${event.trigger_block_timestamp}, ${event.event_type}, ${event.event_data})
          ON CONFLICT (id) DO NOTHING
        `;
      }
    });
    console.log(`  wrote ${Math.min(i + INSERT_BATCH, events.length)}/${events.length}`);
  }
}

async function main() {
  const databaseUrl = process.env.DATABASE_URL;
  if (!databaseUrl && !DRY_RUN) {
    console.error('DATABASE_URL environment variable is required');
    process.exit(1);
  }

  const db = databaseUrl ? getDb(databaseUrl) : null;

  try {
    if (DRY_RUN) console.log('DRY RUN - no writes to db');

    const txHashes = await fetchAllTxHashes();
    const details = await fetchAllTxDetails(txHashes);
    const events = extractAllEvents(details);

    if (DRY_RUN) {
      console.log('events:', JSON.stringify(events, null, 2));
    } else {
      await writeEvents(db!, events);
    }

    console.log(`done. ${txHashes.length} txs, ${events.length} events${DRY_RUN ? ' (dry run)' : ''}`);
  } finally {
    await db?.end();
  }
}

main();
