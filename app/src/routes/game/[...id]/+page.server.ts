import type { PageServerLoad } from './$types';

import { api } from '$lib/api/client';
import { parseGamePath } from '$lib/game';

const APP_ORIGIN = 'https://app.protocol-pawns.com';
const API_ORIGIN = 'https://api.protocol-pawns.com';

export const load: PageServerLoad = async ({ params, fetch }) => {
  const id = params.id ?? '';
  const gameId = parseGamePath(id);
  const gameIdStr = JSON.stringify(gameId);
  const path = `/game/${id}`;

  let game: Awaited<ReturnType<typeof api.game>> | null = null;
  try {
    const res = await fetch(
      `${API_ORIGIN}/game/${encodeURIComponent(gameIdStr)}`
    );
    if (res.ok) game = await res.json();
  } catch {
    // Non-critical: fall back to default meta tags.
  }

  if (!game) {
    return {
      meta: {
        title: 'Protocol Pawns',
        description:
          'Fully decentralized on-chain chess on NEAR Protocol. Play vs AI or challenge other wallets.',
        url: `${APP_ORIGIN}${path}`,
        image: `${APP_ORIGIN}/favicon.png`
      }
    };
  }

  const whiteName =
    game.white.type === 'Human' ? game.white.value : `AI (${game.white.value})`;
  const blackName = game.black
    ? game.black.type === 'Human'
      ? game.black.value
      : `AI (${game.black.value})`
    : 'Unknown';

  const resultText = (() => {
    if (!game.outcome) {
      if (game.status === 'cancelled') return 'Game cancelled';
      return game.status === 'in_progress'
        ? 'In progress'
        : 'Waiting for opponent';
    }
    if (game.outcome.result === 'Stalemate') return 'Draw — Stalemate';
    if (game.resigner) return `${game.outcome.color} wins by resignation`;
    return `${game.outcome.color} wins by checkmate`;
  })();

  const title = `${whiteName} vs ${blackName} — Protocol Pawns`;
  const description = `${resultText}. Watch this on-chain chess game on Protocol Pawns.`;
  const previewUrl = `${API_ORIGIN}/game/${encodeURIComponent(gameIdStr)}/preview.png`;

  return {
    game,
    meta: {
      title,
      description,
      url: `${APP_ORIGIN}${path}`,
      image: previewUrl
    }
  };
};
