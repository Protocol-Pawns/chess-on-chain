import type { KVNamespace } from '@cloudflare/workers-types';

const KV_KEY = 'elo_leaderboard';
const PAGE_SIZE = 100;

export interface EloLeaderboardEntry {
  rank: number;
  account_id: string;
  elo: number;
}

export interface EloLeaderboardPage {
  total: number;
  page: number;
  per_page: number;
  total_pages: number;
  entries: EloLeaderboardEntry[];
}

export async function fetchAndCacheLeaderboard(
  kv: KVNamespace,
  rpcUrl: string,
  contractId: string
): Promise<EloLeaderboardEntry[]> {
  const allEntries: [string, number][] = [];
  let skip = 0;

  while (true) {
    const result = await fetchEloRatings(rpcUrl, contractId, skip, PAGE_SIZE);
    if (result.length === 0) break;
    allEntries.push(...result);
    if (result.length < PAGE_SIZE) break;
    skip += result.length;
  }

  allEntries.sort((a, b) => b[1] - a[1]);

  const entries: EloLeaderboardEntry[] = allEntries.map(
    ([account_id, elo], i) => ({
      rank: i + 1,
      account_id,
      elo
    })
  );

  await kv.put(KV_KEY, JSON.stringify(entries), { expirationTtl: 600 });

  return entries;
}

export async function getLeaderboardPage(
  kv: KVNamespace,
  rpcUrl: string,
  contractId: string,
  page: number,
  perPage: number
): Promise<EloLeaderboardPage> {
  let raw = await kv.get<EloLeaderboardEntry[]>(KV_KEY, 'json');

  if (!raw || raw.length === 0) {
    raw = await fetchAndCacheLeaderboard(kv, rpcUrl, contractId);
  }

  const total = raw.length;
  const totalPages = Math.max(1, Math.ceil(total / perPage));
  const safePage = Math.max(1, Math.min(page, totalPages));
  const start = (safePage - 1) * perPage;
  const entries = raw.slice(start, start + perPage);

  return {
    total,
    page: safePage,
    per_page: perPage,
    total_pages: totalPages,
    entries
  };
}

async function fetchEloRatings(
  rpcUrl: string,
  contractId: string,
  skip: number,
  limit: number
): Promise<[string, number][]> {
  const args = btoa(JSON.stringify({ skip, limit }));
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
        method_name: 'get_elo_ratings',
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
    throw new Error(`RPC error: ${json.error.message}`);
  }

  const resultBytes = json.result?.result;
  if (!resultBytes || resultBytes.length === 0) return [];

  const decoded = new TextDecoder().decode(new Uint8Array(resultBytes));
  return JSON.parse(decoded);
}
