const CG_BASE = 'https://api.coingecko.com/api/v3';

const CONTRACT_CG_ID: Record<string, string> = {
  'wrap.near': 'near',
  'wrap.testnet': 'near'
};

const SYMBOL_CG_ID: Record<string, string> = {
  NEAR: 'near',
  SHITZU: 'shitzu',
  USDC: 'usdc',
  USDT: 'usdt'
};

interface CacheEntry {
  price: number;
  ts: number;
}

const cache = new Map<string, CacheEntry>();
const TTL = 120_000;
const STORAGE_KEY = 'cg_price_cache';

function loadCache(): void {
  try {
    const raw = localStorage.getItem(STORAGE_KEY);
    if (!raw) return;
    const parsed = JSON.parse(raw) as Record<string, CacheEntry>;
    for (const [key, entry] of Object.entries(parsed)) {
      if (Date.now() - entry.ts < TTL) {
        cache.set(key, entry);
      }
    }
  } catch {
    /* localStorage unavailable */
  }
}

function saveCache(): void {
  try {
    const obj: Record<string, CacheEntry> = {};
    for (const [key, entry] of cache) {
      obj[key] = entry;
    }
    localStorage.setItem(STORAGE_KEY, JSON.stringify(obj));
  } catch {
    /* localStorage unavailable */
  }
}

loadCache();

async function fetchPrice(cgId: string): Promise<number | undefined> {
  try {
    const url = `${CG_BASE}/simple/price?ids=${cgId}&vs_currencies=usd`;
    const res = await fetch(url);
    if (!res.ok) return undefined;
    const data = await res.json();
    return data[cgId]?.usd as number | undefined;
  } catch {
    return undefined;
  }
}

function resolveCgId(tokenId: string, symbol?: string): string | undefined {
  return CONTRACT_CG_ID[tokenId] ?? (symbol ? SYMBOL_CG_ID[symbol] : undefined);
}

export async function getTokenPrice(
  tokenId: string,
  symbol?: string
): Promise<number | undefined> {
  const cgId = resolveCgId(tokenId, symbol);
  if (!cgId) return undefined;

  const cached = cache.get(cgId);
  if (cached && Date.now() - cached.ts < TTL) return cached.price;

  const price = await fetchPrice(cgId);
  if (price != null) {
    cache.set(cgId, { price, ts: Date.now() });
    saveCache();
  }
  return price;
}

export function estimateUsd(
  amount: string,
  price: number | undefined
): string | undefined {
  if (price == null) return undefined;
  const n = parseFloat(amount);
  if (isNaN(n)) return undefined;
  return (n * price).toLocaleString('en', {
    style: 'currency',
    currency: 'USD',
    minimumFractionDigits: 2,
    maximumFractionDigits: 2
  });
}
