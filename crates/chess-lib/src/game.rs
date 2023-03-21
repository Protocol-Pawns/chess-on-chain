use crate::ContractError;
use chess_engine::{Board, Color, GameResult, Move, Piece, Position};
use near_sdk::{
    borsh::{self, BorshDeserialize, BorshSerialize},
    env, log, near_bindgen,
    serde::{Deserialize, Serialize},
    AccountId,
};
use witgen::witgen;

/// Unique game ID, which consists of:
///
/// - block height
/// - wallet ID, e.g. "my-wallet.near"
/// - enemy wallet ID if player or empty if AI
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

#[derive(BorshDeserialize, BorshSerialize, Clone, Serialize)]
#[serde(crate = "near_sdk::serde")]
#[witgen]
pub enum Player {
    Human(AccountId),
    Ai(Difficulty),
}

/// AI difficulty setting.
///
/// The AI uses the [Minimax algorithm, along with Alpha-Beta pruning](https://github.com/Tarnadas/chess-engine#how-does-it-work)
/// The higher the difficulty the more moves will be calculated in advance.
///
/// Please be aware, that gas usage increases on higher difficulties:
/// - Easy: ~8TGas
/// - Medium: ~30TGas
/// - Hard: ~110TGas
#[derive(BorshDeserialize, BorshSerialize, Clone, Deserialize, Serialize)]
#[serde(crate = "near_sdk::serde")]
#[witgen]

pub enum Difficulty {
    Easy,
    Medium,
    Hard,
}

#[near_bindgen]
#[derive(BorshDeserialize, BorshSerialize)]
pub struct Game {
    pub white: Player,
    pub black: Player,
    pub board: Board,
}

#[derive(Serialize)]
#[serde(crate = "near_sdk::serde")]
#[witgen]
pub struct GameInfo {
    pub white: Player,
    pub black: Player,
    pub turn_color: Color,
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

    pub fn is_turn(&self, account_id: AccountId) -> bool {
        let player = match self.board.get_turn_color() {
            Color::White => &self.white,
            Color::Black => &self.black,
        };
        if let Player::Human(id) = player {
            id == &account_id
        } else {
            false
        }
    }

    pub fn is_player(&self, account_id: AccountId) -> bool {
        if let Player::Human(id) = &self.white {
            if id == &account_id {
                return true;
            }
        }
        if let Player::Human(id) = &self.black {
            if id == &account_id {
                return true;
            }
        }
        false
    }

    pub fn play_move(&mut self, mv: Move) -> Result<Option<GameOutcome>, ContractError> {
        let turn_color = self.board.get_turn_color();
        log!("{}: {}", turn_color, mv);
        let outcome = match self.board.play_move(mv) {
            GameResult::Continuing(board) => {
                let (board, outcome) = if let Player::Ai(difficulty) = &self.black {
                    let depths = match difficulty {
                        Difficulty::Easy => vec![24],
                        Difficulty::Medium => vec![20, 16],
                        Difficulty::Hard => vec![16, 12, 8],
                    };
                    let (ai_mv, _, _) = board.get_next_move(&depths, env::random_seed_array());
                    log!("Black: {}", ai_mv);
                    match board.play_move(ai_mv) {
                        GameResult::Continuing(board) => (board, None),
                        GameResult::Victory(color) => {
                            log!("{} won the match", color);
                            (board, Some(GameOutcome::Victory(color)))
                        }
                        GameResult::Stalemate => {
                            log!("Draw due to stalement");
                            (board, Some(GameOutcome::Stalemate))
                        }
                        GameResult::IllegalMove(_) => return Err(ContractError::IllegalMove),
                    }
                } else {
                    (board, None)
                };
                self.board = board;
                outcome
            }
            GameResult::Victory(color) => {
                log!("{} won the match", color);
                Some(GameOutcome::Victory(color))
            }
            GameResult::Stalemate => {
                log!("Draw due to stalement");
                Some(GameOutcome::Stalemate)
            }
            GameResult::IllegalMove(_) => return Err(ContractError::IllegalMove),
        };
        Ok(outcome)
    }

    pub fn render_board(&self) -> String {
        (-1..8)
            .rev()
            .flat_map(|row| {
                (-1..9).map(move |col| -> char {
                    if col == -1 && row == -1 {
                        ' '
                    } else if col == -1 {
                        (b'1' + row as u8) as char
                    } else if col == 8 {
                        '\n'
                    } else if row == -1 {
                        (b'A' + col as u8) as char
                    } else if let Some(piece) = self.board.get_piece(Position::new(row, col)) {
                        match piece {
                            Piece::King(Color::Black, _) => '♚',
                            Piece::King(Color::White, _) => '♔',
                            Piece::Queen(Color::Black, _) => '♛',
                            Piece::Queen(Color::White, _) => '♕',
                            Piece::Rook(Color::Black, _) => '♜',
                            Piece::Rook(Color::White, _) => '♖',
                            Piece::Bishop(Color::Black, _) => '♝',
                            Piece::Bishop(Color::White, _) => '♗',
                            Piece::Knight(Color::Black, _) => '♞',
                            Piece::Knight(Color::White, _) => '♘',
                            Piece::Pawn(Color::Black, _) => '♟',
                            Piece::Pawn(Color::White, _) => '♙',
                        }
                    } else if (row + col) % 2 == 0 {
                        '⬜'
                    } else {
                        '⬛'
                    }
                })
            })
            .collect()
    }
}
