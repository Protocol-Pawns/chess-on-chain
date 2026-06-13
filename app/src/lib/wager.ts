import { getTokenPrice, estimateUsd, isStablecoin } from '$lib/prices';
import { formatBalance, getTokenMetadata, isWrapNear } from '$lib/tokens';

export interface WagerDisplay {
  amount: string;
  symbol: string;
  usd?: string;
}

export async function formatWager(
  rawAmount: string,
  tokenId: string
): Promise<WagerDisplay> {
  let decimals = 24;
  let symbol = 'NEAR';

  if (!isWrapNear(tokenId)) {
    try {
      const meta = await getTokenMetadata(tokenId);
      decimals = meta.decimals;
      symbol = meta.symbol;
    } catch {
      return { amount: rawAmount, symbol: '' };
    }
  }

  const formatted = formatBalance(rawAmount, decimals);
  const result: WagerDisplay = { amount: formatted, symbol };

  if (!isStablecoin(symbol)) {
    try {
      const price = await getTokenPrice(tokenId, symbol);
      if (price != null) {
        result.usd = estimateUsd(formatted.replace(/,/g, ''), price);
      }
    } catch {
      /* price fetch failed, skip USD */
    }
  }

  return result;
}

export function formatWagerText(d: WagerDisplay | null): string {
  if (!d) return '';
  return `${d.amount} ${d.symbol}${d.usd ? ` (${d.usd})` : ''}`;
}
