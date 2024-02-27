import { z } from 'zod';

import { Optional } from './types';

const zodGameId = z.tuple([z.number(), z.string(), z.nullable(z.string())]);
export type GameId = z.infer<typeof zodGameId>;

const zodDifficulty = z.enum(['Easy', 'Medium', 'Hard']);
export type Difficulty = z.infer<typeof zodDifficulty>;

const zodColor = z.enum(['White', 'Black']);
export type Color = z.infer<typeof zodColor>;

const zodBoard = z.string().array().length(8);

const zodGameOutcome = z.literal('Stalemate').or(
  z.object({
    Victory: zodColor
  })
);
export type GameOutcome = z.infer<typeof zodGameOutcome>;

const zodPlayer = z.discriminatedUnion('type', [
  z
    .object({
      type: z.literal('Human'),
      value: z.string()
    })
    .strict(),
  z
    .object({
      type: z.literal('Ai'),
      value: zodDifficulty
    })
    .strict()
]);
export type Player = z.infer<typeof zodPlayer>;

const zodCreateGame = z
  .object({
    game_id: zodGameId,
    white: zodPlayer,
    black: zodPlayer,
    board: zodBoard
  })
  .strict();
export type CreateGame = z.infer<typeof zodCreateGame>;

export type Move = {
  color: Color;
  mv: string;
  board: string[];
};

export type Game = CreateGame & {
  moves: Move[];
  resigner?: Color | null;
  outcome?: GameOutcome | null;
};

export type GameOverview = Optional<Game, 'moves'>;

export type Account = {
  finishedGameIds: GameId[];
};

const zodPlayMove = z
  .object({
    game_id: zodGameId,
    color: zodColor,
    mv: z.string(),
    board: zodBoard,
    outcome: zodGameOutcome.nullish()
  })
  .strict();
export type PlayMove = z.infer<typeof zodPlayMove>;

const zodResignGame = z
  .object({
    game_id: zodGameId,
    resigner: zodColor,
    outcome: zodGameOutcome
  })
  .strict();
export type ResignGame = z.infer<typeof zodResignGame>;

const zodCancelGame = z
  .object({
    game_id: zodGameId,
    cancelled_by: z.string()
  })
  .strict();
export type CancelGame = z.infer<typeof zodCancelGame>;

export const zodBatchEvent = z.object({
  block_height: z.number(),
  timestamp: z.number(),
  events: z
    .discriminatedUnion('event', [
      z
        .object({
          event: z.literal('create_game'),
          data: zodCreateGame
        })
        .strict(),
      z
        .object({
          event: z.literal('play_move'),
          data: zodPlayMove
        })
        .strict(),
      z
        .object({
          event: z.literal('resign_game'),
          data: zodResignGame
        })
        .strict(),
      z
        .object({
          event: z.literal('cancel_game'),
          data: zodCancelGame
        })
        .strict()
    ])
    .array()
});

export type BatchEvent = z.infer<typeof zodBatchEvent>;
