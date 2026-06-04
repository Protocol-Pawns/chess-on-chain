const API_URL =
  import.meta.env.VITE_API_URL || 'https://api.protocol-pawns.com';

export interface PaginatedResult<T> {
  items: T[];
  next_cursor: string | null;
}

export interface GameOverview {
  game_id: [number, string, string | null];
  white: { type: string; value: string };
  black: { type: string; value: string | null };
  board: string[];
  fen?: string;
  status?: string;
  outcome?: { result: string; color?: string } | null;
  resigner?: string | null;
  created_at?: string;
  finished_at?: string | null;
}

export interface Game extends GameOverview {
  moves: Array<{ color: string; mv: string; board: string[]; fen?: string }>;
}

export interface GameMove {
  move_number: number;
  color: string;
  move_notation: string;
  fen: string;
  outcome?: { result: string; color?: string } | null;
}

export interface AccountStats {
  account_id: string;
  wins: number;
  losses: number;
  draws: number;
  total_games: number;
}

export interface LeaderboardEntry {
  account_id: string;
  wins: number;
  losses: number;
  draws: number;
  total_games: number;
  win_rate: number;
}

export interface EloLeaderboardEntry {
  rank: number;
  account_id: string;
  elo: number;
}

export interface EloLeaderboardPage {
  total: number;
  page: number;
  per_page: number;
  total_pages: number;
  entries: EloLeaderboardEntry[];
}

export interface GlobalStats {
  total_games: number;
  active_games: number;
  finished_games: number;
  cancelled_games: number;
  total_moves: number;
}

export interface Challenge {
  id: string;
  challenger: string;
  challenged: string;
  wager_token: string | null;
  wager_amount: string | null;
  status: 'pending' | 'accepted' | 'rejected';
  game_id: string | null;
  created_at: string;
  resolved_at: string | null;
}

async function request<T>(path: string, init?: RequestInit): Promise<T> {
  const res = await fetch(`${API_URL}${path}`, init);
  if (!res.ok) throw new Error(`API error: ${res.status} ${res.statusText}`);
  return res.json();
}

export const api = {
  info: () => request<{ lastBlockHeight: number }>('/info'),
  stats: () => request<GlobalStats>('/stats'),
  game: (id: string) => request<Game>(`/game/${encodeURIComponent(id)}`),
  gameMoves: (id: string) =>
    request<GameMove[]>(`/game/${encodeURIComponent(id)}/moves`),
  games: (status: 'active' | 'finished', cursor?: string, limit?: number) =>
    request<PaginatedResult<GameOverview>>(
      `/games?status=${status}${cursor ? `&cursor=${cursor}` : ''}${limit ? `&limit=${limit}` : ''}`
    ),
  activeGame: (accountId: string) =>
    request<Game>(`/account/${accountId}/active-game`),
  account: (accountId: string) =>
    request<{ finishedGameIds: [number, string, string | null][] }>(
      `/account/${accountId}`
    ),
  accountStats: (accountId: string) =>
    request<AccountStats>(`/account/${accountId}/stats`),
  challenges: (accountId: string) =>
    request<Challenge[]>(`/account/${accountId}/challenges`),
  leaderboard: (cursor?: string, limit?: number) =>
    request<PaginatedResult<LeaderboardEntry>>(
      `/leaderboard${cursor ? `?cursor=${cursor}` : ''}${limit ? `${cursor ? '&' : '?'}limit=${limit}` : ''}`
    ),
  leaderboardElo: (page = 1, perPage = 25) =>
    request<EloLeaderboardPage>(
      `/leaderboard/elo?page=${page}&per_page=${perPage}`
    ),
  vapidPublicKey: () => request<{ publicKey: string }>('/vapid-public-key'),
  subscribePush: (
    accountId: string,
    subscription: { endpoint: string; keys: { p256dh: string; auth: string } }
  ) =>
    request<{ ok: boolean }>(`/account/${accountId}/push-subscription`, {
      method: 'POST',
      headers: { 'Content-Type': 'application/json' },
      body: JSON.stringify(subscription)
    }),
  unsubscribePush: (accountId: string, endpoint: string) =>
    request<{ ok: boolean }>(`/account/${accountId}/push-subscription`, {
      method: 'DELETE',
      headers: { 'Content-Type': 'application/json' },
      body: JSON.stringify({ endpoint })
    })
};
