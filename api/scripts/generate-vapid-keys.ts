async function main() {
  const keyPair = await crypto.subtle.generateKey(
    { name: 'ECDSA', namedCurve: 'P-256' },
    true,
    ['sign']
  );
  const privateJwk = await crypto.subtle.exportKey('jwk', keyPair.privateKey);
  const publicRaw = await crypto.subtle.exportKey('raw', keyPair.publicKey);
  const publicKeyBase64url = Buffer.from(publicRaw).toString('base64url');

  console.log('Add these as wrangler secrets or .dev.vars:\n');
  console.log('VAPID_PRIVATE_KEY=');
  console.log(JSON.stringify(privateJwk));
  console.log('\nVAPID_PUBLIC_KEY=');
  console.log(publicKeyBase64url);
}

main();
