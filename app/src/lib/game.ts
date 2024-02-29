import { writable } from "svelte/store";

import type { GameId } from "$abi";
import { navigating } from "$app/stores";

export const gameId$ = writable<GameId | undefined>();

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

navigating.subscribe(() => {
  gameId$.set(undefined);
});

window.addEventListener("popstate", () => {
  const loadedUrl = new URL(window.location.href);
  const loadedGameId = loadedUrl.searchParams.get("game_id");
  gameId$.set(loadedGameId ? JSON.parse(decodeURI(loadedGameId)) : undefined);
});
