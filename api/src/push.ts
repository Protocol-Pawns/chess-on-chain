function base64urlEncode(data: ArrayBuffer | Uint8Array): string {
  const bytes = data instanceof Uint8Array ? data : new Uint8Array(data);
  let binary = '';
  for (const byte of bytes) binary += String.fromCharCode(byte);
  return btoa(binary)
    .replace(/\+/g, '-')
    .replace(/\//g, '_')
    .replace(/=+$/, '');
}

function base64urlDecode(str: string): Uint8Array {
  const padded = str.replace(/-/g, '+').replace(/_/g, '/');
  const binary = atob(padded);
  const bytes = new Uint8Array(binary.length);
  for (let i = 0; i < binary.length; i++) bytes[i] = binary.charCodeAt(i);
  return bytes;
}

function concat(...arrays: Uint8Array[]): Uint8Array {
  const total = arrays.reduce((sum, a) => sum + a.length, 0);
  const result = new Uint8Array(total);
  let offset = 0;
  for (const a of arrays) {
    result.set(a, offset);
    offset += a.length;
  }
  return result;
}

async function hmacSha256(
  key: Uint8Array,
  data: Uint8Array
): Promise<Uint8Array> {
  const cryptoKey = await crypto.subtle.importKey(
    'raw',
    key,
    { name: 'HMAC', hash: 'SHA-256' },
    false,
    ['sign']
  );
  const sig = await crypto.subtle.sign('HMAC', cryptoKey, data);
  return new Uint8Array(sig);
}

async function importEcdhPublicKey(rawBase64url: string): Promise<CryptoKey> {
  const raw = base64urlDecode(rawBase64url);
  return crypto.subtle.importKey(
    'raw',
    raw,
    { name: 'ECDH', namedCurve: 'P-256' },
    true,
    []
  );
}

function derToRaw(der: Uint8Array): Uint8Array {
  if (der[0] !== 0x30) throw new Error('Invalid DER');
  let offset = 2;
  if (der[offset] !== 0x02) throw new Error('Invalid DER');
  offset++;
  const rLen = der[offset];
  offset++;
  const r = der.slice(offset, offset + rLen);
  offset += rLen;
  if (der[offset] !== 0x02) throw new Error('Invalid DER');
  offset++;
  const sLen = der[offset];
  offset++;
  const s = der.slice(offset, offset + sLen);
  return concat(norm32(r), norm32(s));
}

function norm32(bytes: Uint8Array): Uint8Array {
  if (bytes.length === 32) return bytes;
  if (bytes.length > 32) return bytes.slice(bytes.length - 32);
  const result = new Uint8Array(32);
  result.set(bytes, 32 - bytes.length);
  return result;
}

function buildContext(
  ephemeralPub: Uint8Array,
  subscriptionPub: Uint8Array
): Uint8Array {
  const prefix = new TextEncoder().encode('P-256\0');
  const eLen = new Uint8Array([0, ephemeralPub.length]);
  const sLen = new Uint8Array([0, subscriptionPub.length]);
  return concat(prefix, eLen, ephemeralPub, sLen, subscriptionPub);
}

async function generateVapidJwt(
  privateKey: CryptoKey,
  origin: string,
  subject: string
): Promise<string> {
  const header = base64urlEncode(
    new TextEncoder().encode(JSON.stringify({ typ: 'JWT', alg: 'ES256' }))
  );
  const now = Math.floor(Date.now() / 1000);
  const body = base64urlEncode(
    new TextEncoder().encode(
      JSON.stringify({ aud: origin, exp: now + 43200, sub: subject })
    )
  );
  const toSign = new TextEncoder().encode(`${header}.${body}`);
  const sig = await crypto.subtle.sign(
    { name: 'ECDSA', hash: 'SHA-256' },
    privateKey,
    toSign
  );
  const rawSig = derToRaw(new Uint8Array(sig));
  return `${header}.${body}.${base64urlEncode(rawSig)}`;
}

async function encryptPayload(
  payload: string,
  p256dhBase64url: string,
  authBase64url: string
): Promise<Uint8Array> {
  const auth = base64urlDecode(authBase64url);

  const ephemeralKey = await crypto.subtle.generateKey(
    { name: 'ECDH', namedCurve: 'P-256' },
    true,
    ['deriveBits']
  );
  const ephemeralPubRaw = new Uint8Array(
    await crypto.subtle.exportKey('raw', ephemeralKey.publicKey)
  );

  const subscriptionPub = await importEcdhPublicKey(p256dhBase64url);
  const subscriptionPubRaw = new Uint8Array(
    await crypto.subtle.exportKey('raw', subscriptionPub)
  );

  const sharedSecret = new Uint8Array(
    await crypto.subtle.deriveBits(
      { name: 'ECDH', public: subscriptionPub },
      ephemeralKey.privateKey,
      256
    )
  );

  const prk = await hmacSha256(auth, sharedSecret);

  const keyInfoPrefix = new TextEncoder().encode(
    'Content-Encoding: aes128gcm\0'
  );
  const context = buildContext(ephemeralPubRaw, subscriptionPubRaw);
  const keyInfo = concat(keyInfoPrefix, context);
  const cekFull = await hmacSha256(prk, concat(keyInfo, new Uint8Array([1])));
  const cek = cekFull.slice(0, 16);

  const nonceInfoPrefix = new TextEncoder().encode('Content-Encoding: nonce\0');
  const nonceInfo = concat(nonceInfoPrefix, context);
  const nonceFull = await hmacSha256(
    prk,
    concat(nonceInfo, new Uint8Array([1]))
  );
  const nonce = nonceFull.slice(0, 12);

  const plaintext = concat(
    new TextEncoder().encode(payload),
    new Uint8Array([2])
  );

  const aesKey = await crypto.subtle.importKey(
    'raw',
    cek,
    { name: 'AES-GCM' },
    false,
    ['encrypt']
  );
  const encrypted = await crypto.subtle.encrypt(
    { name: 'AES-GCM', iv: nonce, tagLength: 128 },
    aesKey,
    plaintext
  );

  const salt = crypto.getRandomValues(new Uint8Array(16));
  const rs = new Uint8Array([0, 0, 0x10, 0]);

  return concat(
    salt,
    rs,
    new Uint8Array([65]),
    ephemeralPubRaw,
    new Uint8Array(encrypted)
  );
}

export async function sendPush(
  subscription: {
    endpoint: string;
    p256dh: string;
    auth: string;
  },
  payload: object,
  vapidPrivateKey: CryptoKey,
  vapidPublicKeyB64: string,
  vapidSubject: string
): Promise<{ ok: boolean; subscriptionExpired: boolean }> {
  try {
    const origin = new URL(subscription.endpoint).origin;
    const jwt = await generateVapidJwt(vapidPrivateKey, origin, vapidSubject);

    const encryptedPayload = await encryptPayload(
      JSON.stringify(payload),
      subscription.p256dh,
      subscription.auth
    );

    const response = await fetch(subscription.endpoint, {
      method: 'POST',
      headers: {
        'Content-Type': 'application/octet-stream',
        'Content-Encoding': 'aes128gcm',
        TTL: '86400',
        Authorization: `vapid t=${jwt}, k=${vapidPublicKeyB64}`
      },
      body: encryptedPayload
    });

    if (response.status === 410 || response.status === 404) {
      return { ok: false, subscriptionExpired: true };
    }

    return { ok: response.ok, subscriptionExpired: false };
  } catch (err) {
    console.error('Push send error:', err);
    return { ok: false, subscriptionExpired: false };
  }
}

export function importVapidKey(jwkJson: string): Promise<CryptoKey> {
  return crypto.subtle.importKey(
    'jwk',
    JSON.parse(jwkJson),
    { name: 'ECDSA', namedCurve: 'P-256' },
    true,
    ['sign']
  );
}
