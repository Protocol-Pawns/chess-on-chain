import { z } from 'zod';

export const GameIdSchema = z.tuple([
  z.number(),
  z.string(),
  z.nullable(z.string())
]);
export type GameId = z.infer<typeof GameIdSchema>;

export const DifficultySchema = z.enum(['Easy', 'Medium', 'Hard']);
export type Difficulty = z.infer<typeof DifficultySchema>;

export const ColorSchema = z.enum(['White', 'Black']);
export type Color = z.infer<typeof ColorSchema>;

const BoardSchema = z.string().array().length(8);

export const GameOutcomeSchema = z.discriminatedUnion('result', [
  z
    .object({
      result: z.literal('Stalemate')
    })
    .strict(),
  z
    .object({
      result: z.literal('Victory'),
      color: ColorSchema
    })
    .strict()
]);
export type GameOutcome = z.infer<typeof GameOutcomeSchema>;

export const PlayerSchema = z.discriminatedUnion('type', [
  z
    .object({
      type: z.literal('Human'),
      value: z.string()
    })
    .strict(),
  z
    .object({
      type: z.literal('Ai'),
      value: DifficultySchema
    })
    .strict()
]);
export type Player = z.infer<typeof PlayerSchema>;

export const GameStatusSchema = z.enum([
  'in_progress',
  'finished',
  'cancelled'
]);
export type GameStatus = z.infer<typeof GameStatusSchema>;

export const CreateGameSchema = z
  .object({
    game_id: GameIdSchema,
    white: PlayerSchema,
    black: PlayerSchema,
    board: BoardSchema
  })
  .strict();
export type CreateGame = z.infer<typeof CreateGameSchema>;

export const MoveSchema = z.object({
  color: ColorSchema,
  mv: z.string(),
  board: BoardSchema,
  fen: z.string().optional()
});
export type Move = z.infer<typeof MoveSchema>;

export const GameMoveSchema = z.object({
  move_number: z.number(),
  color: ColorSchema,
  move_notation: z.string(),
  fen: z.string(),
  outcome: GameOutcomeSchema.nullable().optional()
});
export type GameMove = z.infer<typeof GameMoveSchema>;

export const GameSchema = CreateGameSchema.extend({
  fen: z.string().nullable().optional(),
  status: GameStatusSchema.optional(),
  resigner: ColorSchema.nullable().optional(),
  outcome: GameOutcomeSchema.nullable().optional(),
  created_at: z.string().optional(),
  finished_at: z.string().nullable().optional()
});
export type Game = z.infer<typeof GameSchema>;

export const GameOverviewSchema = CreateGameSchema.extend({
  fen: z.string().nullable().optional(),
  status: GameStatusSchema.optional(),
  resigner: ColorSchema.nullable().optional(),
  outcome: GameOutcomeSchema.nullable().optional(),
  created_at: z.string().optional(),
  finished_at: z.string().nullable().optional()
});
export type GameOverview = z.infer<typeof GameOverviewSchema>;

export const PaginatedGamesSchema = z.object({
  items: GameOverviewSchema.array(),
  next_cursor: z.string().nullable(),
  total_count: z.number().optional(),
  total_pages: z.number().optional(),
  page: z.number().optional(),
  per_page: z.number().optional()
});
export type PaginatedGames = z.infer<typeof PaginatedGamesSchema>;

export const ChallengeStatusSchema = z.enum([
  'pending',
  'accepted',
  'rejected'
]);
export type ChallengeStatus = z.infer<typeof ChallengeStatusSchema>;

export const ChallengeSchema = z.object({
  id: z.string(),
  challenger: z.string(),
  challenged: z.string(),
  wager_token: z.string().nullable(),
  wager_amount: z.string().nullable(),
  status: ChallengeStatusSchema,
  game_id: z.string().nullable(),
  created_at: z.string(),
  resolved_at: z.string().nullable()
});
export type Challenge = z.infer<typeof ChallengeSchema>;

