import { api } from '$lib/api/client';

export type PushErrorCode = 'unsupported' | 'denied' | 'api' | 'unknown';

export class PushError extends Error {
  code: PushErrorCode;
  constructor(code: PushErrorCode, message: string) {
    super(message);
    this.name = 'PushError';
    this.code = code;
  }
}

export async function registerPushNotifications(
  accountId: string
): Promise<boolean> {
  if (!('serviceWorker' in navigator) || !('PushManager' in window))
    throw new PushError(
      'unsupported',
      'Your browser does not support push notifications.'
    );

  try {
    const registration = await navigator.serviceWorker.ready;

    if (typeof Notification !== 'undefined') {
      if (Notification.permission === 'denied')
        throw new PushError(
          'denied',
          'Notifications are blocked for this site.'
        );
      if (Notification.permission === 'default') {
        const result = await Notification.requestPermission();
        if (result !== 'granted')
          throw new PushError(
            'denied',
            'Notifications are blocked for this site.'
          );
      }
    }

    let subscription = await registration.pushManager.getSubscription();

    if (!subscription) {
      let publicKey: string;
      try {
        const res = await api.vapidPublicKey();
        publicKey = res.publicKey;
      } catch {
        throw new PushError('api', 'Could not reach the notification service.');
      }
      subscription = await registration.pushManager.subscribe({
        userVisibleOnly: true,
        applicationServerKey: publicKey
      });
    }

    const sub = subscription.toJSON();
    if (!sub.endpoint || !sub.keys?.p256dh || !sub.keys?.auth)
      throw new PushError('unknown', 'Invalid push subscription payload.');

    try {
      await api.subscribePush(accountId, {
        endpoint: sub.endpoint,
        keys: { p256dh: sub.keys.p256dh, auth: sub.keys.auth }
      });
    } catch {
      throw new PushError('api', 'Could not reach the notification service.');
    }

    return true;
  } catch (e) {
    if (e instanceof PushError) throw e;
    throw new PushError('unknown', e instanceof Error ? e.message : String(e));
  }
}

export async function unregisterPushNotifications(
  accountId: string
): Promise<void> {
  if (!('serviceWorker' in navigator)) return;

  try {
    const registration = await navigator.serviceWorker.ready;
    const subscription = await registration.pushManager.getSubscription();
    if (subscription) {
      await api.unsubscribePush(accountId, subscription.endpoint);
      await subscription.unsubscribe();
    }
  } catch (e) {
    console.error('Push unregistration failed:', e);
  }
}
