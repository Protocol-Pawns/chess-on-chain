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
  moves: MoveSchema.array(),
  fen: z.string().nullable().optional(),
  status: GameStatusSchema.optional(),
  resigner: ColorSchema.nullable().optional(),
  outcome: GameOutcomeSchema.nullable().optional(),
  created_at: z.string().optional(),
  finished_at: z.string().nullable().optional()
});
export type Game = z.infer<typeof GameSchema>;

export const GameOverviewSchema = CreateGameSchema.extend({
  moves: MoveSchema.array().optional(),
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
  next_cursor: z.string().nullable()
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

export const LeaderboardEntrySchema = z.object({
  account_id: z.string(),
  wins: z.number(),
  losses: z.number(),
  draws: z.number(),
  total_games: z.number(),
  win_rate: z.number()
});
export type LeaderboardEntry = z.infer<typeof LeaderboardEntrySchema>;

export const PaginatedLeaderboardSchema = z.object({
  items: LeaderboardEntrySchema.array(),
  next_cursor: z.string().nullable()
});
export type PaginatedLeaderboard = z.infer<typeof PaginatedLeaderboardSchema>;

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
