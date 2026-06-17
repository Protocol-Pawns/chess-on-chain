import { writable } from 'svelte/store';

declare const __APP_VERSION__: string;

export const pwaInstallAvailable = writable(false);
export const pwaInstalled = writable(false);
export const swUpdateAvailable = writable(false);
export const swVersion = writable<string | null>(null);

let deferredPrompt: BeforeInstallPromptEvent | null = null;
let hadController = false;
let checkingForUpdate = false;

interface BeforeInstallPromptEvent extends Event {
  prompt(): Promise<void>;
  userChoice: Promise<{ outcome: 'accepted' | 'dismissed' }>;
}

declare global {
  interface WindowEventMap {
    beforeinstallprompt: BeforeInstallPromptEvent;
  }
}

async function checkForUpdate() {
  if (checkingForUpdate) return;
  checkingForUpdate = true;
  try {
    const res = await fetch('/version.json', { cache: 'no-store' });
    if (!res.ok) {
      console.error('[PWA] version.json returned', res.status);
      return;
    }
    const data = await res.json();
    const deployedVersion: string = data.version;

    swVersion.set(deployedVersion);

    if (deployedVersion === __APP_VERSION__) {
      swUpdateAvailable.set(false);
      return;
    }

    const acknowledged = localStorage.getItem('sw_acknowledged_version');
    if (deployedVersion === acknowledged) {
      swUpdateAvailable.set(false);
      return;
    }

    console.log('[PWA] new version available:', deployedVersion);
    swUpdateAvailable.set(true);
  } catch (err) {
    console.error('[PWA] failed to check version:', err);
  } finally {
    checkingForUpdate = false;
  }
}

export function registerServiceWorker() {
  if (typeof window === 'undefined') return;
  if (!('serviceWorker' in navigator)) return;

  hadController = !!navigator.serviceWorker.controller;

  navigator.serviceWorker
    .register('/sw.js', { updateViaCache: 'none' })
    .then(function (reg) {
      reg.addEventListener('updatefound', function () {
        const newWorker = reg.installing;
        if (!newWorker) return;
        newWorker.addEventListener('statechange', function () {
          if (
            hadController &&
            newWorker!.state === 'activated' &&
            navigator.serviceWorker.controller
          ) {
            console.log('[PWA] new SW activated, checking version');
            checkForUpdate();
          }
        });
      });
    })
    .catch(function (err) {
      console.error('[PWA] SW registration failed:', err);
    });

  navigator.serviceWorker.addEventListener('controllerchange', function () {
    if (hadController) {
      console.log('[PWA] SW controller changed, checking version');
      checkForUpdate();
    }
  });

  navigator.serviceWorker.addEventListener('message', function (event) {
    if (event.data?.type === 'SW_UPDATE_READY') {
      console.log('[PWA] SW_UPDATE_READY, checking version');
      checkForUpdate();
    }
  });

  document.addEventListener('visibilitychange', function () {
    if (document.visibilityState === 'visible') {
      checkForUpdate();
    }
  });

  window.addEventListener('pageshow', function (e) {
    if (e.persisted) {
      checkForUpdate();
    }
  });

  setTimeout(checkForUpdate, 1000);
}

export function initPwaInstallPrompt() {
  if (typeof window === 'undefined') return;

  if (window.matchMedia('(display-mode: standalone)').matches) {
    pwaInstalled.set(true);
    return;
  }

  window.addEventListener('beforeinstallprompt', function (e) {
    e.preventDefault();
    deferredPrompt = e;
    pwaInstallAvailable.set(true);
  });

  window.addEventListener('appinstalled', function () {
    deferredPrompt = null;
    pwaInstallAvailable.set(false);
    pwaInstalled.set(true);
  });
}

export async function promptInstall(): Promise<boolean> {
  if (!deferredPrompt) return false;

  await deferredPrompt.prompt();
  const result = await deferredPrompt.userChoice;
  deferredPrompt = null;

  if (result.outcome === 'accepted') {
    pwaInstallAvailable.set(false);
    return true;
  }
  return false;
}
