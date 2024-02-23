import type { Account } from "@near-wallet-selector/core";
import { writable } from "svelte/store";

import Login, { showWalletSelector } from "./Login.svelte";
import WalletSelector from "./WalletSelector.svelte";

// undefined: uninitialized
// null: not logged in
export const account$ = writable<Account | null | undefined>(undefined);

export { showWalletSelector, Login, WalletSelector };
