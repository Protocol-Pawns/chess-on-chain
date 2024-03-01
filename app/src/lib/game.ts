import { get, writable } from "svelte/store";

import { contract$ } from "./near";

import type { AccountId, GameId } from "$abi";
import { navigating } from "$app/stores";

export const gameId$ = writable<GameId | undefined>();

export const eloRatings$ = writable<Record<AccountId, number | undefined>>({});

export const defaultBoard = [
  "RNBQKBNR",
  "PPPPPPPP",
  "        ",
  "        ",
  "        ",
  "        ",
  "pppppppp",
  "rnbqkbnr",
];

export async function updateEloRatings(accountIds: AccountId[]) {
  const contract = await get(contract$);
  const updatedRatings = Object.fromEntries(
    await contract.get_elo_ratings_by_ids({
      account_ids: Array.from(accountIds),
    }),
  );
  eloRatings$.update((oldRatings) => ({ ...oldRatings, ...updatedRatings }));
}

navigating.subscribe(() => {
  gameId$.set(undefined);
});

window.addEventListener("popstate", () => {
  const loadedUrl = new URL(window.location.href);
  const loadedGameId = loadedUrl.searchParams.get("game_id");
  gameId$.set(loadedGameId ? JSON.parse(decodeURI(loadedGameId)) : undefined);
});
