
/** 
* @minimum 0
* @maximum 18446744073709551615
* @asType integer
*/
export type u64 = number;
/** 
* @minimum -9223372036854775808
* @maximum 9223372036854775807
* @asType integer
*/
export type i64 = number;

/**
* @minimum  0 
* @maximum 255
* @asType integer
* */
export type u8 = number;
/**
* @minimum  -128 
* @maximum 127
* @asType integer
* */
export type i8 = number;
/**
* @minimum  0 
* @maximum 65535
* @asType integer
* */
export type u16 = number;
/**
* @minimum -32768 
* @maximum 32767
* @asType integer
* */
export type i16 = number;
/**
* @minimum 0 
* @maximum 4294967295
* @asType integer
* */
export type u32 = number;
/**
* @minimum 0 
* @maximum 4294967295
* @asType integer
* */
export type usize = number;
/**
* @minimum  -2147483648 
* @maximum 2147483647
* @asType integer
* */
export type i32 = number;

/**
* @minimum -3.40282347E+38
* @maximum 3.40282347E+38
*/
export type f32 = number;

/**
* @minimum -1.7976931348623157E+308
* @maximum 1.7976931348623157E+308
*/
export type f64 = number;

export type CallOptions = {
  /** Units in gas
  * @pattern [0-9]+
  * @default "30000000000000"
  */
  gas?: string;
  /** Units in yoctoNear
  * @default "0"
  */
  attachedDeposit?: Balance;
}


export type ContractError = ContractErrorAlreadyInitilized | ContractErrorAccountNotRegistered | ContractErrorAccountIsPlaying | ContractErrorGameNotExists | ContractErrorMoveParse | ContractErrorIllegalMove | ContractErrorNotYourTurn | ContractErrorNotPlaying | ContractErrorNotEnoughDeposit | ContractErrorOperationNotSupported;
export interface ContractErrorAlreadyInitilized {
  tag: "already-initilized",
}
export interface ContractErrorAccountNotRegistered {
  tag: "account-not-registered",
  val: AccountId,
}
export interface ContractErrorAccountIsPlaying {
  tag: "account-is-playing",
}
export interface ContractErrorGameNotExists {
  tag: "game-not-exists",
}
export interface ContractErrorMoveParse {
  tag: "move-parse",
  val: string,
}
export interface ContractErrorIllegalMove {
  tag: "illegal-move",
}
export interface ContractErrorNotYourTurn {
  tag: "not-your-turn",
}
export interface ContractErrorNotPlaying {
  tag: "not-playing",
}
export interface ContractErrorNotEnoughDeposit {
  tag: "not-enough-deposit",
  val: [Balance, Balance],
}
export interface ContractErrorOperationNotSupported {
  tag: "operation-not-supported",
}
export type GameId = [u64, AccountId, AccountId | null];
export type Player = PlayerHuman | PlayerAi;
export interface PlayerHuman {
  tag: "human",
  val: AccountId,
}
export interface PlayerAi {
  tag: "ai",
  val: Difficulty,
}
export enum Difficulty {
  Easy = "Easy",
  Medium = "Medium",
}
export interface GameInfo {
  white: Player;
  black: Player;
  turn_color: Color;
}
export type GameOutcome = GameOutcomeVictory | GameOutcomeStalemate;
export interface GameOutcomeVictory {
  tag: "victory",
  val: Color,
}
export interface GameOutcomeStalemate {
  tag: "stalemate",
}
/**
* The color of a piece.
*/
export enum Color {
  White = "White",
  Black = "Black",
}
/**
* StorageUsage is used to count the amount of storage used by a contract.
*/
export type StorageUsage = u64;
/**
* Balance is a type for storing amounts of tokens, specified in yoctoNEAR.
*/
export type Balance = U128;
/**
* Represents the amount of NEAR tokens in "gas units" which are used to fund transactions.
*/
export type Gas = u64;
/**
* base64 string.
*/
export type Base64VecU8 = string;
/**
* Raw type for duration in nanoseconds
*/
export type Duration = u64;
/**
* @minLength 2
* @maxLength 64
* @pattern ^(([a-z\d]+[-_])*[a-z\d]+\.)*([a-z\d]+[-_])*[a-z\d]+$
*/
export type AccountId = string;
/**
* String representation of a u128-bit integer
* @pattern ^[0-9]+$
*/
export type U128 = string;
/**
* Public key in a binary format with base58 string serialization with human-readable curve.
* The key types currently supported are `secp256k1` and `ed25519`.
* 
* Ed25519 public keys accepted are 32 bytes and secp256k1 keys are the uncompressed 64 format.
*/
export type PublicKey = string;
/**
* Raw type for timestamp in nanoseconds
*/
export type Timestamp = u64;
/**
* In this implementation, the Token struct takes two extensions standards (metadata and approval) as optional fields, as they are frequently used in modern NFTs.
*/
export interface Token {
  token_id: TokenId;
  owner_id: AccountId;
  metadata?: TokenMetadata;
  approved_account_ids?: Record<AccountId, u64>;
}
export interface FungibleTokenMetadata {
  spec: string;
  name: string;
  symbol: string;
  icon?: string;
  reference?: string;
  reference_hash?: Base64VecU8;
  decimals: u8;
}
/**
* Note that token IDs for NFTs are strings on NEAR. It's still fine to use autoincrementing numbers as unique IDs if desired, but they should be stringified. This is to make IDs more future-proof as chain-agnostic conventions and standards arise, and allows for more flexibility with considerations like bridging NFTs across chains, etc.
*/
export type TokenId = string;
/**
* Metadata for the NFT contract itself.
*/
export interface NftContractMetadata {
  spec: string;
  name: string;
  symbol: string;
  icon?: string;
  base_uri?: string;
  reference?: string;
  reference_hash?: Base64VecU8;
}
export interface StorageBalanceBounds {
  min: U128;
  max?: U128;
}
/**
* Metadata on the individual token level.
*/
export interface TokenMetadata {
  title?: string;
  description?: string;
  media?: string;
  media_hash?: Base64VecU8;
  copies?: u64;
  issued_at?: string;
  expires_at?: string;
  starts_at?: string;
  updated_at?: string;
  extra?: string;
  reference?: string;
  reference_hash?: Base64VecU8;
}
export interface StorageBalance {
  total: U128;
  available: U128;
}
export type WrappedDuration = string;
export type Result<T, E> = { tag: "ok", val: T } | { tag: "err", val: E };
