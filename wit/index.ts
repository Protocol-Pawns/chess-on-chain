
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
  Difficulty,
  GameId,
  GameInfo,
  U128,
  Player,
  FungibleTokenMetadata,
  TokenId,
  StorageBalance,
  Color,
  WrappedDuration,
  Result,
  NftContractMetadata,
  Timestamp,
  AccountId,
  Gas,
  Balance,
  Token,
  StorageUsage,
  TokenMetadata,
  PublicKey,
  GameOutcome,
  Base64VecU8,
  StorageBalanceBounds,
  Duration,
  ContractError,
} from "./types";

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
* 
* @contractMethod change
*/
export interface CreateAiGame {
  args: {
    difficulty: Difficulty;
  };
  options: CallOptions
  
}
export type CreateAiGame__Result = GameId;
/**
* 
* @contractMethod change
*/
export interface PlayMove {
  args: {
    game_id: GameId;
    mv: string;
  };
  options: CallOptions
  
}
export type PlayMove__Result = Result<[GameOutcome | null, string], ContractError>;
/**
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
* 
* @contractMethod view
*/
export interface GameInfo {
  args: {
    game_id: GameId;
  };
  
}
export type GameInfo__Result = Result<GameInfo, ContractError>;
