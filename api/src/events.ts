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
  board: BoardSchema
});
export type Move = z.infer<typeof MoveSchema>;

export const GameSchema = CreateGameSchema.extend({
  moves: MoveSchema.array(),
  resigner: ColorSchema.nullable().optional(),
  outcome: GameOutcomeSchema.nullable().optional()
});
export type Game = z.infer<typeof GameSchema>;

export const GameOverviewSchema = CreateGameSchema.extend({
  moves: MoveSchema.array().optional(),
  resigner: ColorSchema.nullable().optional(),
  outcome: GameOutcomeSchema.nullable().optional()
});
export type GameOverview = z.infer<typeof GameOverviewSchema>;

export const AccountSchema = z.object({
  finishedGameIds: GameIdSchema.array()
});
export type Account = z.infer<typeof AccountSchema>;

export const InfoSchema = z.object({
  lastBlockHeight: z.number()
});
