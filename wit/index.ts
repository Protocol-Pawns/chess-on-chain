
import {
  u8,
  i8,
  u16,
  i16,
  u32,
  i32,
  u64,
  i64,
  f32,
  f64,
  CallOptions,
  U128,
  MoveStr,
  StorageUsage,
  TokenId,
  Balance,
  StorageBalance,
  GameOutcome,
  Color,
  Timestamp,
  Base64VecU8,
  Duration,
  FungibleTokenMetadata,
  WrappedDuration,
  Player,
  GameInfo,
  Result,
  StorageBalanceBounds,
  PublicKey,
  TokenMetadata,
  NftContractMetadata,
  GameId,
  ContractError,
  Token,
  Gas,
  AccountId,
  Difficulty,
} from "./types";

/**
* Requires exactly 0.05N (50000000000000000000000 yoctoNear) to pay for storage.
* 
* @contractMethod change
*/
export interface StorageDeposit {
  args: {
    account_id?: AccountId;
    registration_only?: boolean;
  };
  options: CallOptions
  
}
export type StorageDeposit__Result = StorageBalance;
/**
* 
* @contractMethod change
*/
export interface StorageWithdraw {
  args: {
    amount?: U128;
  };
  options: CallOptions
  
}
export type StorageWithdraw__Result = StorageBalance;
/**
* 
* @contractMethod change
*/
export interface StorageUnregister {
  args: {
    force?: boolean;
  };
  options: CallOptions
  
}
export type StorageUnregister__Result = boolean;
/**
* 
* @contractMethod view
*/
export interface StorageBalanceBounds {
  args: {};
  
}
export type StorageBalanceBounds__Result = StorageBalanceBounds;
/**
* 
* @contractMethod view
*/
export interface StorageBalanceOf {
  args: {
    account_id: AccountId;
  };
  
}
export type StorageBalanceOf__Result = StorageBalance | null;
/**
* 
* @contractMethod change
*/
export interface New {
  args: {};
  options: CallOptions
  
}
export type New__Result = void;
/**
* Create a new game against an AI player.
* 
* Returns game ID, which is necessary to play the game.
* You can only have 5 open games due to storage limitations.
* If you reach the limit you can call `resign` method.
* 
* Before you can play a game you need to pay `storage_deposit`.
* 
* @contractMethod change
*/
export interface CreateAiGame {
  args: {
    difficulty: Difficulty;
  };
  options: CallOptions
  
}
export type CreateAiGame__Result = Result<GameId, ContractError>;
/**
* Plays a move.
* 
* Only works, if it is your turn. Panics otherwise.
* 
* @contractMethod change
*/
export interface PlayMove {
  args: {
    game_id: GameId;
    mv: MoveStr;
  };
  options: CallOptions
  
}
export type PlayMove__Result = Result<[GameOutcome | null, string], ContractError>;
/**
* Resigns a game.
* 
* Can be called even if it is not your turn.
* You might need to call this if a game is stuck and the AI refuses to work.
* You can also only have 5 open games due to storage limitations.
* 
* @contractMethod change
*/
export interface Resign {
  args: {
    game_id: GameId;
  };
  options: CallOptions
  
}
export type Resign__Result = Result<[], ContractError>;
/**
* Renders a game as a string.
* 
* @contractMethod view
*/
export interface RenderBoard {
  args: {
    game_id: GameId;
  };
  
}
export type RenderBoard__Result = Result<string, ContractError>;
/**
* Returns information about a game including players and turn color.
* 
* @contractMethod view
*/
export interface GameInfo {
  args: {
    game_id: GameId;
  };
  
}
export type GameInfo__Result = Result<GameInfo, ContractError>;
/**
* Returns all open game IDs for given wallet ID.
* 
* @contractMethod view
*/
export interface GetGameIds {
  args: {
    account_id: AccountId;
  };
  
}
export type GetGameIds__Result = Result<GameId[], ContractError>;
