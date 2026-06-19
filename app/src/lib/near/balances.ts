import { writable } from 'svelte/store';

import { accountStore } from './account';
import { contract } from './connector';

import { browser } from '$app/environment';
import { onReconnect } from '$lib/sse';

export const escrowTokens = writable<Array<[string, string]>>([]);

let pollInterval: ReturnType<typeof setInterval> | null = null;
let currentAccount: string | undefined;

export async function refreshEscrowTokens(): Promise<void> {
  if (!currentAccount) {
    escrowTokens.set([]);
    return;
  }
  try {
    const tokens = await contract.getTokens(currentAccount);
    escrowTokens.set(tokens ?? []);
  } catch {
    /* keep stale data on error */
  }
}

function startPolling() {
  stopPolling();
  refreshEscrowTokens();
  pollInterval = setInterval(refreshEscrowTokens, 30_000);
}

function stopPolling() {
  if (pollInterval) {
    clearInterval(pollInterval);
    pollInterval = null;
  }
}

if (browser) {
  accountStore.subscribe(account => {
    currentAccount = account;
    if (account) {
      startPolling();
    } else {
      stopPolling();
      escrowTokens.set([]);
    }
  });

  document.addEventListener('visibilitychange', () => {
    if (document.visibilityState === 'visible') refreshEscrowTokens();
  });

  onReconnect(refreshEscrowTokens);
}
