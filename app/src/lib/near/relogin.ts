import { writable } from 'svelte/store';

export const reloginPromptOpen = writable(false);

let reloginPromise: {
  promise: Promise<void>;
  resolve: () => void;
  reject: () => void;
} | null = null;

let reloginHandler: (() => Promise<void>) | null = null;

export function setReloginHandler(handler: () => Promise<void>) {
  reloginHandler = handler;
}

export function requestRelogin(): Promise<void> {
  if (reloginPromise) return reloginPromise.promise;

  reloginPromptOpen.set(true);

  let resolve!: () => void;
  let reject!: () => void;
  const promise = new Promise<void>((res, rej) => {
    resolve = res;
    reject = rej;
  });

  reloginPromise = { promise, resolve, reject };
  return promise;
}

export async function confirmRelogin() {
  if (!reloginPromise) return;

  try {
    if (reloginHandler) {
      await reloginHandler();
    }
    reloginPromptOpen.set(false);
    reloginPromise.resolve();
  } catch (e) {
    reloginPromptOpen.set(false);
    reloginPromise.reject();
    throw e;
  } finally {
    reloginPromise = null;
  }
}

export function cancelRelogin() {
  reloginPromptOpen.set(false);
  if (reloginPromise) {
    reloginPromise.reject();
    reloginPromise = null;
  }
}
