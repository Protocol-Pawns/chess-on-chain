import { KeyPair, KeyPairEd25519 } from 'near-api-js';
import { writable, derived } from 'svelte/store';

import { getConnector, contract } from './connector';

import { browser } from '$app/environment';

const KEY_STORE_KEY = 'chess-fc-keypair';

export const accountStore = writable<string | undefined>(undefined);
export const isLoggedIn = derived(accountStore, $a => $a !== undefined);
export const isRegistered = writable(false);
export const isCheckingRegistration = writable(false);
export const pushEnabled = writable(false);

export function getLocalKeyPair(): KeyPairEd25519 | null {
  if (!browser) return null;
  try {
    const raw = localStorage.getItem(KEY_STORE_KEY);
    if (!raw) return null;
    return KeyPair.fromString(raw as `ed25519:${string}`) as KeyPairEd25519;
  } catch {
    return null;
  }
}

if (browser) {
  const c = getConnector();
  c.on('wallet:signIn', async payload => {
    const accountId = payload.accounts?.[0]?.accountId;
    if (accountId) {
      accountStore.set(accountId);
      await checkRegistration(accountId);
      checkPushStatus();
    }
  });
  c.on('wallet:signOut', () => {
    accountStore.set(undefined);
    isRegistered.set(false);
    pushEnabled.set(false);
    localStorage.removeItem(KEY_STORE_KEY);
  });

  (async () => {
    try {
      const accounts = await c.wallet().then(w => w.getAccounts());
      const accountId = accounts?.[0]?.accountId;
      if (accountId) {
        console.log('[account] restored session for', accountId);
        accountStore.set(accountId);
        await checkRegistration(accountId);
        checkPushStatus();
      } else {
        console.log('[account] no existing session found');
      }
    } catch (e) {
      console.log('[account] could not restore session:', e);
    }
  })();
}

async function checkRegistration(accountId: string) {
  isCheckingRegistration.set(true);
  try {
    const balance = await contract.storageBalanceOf(accountId);
    isRegistered.set(balance !== null);
    if (balance !== null) {
      const ad = await contract.getAccount(accountId);
      if (ad.pending_points !== '0') {
        contract.claimPoints().catch(() => {});
      }
    }
  } catch {
    isRegistered.set(false);
  } finally {
    isCheckingRegistration.set(false);
  }
}

function checkPushStatus() {
  if (!('serviceWorker' in navigator)) return;
  navigator.serviceWorker.ready
    .then(async (reg: ServiceWorkerRegistration) => {
      const sub = await reg.pushManager.getSubscription();
      pushEnabled.set(sub !== null);
    })
    .catch(() => {});
}
export async function connect() {
  const c = getConnector();
  const keyPair = KeyPairEd25519.fromRandom();
  localStorage.setItem(KEY_STORE_KEY, keyPair.toString());
  console.log(
    '[account] stored local fc key:',
    keyPair.getPublicKey().toString()
  );
  await c.connect({
    addFunctionCallKey: {
      contractId: import.meta.env.VITE_CONTRACT_ID || 'app.chess-game.near',
      publicKey: keyPair.getPublicKey().toString(),
      allowMethods: {
        anyMethod: true
      },
      gasAllowance: { kind: 'unlimited' }
    }
  });
}

export async function disconnect() {
  const c = getConnector();
  await c.disconnect();
  accountStore.set(undefined);
  isRegistered.set(false);
  pushEnabled.set(false);
}

export async function register() {
  isCheckingRegistration.set(true);
  try {
    await contract.storageDeposit();
    const accountId = await new Promise<string>(resolve => {
      const unsub = accountStore.subscribe(v => {
        if (v) {
          resolve(v);
          unsub();
        }
      });
    });
    await checkRegistration(accountId);
  } finally {
    isCheckingRegistration.set(false);
  }
}

export async function enablePush() {
  if (!('serviceWorker' in navigator)) return false;
  await navigator.serviceWorker.register('/sw.js');
  await navigator.serviceWorker.ready;
  const { registerPushNotifications } = await import('$lib/push/register');
  const accountId = await new Promise<string>(resolve => {
    const unsub = accountStore.subscribe(v => {
      if (v) {
        resolve(v);
        unsub();
      }
    });
  });
  const ok = await registerPushNotifications(accountId);
  if (ok) pushEnabled.set(true);
  return ok;
}

export async function disablePush() {
  const accountId = await new Promise<string>(resolve => {
    const unsub = accountStore.subscribe(v => {
      if (v) {
        resolve(v);
        unsub();
      }
    });
  });
  const { unregisterPushNotifications } = await import('$lib/push/register');
  await unregisterPushNotifications(accountId);
  pushEnabled.set(false);
}
