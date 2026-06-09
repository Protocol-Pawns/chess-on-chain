import type { KVNamespace } from '@cloudflare/workers-types';

import type { Db } from './db';
import { getAccountStatsBatch } from './db';
import type { AccountStats } from './events';

const ELO_KV_KEY = 'elo_leaderboard';
const PPP_KV_KEY = 'ppp_leaderboard';
const PAGE_SIZE = 100;

export interface RankingEntry {
  rank: number;
  account_id: string;
  elo: number | null;
  ppp: string;
  wins: number;
  losses: number;
  draws: number;
  total_games: number;
}

export interface RankingPage {
  total: number;
  page: number;
  per_page: number;
  total_pages: number;
  entries: RankingEntry[];
}

interface CachedEloEntry {
  rank: number;
  account_id: string;
  elo: number;
}

interface CachedPppEntry {
  account_id: string;
  balance: string;
}

export async function fetchAndCacheLeaderboard(
  kv: KVNamespace,
  rpcUrl: string,
  contractId: string
): Promise<CachedEloEntry[]> {
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

  const entries: CachedEloEntry[] = allEntries.map(([account_id, elo], i) => ({
    rank: i + 1,
    account_id,
    elo
  }));

  await kv.put(ELO_KV_KEY, JSON.stringify(entries), { expirationTtl: 600 });

  return entries;
}

async function fetchPppBalancesByIds(
  rpcUrl: string,
  contractId: string,
  accountIds: string[]
): Promise<Map<string, string>> {
  if (accountIds.length === 0) return new Map();

  const args = btoa(JSON.stringify({ account_ids: accountIds }));
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
        method_name: 'get_ppp_balances_by_ids',
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
    console.error(`RPC error fetching PPP balances: ${json.error.message}`);
    return new Map();
  }

  const resultBytes = json.result?.result;
  if (!resultBytes || resultBytes.length === 0) return new Map();

  const decoded = new TextDecoder().decode(new Uint8Array(resultBytes));
  const pairs: [string, string][] = JSON.parse(decoded);
  return new Map(pairs);
}

async function fetchFastNearTop(): Promise<CachedPppEntry[]> {
  const res = await fetch(
    'https://api.fastnear.com/v1/ft/app.chess-game.near/top'
  );
  if (!res.ok) throw new Error(`FastNear HTTP ${res.status}`);
  const json: { accounts?: CachedPppEntry[] } = await res.json();
  return json.accounts ?? [];
}

async function getCachedPppEntries(kv: KVNamespace): Promise<CachedPppEntry[]> {
  const raw = await kv.get<CachedPppEntry[]>(PPP_KV_KEY, 'json');
  if (raw && raw.length > 0) return raw;

  const entries = await fetchFastNearTop();
  await kv.put(PPP_KV_KEY, JSON.stringify(entries), { expirationTtl: 300 });
  return entries;
}

async function getCachedEloEntries(
  kv: KVNamespace,
  rpcUrl: string,
  contractId: string
): Promise<CachedEloEntry[]> {
  let raw = await kv.get<CachedEloEntry[]>(ELO_KV_KEY, 'json');
  if (!raw || raw.length === 0) {
    raw = await fetchAndCacheLeaderboard(kv, rpcUrl, contractId);
  }
  return raw;
}

