use crate::ContractError;
use chess_engine::{Board, Color, Evaluate, GameResult, Move};
use near_sdk::{
    borsh::{self, BorshDeserialize, BorshSerialize},
    near_bindgen,
    serde::{Deserialize, Serialize},
    AccountId,
};
use witgen::witgen;

#[derive(
    BorshDeserialize,
    BorshSerialize,
    Deserialize,
    Serialize,
    Clone,
    Hash,
    Ord,
    PartialOrd,
    PartialEq,
    Eq,
)]
#[serde(crate = "near_sdk::serde")]
#[witgen]
pub struct GameId(pub u64, pub AccountId, pub Option<AccountId>);

#[derive(BorshDeserialize, BorshSerialize)]
pub enum Player {
    Human(AccountId),
    Ai(Difficulty),
}

#[derive(BorshDeserialize, BorshSerialize, Deserialize, Serialize)]
#[serde(crate = "near_sdk::serde")]
#[witgen]

pub enum Difficulty {
    Easy,
    Medium,
    Hard,
    VeryHard,
}

#[near_bindgen]
#[derive(BorshDeserialize, BorshSerialize)]
pub struct Game {
    white: Player,
    black: Player,
    board: Board,
}

#[derive(BorshDeserialize, BorshSerialize, Deserialize, Serialize)]
#[serde(crate = "near_sdk::serde")]
#[witgen]

pub enum GameOutcome {
    Victory(Color),
    Stalemate,
}

impl Game {
    pub fn new(white: Player, black: Player) -> Self {
        Self {
            white,
            black,
            board: Board::default(),
        }
    }

    pub fn play_move(&mut self, mv: Move) -> Result<Option<GameOutcome>, ContractError> {
        let outcome = match self.board.play_move(mv) {
            GameResult::Continuing(board) => {
                let (board, outcome) = if let Player::Ai(difficulty) = &self.black {
                    let depth = match difficulty {
                        Difficulty::Easy => 1,
                        Difficulty::Medium => 2,
                        Difficulty::Hard => 3,
                        Difficulty::VeryHard => 4,
                    };
                    let (ai_mv, _, _) = self.board.get_best_next_move(depth);
                    match self.board.play_move(ai_mv) {
                        GameResult::Continuing(board) => (board, None),
                        GameResult::Victory(color) => (board, Some(GameOutcome::Victory(color))),
                        GameResult::Stalemate => (board, Some(GameOutcome::Stalemate)),
                        GameResult::IllegalMove(_) => return Err(ContractError::IllegalMove),
                    }
                } else {
                    (board, None)
                };
                self.board = board;
                outcome
            }
            GameResult::Victory(color) => Some(GameOutcome::Victory(color)),
            GameResult::Stalemate => Some(GameOutcome::Stalemate),
            GameResult::IllegalMove(_) => return Err(ContractError::IllegalMove),
        };
        Ok(outcome)
    }
}
