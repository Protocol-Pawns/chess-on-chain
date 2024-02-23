import { readable } from "svelte/store";

import { browser } from "$app/environment";

export const selector$ = readable(
  browser
    ? Promise.all([
        import("@near-wallet-selector/core"),
        import("@near-wallet-selector/here-wallet"),
        import("@near-wallet-selector/meteor-wallet"),
      ]).then(
        ([
          { setupWalletSelector },
          { setupHereWallet },
          { setupMeteorWallet },
        ]) =>
          setupWalletSelector({
            network: import.meta.env.VITE_NETWORK_ID,
            modules: [setupHereWallet(), setupMeteorWallet()],
          }),
      )
    : // eslint-disable-next-line @typescript-eslint/no-empty-function
      new Promise<never>(() => {}),
);

export interface WalletMetadata {
  url: string;
  extensionUrl?: string;
  twitter?: string;
  telegram?: string;
  discord?: string;
}

export const NEAR_WALLETS: Record<string, WalletMetadata> = {
  "here-wallet": {
    url: "https://herewallet.app/",
    twitter: "https://twitter.com/here_wallet",
  },
  "meteor-wallet": {
    url: "https://meteorwallet.app/",
    extensionUrl:
      "https://chrome.google.com/webstore/detail/meteor-wallet/pcndjhkinnkaohffealmlmhaepkpmgkb",
    twitter: "https://twitter.com/MeteorWallet",
  },
};
