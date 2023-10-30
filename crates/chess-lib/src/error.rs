use near_sdk::{borsh::BorshSerialize, AccountId, Balance, FunctionError};
use thiserror::Error;

use crate::ChallengeId;

#[derive(BorshSerialize, Debug, Error, FunctionError)]
#[borsh(crate = "near_sdk::borsh")]
pub enum ContractError {
    #[error("Contract already initialized")]
    AlreadyInitilized,
    #[error("Account {} not registered", _0)]
    AccountNotRegistered(AccountId),
    #[error("Account is playing and cannot be unregistered")]
    AccountIsPlaying,
    #[error("Maximum amount of open games reached")]
    MaxGamesReached,
    #[error("Maximum amount of open challenges reached")]
    MaxChallengesReached,
    #[error("Game does not exist")]
    GameNotExists,
    #[error("Cannot parse move {}", _0)]
    MoveParse(String),
    #[error("Illegal move")]
    IllegalMove,
    #[error("It is not your turn")]
    NotYourTurn,
    #[error("You are not a player from this game")]
    NotPlaying,
    #[error("Not enough NEAR deposit. Required: {}, actual: {}", _0, _1)]
    NotEnoughDeposit(Balance, Balance),
    #[error("Operation not supported")]
    OperationNotSupported,
    #[error("Unable to deserialize message")]
    Deserialize,
    #[error("Challenge with ID {} does not exist", _0)]
    ChallengeNotExists(ChallengeId),
    #[error("Only the challenger wallet ID can reject a challenge")]
    WrongChallengerId,
    #[error("Only the challenged wallet ID can accept or reject a challenge")]
    WrongChallengedId,
    #[error("Challenged wallet did not pay proper wager to accept challenge")]
    PaidWager,
    #[error("Challenger and challenged wallet ID can't be the same")]
    SelfChallenge,
    #[error("Challenge alread exists")]
    ChallengeExists,
    #[error("Game not yet cancellable. You still need to wait {} blocks", _0)]
    GameNotCancellable(u64),
    #[error("Game can only be cancelled, if it's not your turn")]
    CancelOnOpponentsTurn,
}
