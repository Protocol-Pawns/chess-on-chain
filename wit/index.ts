
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
  StorageBalanceBounds,
  Difficulty,
  Base64VecU8,
  StorageBalance,
  Gas,
  U128,
  Color,
  WrappedDuration,
  Token,
  Duration,
  TokenMetadata,
  Balance,
  AccountId,
  FungibleTokenMetadata,
  ContractError,
  GameId,
  PublicKey,
  NftContractMetadata,
  StorageUsage,
  GameOutcome,
  TokenId,
  Result,
  Timestamp,
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
export type CreateAiGame__Result = void;
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
export type PlayMove__Result = Result<GameOutcome | null, ContractError>;
