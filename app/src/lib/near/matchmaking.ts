import { writable } from 'svelte/store';

import { accountStore } from './account';
import { contract } from './connector';

import { browser } from '$app/environment';
import { goto } from '$app/navigation';
import { gameUrl } from '$lib/game';
import type { GameId } from '$lib/game';
import { subscribe } from '$lib/sse';
import type { SSEEventData } from '$lib/sse';
import { showToast, decodeSuccessValue } from '$lib/toast';
import { WRAP_NEAR_ID } from '$lib/tokens';

export const searching = writable(false);
export const searchMinElo = writable(0);
export const searchMaxElo = writable(0);
export const searchWager = writable(0);

let unsubSSE: (() => void) | null = null;
let currentAccount: string | undefined;

function startSSE() {
  if (unsubSSE) return;
  unsubSSE = subscribe('create_game', handleCreateGame);
}

function stopSSE() {
  if (unsubSSE) {
    unsubSSE();
    unsubSSE = null;
  }
}

function extractAccount(player: unknown): string | null {
  if (typeof player !== 'object' || player === null) return null;
  const p = player as Record<string, unknown>;
  if (p.type === 'Human' && typeof p.value === 'string') return p.value;
  return null;
}

function handleCreateGame(event: SSEEventData) {
  let isSearching = false;
  const unsub = searching.subscribe(v => (isSearching = v));
  unsub();
  if (!isSearching) return;

  const data = event.event_data;
  if (!currentAccount) return;

  const involved =
    extractAccount(data.white) === currentAccount ||
    extractAccount(data.black) === currentAccount;
  if (!involved) return;

  searching.set(false);
  stopSSE();

  const gameId = data.game_id as GameId;
  if (!gameId) return;

  const url = gameUrl(gameId);
  const onGamePage = window.location.pathname.startsWith('/game/');

  if (onGamePage) {
    showToast(
      'success',
      'Match found!',
      'A new game has been created.',
      url,
      'Open game →'
    );
  } else {
    showToast('success', 'Match found! Redirecting...');
    setTimeout(() => goto(url), 1000);
  }
}

export async function startSearch(
  minElo: number,
  maxElo: number,
  wager: number
): Promise<boolean> {
  if (!currentAccount) return false;

  try {
    if (wager > 0) {
      const rawAmount = (BigInt(wager) * 10n ** 24n).toString();
      await contract.joinMatchmakingWithWager(
        WRAP_NEAR_ID,
        minElo,
        maxElo,
        rawAmount
      );
    } else {
      const result = await contract.joinMatchmaking(minElo, maxElo);
      const gameId = decodeSuccessValue<GameId | null>(result);
      if (gameId) {
        showToast('success', 'Match found! Redirecting...');
        setTimeout(() => goto(gameUrl(gameId)), 1000);
        return true;
      }
    }
    searching.set(true);
    searchMinElo.set(minElo);
    searchMaxElo.set(maxElo);
    searchWager.set(wager);
    startSSE();
    showToast('info', 'Searching for opponent...');
    return true;
  } catch (err: unknown) {
    const msg = err instanceof Error ? err.message : String(err);
    showToast('error', 'Failed to find match', msg);
    return false;
  }
}

export async function cancelSearch(): Promise<void> {
  searching.set(false);
  stopSSE();
  try {
    await contract.cancelMatchmaking();
    showToast('info', 'Matchmaking cancelled');
  } catch (err: unknown) {
    const msg = err instanceof Error ? err.message : String(err);
    showToast('error', 'Failed to cancel matchmaking', msg);
  }
}

if (browser) {
  accountStore.subscribe(account => {
    currentAccount = account;
    if (account) {
      contract
        .isQueued(account)
        .then(entry => {
          if (entry) {
            searching.set(true);
            searchMinElo.set(entry.min_elo);
            searchMaxElo.set(entry.max_elo);
            const w = entry.wager;
            if (w && Array.isArray(w) && w[1]) {
              searchWager.set(Number(BigInt(w[1]) / 10n ** 24n));
            } else {
              searchWager.set(0);
            }
            startSSE();
          }
        })
        .catch(() => {});
    } else {
      searching.set(false);
      stopSSE();
    }
  });
}
