use near_sdk::{
    borsh::{self, BorshSerialize},
    FunctionError,
};
use thiserror::Error;
use witgen::witgen;

#[derive(BorshSerialize, Debug, Error, FunctionError)]
#[witgen]
pub enum ContractError {
    #[error("Contract already initialized")]
    AlreadyInitilized,
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
}
