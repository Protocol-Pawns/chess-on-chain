
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
  Player,
  Timestamp,
  Result,
  Difficulty,
  PublicKey,
  Duration,
  U128,
  AccountId,
  Gas,
  TokenMetadata,
  FungibleTokenMetadata,
  GameId,
  ContractError,
  GameInfo,
  StorageUsage,
  Token,
  TokenId,
  StorageBalance,
  WrappedDuration,
  Base64VecU8,
  Color,
  NftContractMetadata,
  StorageBalanceBounds,
  GameOutcome,
  Balance,
} from "./types";

/**
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
/**
* 
* @contractMethod view
*/
export interface GetGameIds {
  args: {
    account_id: AccountId;
  };
  
}
export type GetGameIds__Result = Result<GameId[], ContractError>;
