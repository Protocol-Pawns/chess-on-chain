import { hc, type InferResponseType } from "hono/client";

import type { AppType } from "$lib/models";

export const apiClient = hc<AppType>(import.meta.env.VITE_API_BASE_URL);

const getGameById = apiClient.games.game[":game_id"].$get;
export type GameApi = InferResponseType<typeof getGameById>;
