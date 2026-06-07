import { writable } from 'svelte/store';

export const pwaInstallAvailable = writable(false);
export const pwaInstalled = writable(false);
export const swUpdateAvailable = writable(false);

let deferredPrompt: BeforeInstallPromptEvent | null = null;

interface BeforeInstallPromptEvent extends Event {
  prompt(): Promise<void>;
  userChoice: Promise<{ outcome: 'accepted' | 'dismissed' }>;
}

declare global {
  interface WindowEventMap {
    beforeinstallprompt: BeforeInstallPromptEvent;
  }
}

export function registerServiceWorker() {
  if (typeof window === 'undefined') return;
  if (!('serviceWorker' in navigator)) return;

  navigator.serviceWorker
    .register('/sw.js')
    .then(function (reg) {
      reg.addEventListener('updatefound', function () {
        const newWorker = reg.installing;
        if (!newWorker) return;
        newWorker.addEventListener('statechange', function () {
          if (
            newWorker!.state === 'activated' &&
            navigator.serviceWorker.controller
          ) {
            swUpdateAvailable.set(true);
          }
        });
      });
    })
    .catch(function (err) {
      console.error('[PWA] SW registration failed:', err);
    });

  navigator.serviceWorker.addEventListener('message', function (event) {
    if (event.data?.type === 'SW_UPDATE_READY') {
      swUpdateAvailable.set(true);
    }
  });
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