export async function getEloRankingPage(
  kv: KVNamespace,
  rpcUrl: string,
  contractId: string,
  db: Db,
  page: number,
  perPage: number,
  dir: 'desc' | 'asc' = 'desc'
): Promise<RankingPage> {
  let eloEntries = await getCachedEloEntries(kv, rpcUrl, contractId);
  if (dir === 'asc') eloEntries = [...eloEntries].reverse();

  const total = eloEntries.length;
  const totalPages = Math.max(1, Math.ceil(total / perPage));
  const safePage = Math.max(1, Math.min(page, totalPages));
  const start = (safePage - 1) * perPage;
  const pageEntries = eloEntries.slice(start, start + perPage);

  const accountIds = pageEntries.map(e => e.account_id);

  const [pppMap, statsList] = await Promise.all([
    fetchPppBalancesByIds(rpcUrl, contractId, accountIds),
    getAccountStatsBatch(db, accountIds)
  ]);

  const statsMap = new Map<string, AccountStats>();
  for (const s of statsList) statsMap.set(s.account_id, s);

  const entries: RankingEntry[] = pageEntries.map((e, i) => {
    const stats = statsMap.get(e.account_id);
    return {
      rank: start + i + 1,
      account_id: e.account_id,
      elo: e.elo,
      ppp: pppMap.get(e.account_id) ?? '0',
      wins: stats?.wins ?? 0,
      losses: stats?.losses ?? 0,
      draws: stats?.draws ?? 0,
      total_games: stats?.total_games ?? 0
    };
  });

  return {
    total,
    page: safePage,
    per_page: perPage,
    total_pages: totalPages,
    entries
  };
}

export async function getPppRankingPage(
  kv: KVNamespace,
  rpcUrl: string,
  contractId: string,
  db: Db,
  page: number,
  perPage: number
): Promise<RankingPage> {
  const [pppEntries, eloEntries] = await Promise.all([
    getCachedPppEntries(kv),
    getCachedEloEntries(kv, rpcUrl, contractId)
  ]);

  const eloMap = new Map<string, number>();
  let rank = 1;
  for (const e of eloEntries) {
    eloMap.set(e.account_id, rank);
    rank++;
  }

  const total = pppEntries.length;
  const totalPages = Math.max(1, Math.ceil(total / perPage));
  const safePage = Math.max(1, Math.min(page, totalPages));
  const start = (safePage - 1) * perPage;
  const pageEntries = pppEntries.slice(start, start + perPage);

  const accountIds = pageEntries.map(e => e.account_id);

  const [eloByIds, statsList] = await Promise.all([
    fetchEloRatingsByIds(rpcUrl, contractId, accountIds),
    getAccountStatsBatch(db, accountIds)
  ]);

  const eloByIdsMap = new Map<string, number>();
  for (const [id, elo] of eloByIds) eloByIdsMap.set(id, elo);

  const statsMap = new Map<string, AccountStats>();
  for (const s of statsList) statsMap.set(s.account_id, s);

  const entries: RankingEntry[] = pageEntries.map((e, i) => {
    const stats = statsMap.get(e.account_id);
    const elo = eloByIdsMap.get(e.account_id);
    return {
      rank: start + i + 1,
      account_id: e.account_id,
      elo: elo ?? null,
      ppp: e.balance,
      wins: stats?.wins ?? 0,
      losses: stats?.losses ?? 0,
      draws: stats?.draws ?? 0,
      total_games: stats?.total_games ?? 0
    };
  });

  return {
    total,
    page: safePage,
    per_page: perPage,
    total_pages: totalPages,
    entries
  };
}

async function fetchEloRatingsByIds(
  rpcUrl: string,
  contractId: string,
  accountIds: string[]
): Promise<[string, number][]> {
  if (accountIds.length === 0) return [];

  const args = btoa(JSON.stringify({ account_ids: accountIds }));
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
        method_name: 'get_elo_ratings_by_ids',
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
    console.error(`RPC error fetching ELO by IDs: ${json.error.message}`);
    return [];
  }

  const resultBytes = json.result?.result;
  if (!resultBytes || resultBytes.length === 0) return [];

  const decoded = new TextDecoder().decode(new Uint8Array(resultBytes));
  return JSON.parse(decoded);
}

export async function getLeaderboardPage(
  kv: KVNamespace,
  rpcUrl: string,
  contractId: string,
  page: number,
  perPage: number
) {
  let raw = await kv.get<CachedEloEntry[]>(ELO_KV_KEY, 'json');

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
