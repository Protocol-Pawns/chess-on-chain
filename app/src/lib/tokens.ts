import { FixedNumber } from '@tarnadas/fixed-number';

import { viewTokenFunction, getNearNativeBalance } from '$lib/near/connector';

export const WRAP_NEAR_ID =
  (import.meta.env.VITE_NETWORK_ID || 'mainnet') === 'testnet'
    ? 'wrap.testnet'
    : 'wrap.near';

export function isWrapNear(tokenId: string): boolean {
  return tokenId === WRAP_NEAR_ID;
}

export interface FtMetadata {
  spec: string;
  name: string;
  symbol: string;
  icon: string | null;
  decimals: number;
  reference: string | null;
  reference_hash: string | null;
}

const metadataCache = new Map<string, FtMetadata>();

export async function getTokenMetadata(tokenId: string): Promise<FtMetadata> {
  const cached = metadataCache.get(tokenId);
  if (cached) return cached;

  const meta = await viewTokenFunction<FtMetadata>(tokenId, 'ft_metadata', {});
  metadataCache.set(tokenId, meta);
  return meta;
}

export async function fetchAllMetadata(
  tokenIds: string[]
): Promise<Map<string, FtMetadata>> {
  const missing = tokenIds.filter(id => !metadataCache.has(id));
  if (missing.length > 0) {
    const results = await Promise.allSettled(
      missing.map(async id => {
        const meta = await viewTokenFunction<FtMetadata>(id, 'ft_metadata', {});
        return { id, meta };
      })
    );
    for (const r of results) {
      if (r.status === 'fulfilled') {
        metadataCache.set(r.value.id, r.value.meta);
      }
    }
  }
  const entries: [string, FtMetadata][] = [];
  for (const id of tokenIds) {
    const m = metadataCache.get(id);
    if (m) entries.push([id, m]);
  }
  return new Map(entries);
}

export async function getTokenBalance(
  tokenId: string,
  accountId: string
): Promise<string> {
  return viewTokenFunction<string>(tokenId, 'ft_balance_of', {
    account_id: accountId
  });
}

export function filterAllowedCharacters(value: string): string {
  const dotPos = value.indexOf('.');
  if (dotPos >= 0) {
    return (
      value.substring(0, dotPos) +
      '.' +
      value.substring(dotPos + 1).replace(/[^\d]/g, '')
    );
  }
  return value.replace(/[^\d.]/g, '');
}

export function getNumberAsUInt128(
  value: string,
  decimals: number
): [string, number] {
  value = value.replaceAll(',', '');
  let dotPos = value.indexOf('.');
  if (dotPos === -1) {
    dotPos = value.length;
  }
  return [
    value
      .replace('.', '')
      .slice(0, dotPos + decimals)
      .padEnd(dotPos + decimals, '0'),
    dotPos
  ];
}

export function getFormattedNumber(
  value: string,
  decimals: number
): string | undefined {
  if (value === '' || value === '.') return undefined;
  const [raw, dotPos] = getNumberAsUInt128(value, decimals);
  const formatted = raw.slice(0, dotPos) + '.' + raw.slice(dotPos);
  let result = formatted.replace(/0+$/, '').replace(/^0+/, '');
  if (result === '.') {
    result = '0';
  } else if (result.startsWith('.')) {
    result = '0' + result;
  } else if (result.endsWith('.')) {
    result = result.slice(0, -1);
  }
  return result;
}

export function toFixedNumber(
  value: string,
  decimals: number
): FixedNumber | undefined {
  const formatted = getFormattedNumber(value, decimals);
  if (!formatted) return undefined;
  const [raw] = getNumberAsUInt128(formatted, decimals);
  return new FixedNumber(raw, decimals);
}

export function formatBalance(raw: string, decimals: number): string {
  if (!raw || raw === '0') return '0';
  const fn = new FixedNumber(raw, decimals);
  return fn.format({ maximumFractionDigits: Math.min(decimals, 6) });
}

export async function getCombinedNearBalance(
  tokenId: string,
  accountId: string
): Promise<string | undefined> {
  if (!isWrapNear(tokenId)) return undefined;
  try {
    const [nativeBal, wNearBal] = await Promise.all([
      getNearNativeBalance(accountId),
      getTokenBalance(tokenId, accountId).then(
        b => BigInt(b || '0'),
        () => 0n
      )
    ]);
    return (nativeBal + wNearBal).toString();
  } catch {
    return undefined;
  }
}
