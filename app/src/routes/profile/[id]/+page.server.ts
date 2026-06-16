import type { PageServerLoad } from './$types';

import { truncateAddr } from '$lib/format';

const APP_ORIGIN = 'https://app.protocol-pawns.com';
const API_ORIGIN = 'https://api.protocol-pawns.com';

interface AccountStats {
  account_id: string;
  wins: number;
  losses: number;
  draws: number;
  total_games: number;
}

export const load: PageServerLoad = async ({ params, fetch }) => {
  const accountId = params.id ?? '';
  const path = `/profile/${accountId}`;

  let stats: AccountStats | null = null;
  try {
    const res = await fetch(
      `${API_ORIGIN}/account/${encodeURIComponent(accountId)}/stats`
    );
    if (res.ok) stats = await res.json();
  } catch {
    // Non-critical: fall back to default meta tags.
  }

  const displayName = truncateAddr(accountId);

  if (!stats) {
    return {
      meta: {
        title: `${displayName} — Protocol Pawns`,
        description: `View ${displayName}'s chess profile on Protocol Pawns.`,
        url: `${APP_ORIGIN}${path}`,
        image: `${APP_ORIGIN}/favicon.png`
      }
    };
  }

  const winRate =
    stats.total_games > 0
      ? Math.round((stats.wins / stats.total_games) * 1000) / 10
      : 0;

  const title = `${displayName} — Protocol Pawns`;
  const description = `${displayName} · ${stats.wins}W / ${stats.losses}L / ${stats.draws}D · ${winRate}% win rate on Protocol Pawns.`;
  const previewUrl = `${API_ORIGIN}/account/${encodeURIComponent(accountId)}/preview.png`;

  return {
    meta: {
      title,
      description,
      url: `${APP_ORIGIN}${path}`,
      image: previewUrl
    }
  };
};
