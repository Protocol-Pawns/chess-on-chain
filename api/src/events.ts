import { z } from 'zod';

import {
  Color as ColorAbi,
  Difficulty as DifficultyAbi,
  GameId as GameIdAbi,
  GameOutcome as GameOutcomeAbi,
  Player as PlayerAbi
} from '../../abi';

// This is used for some tricky type testing whether the manually
// generated type from Zod is equal to the type exported from ABI
type IfEquals<T, U, Y = unknown, N = never> =
  (<G>() => G extends T ? 1 : 2) extends <G>() => G extends U ? 1 : 2 ? Y : N;
declare const exactType: <T, U>(
  draft: T & IfEquals<T, U>,
  expected: U & IfEquals<T, U>
) => IfEquals<T, U>;

const zodGameId = z.tuple([z.number(), z.string(), z.nullable(z.string())]);
export type GameId = z.infer<typeof zodGameId>;
declare let gameId: GameId;
declare let gameIdAbi: GameIdAbi;
exactType(gameId, gameIdAbi);

const zodDifficulty = z.enum(['Easy', 'Medium', 'Hard']);
export type Difficulty = z.infer<typeof zodDifficulty>;
declare let difficulty: Difficulty;
declare let difficultyAbi: DifficultyAbi;
exactType(difficulty, difficultyAbi);

const zodColor = z.enum(['White', 'Black']);
export type Color = z.infer<typeof zodColor>;
declare let color: Color;
declare let colorAbi: ColorAbi;
exactType(color, colorAbi);

const zodBoard = z.string().array().length(8);

const zodGameOutcome = z.literal('Stalemate').or(
  z.object({
    Victory: zodColor
  })
);
export type GameOutcome = z.infer<typeof zodGameOutcome>;
declare let gameOutcome: GameOutcome;
declare let gameOutcomeAbi: GameOutcomeAbi;
exactType(gameOutcome, gameOutcomeAbi);

const zodPlayer = z
  .object({
    Human: z.string()
  })
  .or(
    z.object({
      Ai: zodDifficulty
    })
  );
export type Player = z.infer<typeof zodPlayer>;
declare let player: Player;
declare let playerAbi: PlayerAbi;
exactType(player, playerAbi);

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
  outcome?: GameOutcome;
};
export type Game = CreateGame & {
  moves: Move[];
  resigner?: Color;
  outcome?: GameOutcome;
};

const zodPlayMove = z
  .object({
    game_id: zodGameId,
    color: zodColor,
    mv: z.string(),
    board: zodBoard,
    outcome: zodGameOutcome.optional()
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
