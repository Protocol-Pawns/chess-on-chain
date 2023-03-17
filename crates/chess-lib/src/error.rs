use near_sdk::{
    borsh::{self, BorshSerialize},
    AccountId, Balance, FunctionError,
};
use thiserror::Error;
use witgen::witgen;

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
}
