import type { GameId } from '$lib/near/contract-types';

const API_URL =
  import.meta.env.VITE_API_URL || 'https://api.protocol-pawns.com';

export interface PaginatedResult<T> {
  items: T[];
  next_cursor: string | null;
  total_count?: number;
  total_pages?: number;
  page?: number;
  per_page?: number;
}

export interface GameOverview {
  game_id: GameId;
  white: { type: string; value: string };
  black: { type: string; value: string | null };
  board: string[];
  fen?: string;
  status?: string;
  outcome?: { result: string; color?: string } | null;
  resigner?: string | null;
  created_at?: string;
  finished_at?: string | null;
  wager_token?: string | null;
  wager_amount?: string | null;
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

export interface AccountSearchResult extends AccountStats {
  elo: number | null;
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

export interface RankingEntry {
  rank: number;
  account_id: string;
  elo: number | null;
  ppp: string;
  wins: number;
  losses: number;
  draws: number;
  total_games: number;
}

export interface RankingPage {
  total: number;
  page: number;
  per_page: number;
  total_pages: number;
  entries: RankingEntry[];
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

export interface Bet {
  id: string;
  bettor: string;
  player_0: string;
  player_1: string;
  game_id: string | null;
  token_id: string;
  amount: string;
  winner: string;
  status: 'pending' | 'locked' | 'resolved';
  payout: string | null;
  created_at: string;
  locked_at: string | null;
  resolved_at: string | null;
}

export interface TokenBetStats {
  wagered: string;
  won: string;
}

export interface BetStats {
  account_id: string;
  total_bets: number;
  won_bets: number;
  by_token: Record<string, TokenBetStats>;
}

export interface BetLeaderboardEntry {
  account_id: string;
  total_bets: number;
  won_bets: number;
  by_token: Record<string, TokenBetStats>;
}

async function request<T>(path: string, init?: RequestInit): Promise<T> {
  const res = await fetch(`${API_URL}${path}`, init);
  if (!res.ok) throw new Error(`API error: ${res.status} ${res.statusText}`);
  return res.json();
}

export const api = {
  info: () => request<{ lastBlockHeight: number }>('/info'),
  stats: () => request<GlobalStats>('/stats'),
  game: (id: string) =>
    request<GameOverview>(`/game/${encodeURIComponent(id)}`),
  gameMoves: (id: string) =>
    request<GameMove[]>(`/game/${encodeURIComponent(id)}/moves`),
  games: (
    status: 'active' | 'finished',
    cursor?: string,
    limit?: number,
    page?: number,
    excludeAi?: boolean
  ) => {
    const params = new URLSearchParams();
    params.set('status', status);
    if (cursor) params.set('cursor', cursor);
    if (limit) params.set('limit', String(limit));
    if (page) params.set('page', String(page));
    if (excludeAi) params.set('exclude_ai', 'true');
    return request<PaginatedResult<GameOverview>>(`/games?${params}`);
  },
  activeGame: (accountId: string) =>
    request<GameOverview>(`/account/${accountId}/active-game`),
  account: (accountId: string) =>
    request<{ finishedGameIds: GameId[] }>(`/account/${accountId}`),
  accountStats: (accountId: string) =>
    request<AccountStats>(`/account/${accountId}/stats`),
  accountStatsBatch: (accountIds: string[]) =>
    request<AccountStats[]>('/account/stats/batch', {
      method: 'POST',
      headers: { 'Content-Type': 'application/json' },
      body: JSON.stringify({ account_ids: accountIds })
    }),
  challenges: (
    accountId: string,
    page?: number,
    perPage?: number,
    excludeRejected?: boolean
  ) => {
    const params = new URLSearchParams();
    if (page) params.set('page', String(page));
    if (perPage) params.set('per_page', String(perPage));
    if (excludeRejected) params.set('exclude_rejected', 'true');
    const qs = params.toString();
    const suffix = qs ? `?${qs}` : '';
    return request<Challenge[] | PaginatedResult<Challenge>>(
      `/account/${accountId}/challenges${suffix}`
    );
  },
  searchAccounts: (query: string) =>
    request<AccountSearchResult[]>('/account/query', {
      method: 'POST',
      headers: { 'Content-Type': 'application/json' },
      body: JSON.stringify({ query })
    }),
  query: (gameIds: GameId[]) =>
    request<GameOverview[]>('/query', {
      method: 'POST',
      headers: { 'Content-Type': 'application/json' },
      body: JSON.stringify({ gameIds })
    }),
  leaderboardElo: (page = 1, perPage = 25, dir: 'desc' | 'asc' = 'desc') =>
    request<RankingPage>(
      `/leaderboard/elo?page=${page}&per_page=${perPage}&dir=${dir}`
    ),
  leaderboardPpp: (page = 1, perPage = 25) =>
    request<RankingPage>(`/leaderboard/ppp?page=${page}&per_page=${perPage}`),
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
    }),
  bets: (
    accountId: string,
    status?: 'pending' | 'locked' | 'resolved',
    cursor?: string,
    limit?: number
  ) =>
    request<PaginatedResult<Bet>>(
      `/account/${accountId}/bets${status ? `?status=${status}` : ''}${cursor ? `${status ? '&' : '?'}cursor=${cursor}` : ''}${limit ? `${status || cursor ? '&' : '?'}limit=${limit}` : ''}`
    ),
  gameBets: (gameId: string) =>
    request<Bet[]>(`/game/${encodeURIComponent(gameId)}/bets`),
  betStats: (accountId: string) =>
    request<BetStats>(`/account/${accountId}/bet-stats`),
  betLeaderboard: (cursor?: string, limit?: number) =>
    request<PaginatedResult<BetLeaderboardEntry>>(
      `/leaderboard/bets${cursor ? `?cursor=${cursor}` : ''}${limit ? `${cursor ? '&' : '?'}limit=${limit}` : ''}`
    ),
  openChallenges: (cursor?: string, limit?: number) =>
    request<PaginatedResult<Challenge>>(
      `/challenges${cursor ? `?cursor=${cursor}` : ''}${limit ? `${cursor ? '&' : '?'}limit=${limit}` : ''}`
    ),
  globalBets: (
    status?: 'pending' | 'locked' | 'resolved',
    cursor?: string,
    limit?: number
  ) =>
    request<PaginatedResult<Bet>>(
      `/bets${status ? `?status=${status}` : ''}${cursor ? `${status ? '&' : '?'}cursor=${cursor}` : ''}${limit ? `${status || cursor ? '&' : '?'}limit=${limit}` : ''}`
    )
};
