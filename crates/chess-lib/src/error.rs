use near_sdk::{
    borsh::{self, BorshSerialize},
    AccountId, Balance, FunctionError,
};
use thiserror::Error;
use witgen::witgen;

use crate::ChallengeId;

#[derive(BorshSerialize, Debug, Error, FunctionError)]
#[witgen]
pub enum ContractError {
    #[error("Contract already initialized")]
    AlreadyInitilized,
    #[error("Account {} not registered", _0)]
    AccountNotRegistered(AccountId),
    #[error("Account is playing and cannot be unregistered")]
    AccountIsPlaying,
    #[error("Maximum amount of open games reached")]
    MaxGamesReached,
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
}
