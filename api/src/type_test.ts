import type {
  Color as ColorAbi,
  Difficulty as DifficultyAbi,
  GameId as GameIdAbi,
  GameOutcome as GameOutcomeAbi,
  Player as PlayerAbi
} from '../../abi';

import { Color, Difficulty, GameId, GameOutcome, Player } from './events';

// This is used for some tricky type testing whether the manually
// generated type from Zod is equal to the type exported from ABI
type IfEquals<T, U, Y = unknown, N = never> =
  (<G>() => G extends T ? 1 : 2) extends <G>() => G extends U ? 1 : 2 ? Y : N;
declare const exactType: <T, U>(
  draft: T & IfEquals<T, U>,
  expected: U & IfEquals<T, U>
) => IfEquals<T, U>;

declare let gameId: GameId;
declare let gameIdAbi: GameIdAbi;
exactType(gameId, gameIdAbi);

declare let difficulty: Difficulty;
declare let difficultyAbi: DifficultyAbi;
exactType(difficulty, difficultyAbi);

declare let color: Color;
declare let colorAbi: ColorAbi;
exactType(color, colorAbi);

declare let gameOutcome: GameOutcome;
declare let gameOutcomeAbi: GameOutcomeAbi;
exactType(gameOutcome, gameOutcomeAbi);

declare let player: Player;
declare let playerAbi: PlayerAbi;
exactType(player, playerAbi);
