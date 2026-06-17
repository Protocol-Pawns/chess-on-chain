import { encrypt } from 'web-push/src/encryption-helper.js';
import { getVapidHeaders } from 'web-push/src/vapid-helper.js';
import { supportedContentEncodings } from 'web-push/src/web-push-constants.js';

const contentEncoding = supportedContentEncodings.AES_128_GCM;

export type SendPushResult = {
  ok: boolean;
  subscriptionExpired: boolean;
  status: number;
};

export type SendPushFn = (
  subscription: {
    endpoint: string;
    p256dh: string;
    auth: string;
  },
  payload: object,
  vapidPrivateKey: string,
  vapidPublicKeyB64: string,
  vapidSubject: string
) => Promise<SendPushResult>;

export const sendPush: SendPushFn = async (
  subscription,
  payload,
  vapidPrivateKey,
  vapidPublicKeyB64,
  vapidSubject
) => {
  try {
    const payloadStr = JSON.stringify(payload);
    const encrypted = encrypt(
      subscription.p256dh,
      subscription.auth,
      payloadStr,
      contentEncoding
    );

    const url = new URL(subscription.endpoint);
    const audience = `${url.protocol}//${url.host}`;

    const vapidHeaders = getVapidHeaders(
      audience,
      vapidSubject,
      vapidPublicKeyB64,
      vapidPrivateKey,
      contentEncoding
    );

    const headers: Record<string, string> = {
      TTL: '86400',
      'Content-Type': 'application/octet-stream',
      'Content-Encoding': contentEncoding,
      'Content-Length': String(encrypted.cipherText.length),
      Authorization: vapidHeaders.Authorization,
      Urgency: 'normal'
    };

    const response = await fetch(subscription.endpoint, {
      method: 'POST',
      headers,
      body: new Uint8Array(encrypted.cipherText)
    });

    if (response.status === 410 || response.status === 404) {
      return { ok: false, subscriptionExpired: true, status: response.status };
    }

    if (!response.ok) {
      let body = '';
      try {
        body = await response.text();
      } catch {
        // ignore body read errors
      }
      console.error(
        `Push rejected: status=${response.status} endpoint=${subscription.endpoint} body=${body.slice(0, 200)}`
      );
    }

    return {
      ok: response.ok,
      subscriptionExpired: false,
      status: response.status
    };
  } catch (err) {
    console.error('Push send error:', err);
    return { ok: false, subscriptionExpired: false, status: 0 };
  }
};

export function importVapidKey(jwkJson: string): string {
  const jwk = JSON.parse(jwkJson) as { d: string };
  return jwk.d;
}