export const AccountSchema = z.object({
  finishedGameIds: GameIdSchema.array()
});
export type Account = z.infer<typeof AccountSchema>;

export const AccountStatsSchema = z.object({
  account_id: z.string(),
  wins: z.number(),
  losses: z.number(),
  draws: z.number(),
  total_games: z.number()
});
export type AccountStats = z.infer<typeof AccountStatsSchema>;

export const AccountSearchResultSchema = AccountStatsSchema.extend({
  elo: z.number().nullable()
});
export type AccountSearchResult = z.infer<typeof AccountSearchResultSchema>;

export const GlobalStatsSchema = z.object({
  total_games: z.number(),
  active_games: z.number(),
  finished_games: z.number(),
  cancelled_games: z.number(),
  total_moves: z.number()
});
export type GlobalStats = z.infer<typeof GlobalStatsSchema>;

export const InfoSchema = z.object({
  lastBlockHeight: z.number()
});

export const ErrorSchema = z.object({
  error: z.string(),
  message: z.string().optional()
});

export const PushSubscriptionSchema = z.object({
  endpoint: z.string().url(),
  keys: z.object({
    p256dh: z.string(),
    auth: z.string()
  })
});
export type PushSubscriptionRequest = z.infer<typeof PushSubscriptionSchema>;

export const VapidPublicKeySchema = z.object({
  publicKey: z.string()
});

export const BetStatusSchema = z.enum(['pending', 'locked', 'resolved']);
export type BetStatus = z.infer<typeof BetStatusSchema>;

export const BetSchema = z.object({
  id: z.string(),
  bettor: z.string(),
  player_0: z.string(),
  player_1: z.string(),
  game_id: z.string().nullable(),
  token_id: z.string(),
  amount: z.string(),
  winner: z.string(),
  status: BetStatusSchema,
  payout: z.string().nullable(),
  created_at: z.string(),
  locked_at: z.string().nullable(),
  resolved_at: z.string().nullable()
});
export type Bet = z.infer<typeof BetSchema>;

export const PaginatedBetsSchema = z.object({
  items: BetSchema.array(),
  next_cursor: z.string().nullable()
});
export type PaginatedBets = z.infer<typeof PaginatedBetsSchema>;

export const BetStatsSchema = z.object({
  account_id: z.string(),
  total_wagered: z.string(),
  total_won: z.string(),
  total_bets: z.number(),
  won_bets: z.number()
});
export type BetStats = z.infer<typeof BetStatsSchema>;

export const BetLeaderboardEntrySchema = z.object({
  account_id: z.string(),
  total_wagered: z.string(),
  total_won: z.string(),
  total_bets: z.number(),
  won_bets: z.number()
});
export type BetLeaderboardEntry = z.infer<typeof BetLeaderboardEntrySchema>;

export const PaginatedChallengesSchema = z.object({
  items: ChallengeSchema.array(),
  next_cursor: z.string().nullable(),
  total_count: z.number().optional(),
  total_pages: z.number().optional(),
  page: z.number().optional(),
  per_page: z.number().optional()
});
export type PaginatedChallenges = z.infer<typeof PaginatedChallengesSchema>;

export const EloLeaderboardEntrySchema = z.object({
  rank: z.number(),
  account_id: z.string(),
  elo: z.number()
});

export const EloLeaderboardPageSchema = z.object({
  total: z.number(),
  page: z.number(),
  per_page: z.number(),
  total_pages: z.number(),
  entries: EloLeaderboardEntrySchema.array()
});

export const RankingEntrySchema = z.object({
  rank: z.number(),
  account_id: z.string(),
  elo: z.number().nullable(),
  ppp: z.string(),
  wins: z.number(),
  losses: z.number(),
  draws: z.number(),
  total_games: z.number()
});

export const RankingPageSchema = z.object({
  total: z.number(),
  page: z.number(),
  per_page: z.number(),
  total_pages: z.number(),
  entries: RankingEntrySchema.array()
});
