import { api } from '$lib/api/client';

export async function registerPushNotifications(
  accountId: string
): Promise<boolean> {
  if (!('serviceWorker' in navigator) || !('PushManager' in window))
    return false;

  try {
    const registration = await navigator.serviceWorker.ready;

    let subscription = await registration.pushManager.getSubscription();

    if (!subscription) {
      const { publicKey } = await api.vapidPublicKey();
      subscription = await registration.pushManager.subscribe({
        userVisibleOnly: true,
        applicationServerKey: publicKey
      });
    }

    const sub = subscription.toJSON();
    if (!sub.endpoint || !sub.keys?.p256dh || !sub.keys?.auth) return false;

    await api.subscribePush(accountId, {
      endpoint: sub.endpoint,
      keys: { p256dh: sub.keys.p256dh, auth: sub.keys.auth }
    });

    return true;
  } catch (e) {
    console.error('Push registration failed:', e);
    return false;
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
