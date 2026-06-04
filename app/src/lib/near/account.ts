import { getConnector, contract } from './connector';
import { browser } from '$app/environment';
import { writable, derived } from 'svelte/store';

export const accountStore = writable<string | undefined>(undefined);
export const isLoggedIn = derived(accountStore, $a => $a !== undefined);
export const isRegistered = writable(false);
export const isCheckingRegistration = writable(false);
export const pushEnabled = writable(false);

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
  });
}

async function checkRegistration(accountId: string) {
  isCheckingRegistration.set(true);
  try {
    const balance = await contract.storageBalanceOf(accountId);
    isRegistered.set(balance !== null);
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
  await c.connect();
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
  const reg = await navigator.serviceWorker.ready;
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
