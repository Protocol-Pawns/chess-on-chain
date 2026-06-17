declare module 'web-push/src/encryption-helper.js' {
  export function encrypt(
    userPublicKey: string,
    userAuth: string,
    payload: string | Buffer,
    contentEncoding: string
  ): { cipherText: Buffer; salt: string; localPublicKey: Buffer };
}

declare module 'web-push/src/vapid-helper.js' {
  export function getVapidHeaders(
    audience: string,
    subject: string,
    publicKey: string,
    privateKey: string,
    contentEncoding: string
  ): { Authorization: string; 'Crypto-Key'?: string };
}

declare module 'web-push/src/web-push-constants.js' {
  export const supportedContentEncodings: {
    AES_128_GCM: string;
    AES_GCM: string;
  };
  export const supportedUrgency: {
    VERY_LOW: string;
    LOW: string;
    NORMAL: string;
    HIGH: string;
  };
}
