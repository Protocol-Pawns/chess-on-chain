use crate::{ChessEvent, ContractError};
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
    Debug,
    Hash,
    Ord,
    PartialOrd,
    PartialEq,
    Eq,
)]
#[serde(crate = "near_sdk::serde")]
#[witgen]
pub struct GameId(pub u64, pub AccountId, pub Option<AccountId>);

#[derive(BorshDeserialize, BorshSerialize, Clone, Debug, Serialize)]
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
#[derive(BorshDeserialize, BorshSerialize, Clone, Debug, Deserialize, Serialize)]
#[serde(crate = "near_sdk::serde")]
#[witgen]

pub enum Difficulty {
    Easy,
    Medium,
    Hard,
}

#[derive(BorshDeserialize, BorshSerialize)]
pub enum Game {
    V1(GameV1),
}

#[near_bindgen]
#[derive(BorshDeserialize, BorshSerialize)]
pub struct GameV1 {
    pub game_id: GameId,
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

#[derive(BorshDeserialize, BorshSerialize, Clone, Debug, Deserialize, Serialize)]
#[serde(crate = "near_sdk::serde")]
#[witgen]

pub enum GameOutcome {
    Victory(Color),
    Stalemate,
}

impl Game {
    pub fn new(game_id: GameId, white: Player, black: Player) -> Self {
        Game::V1(GameV1 {
            game_id,
            white,
            black,
            board: Board::default(),
        })
    }

    pub fn get_game_id(&self) -> &GameId {
        let Game::V1(game) = self;
        &game.game_id
    }

    pub fn get_white(&self) -> &Player {
        let Game::V1(game) = self;
        &game.white
    }

    pub fn get_black(&self) -> &Player {
        let Game::V1(game) = self;
        &game.black
    }

    pub fn get_board(&self) -> &Board {
        let Game::V1(game) = self;
        &game.board
    }

    pub fn is_turn(&self, account_id: AccountId) -> bool {
        let Game::V1(game) = self;
        let player = match game.board.get_turn_color() {
            Color::White => &game.white,
            Color::Black => &game.black,
        };
        if let Player::Human(id) = player {
            id == &account_id
        } else {
            false
        }
    }

    pub fn is_player(&self, account_id: AccountId) -> bool {
        let Game::V1(game) = self;
        if let Player::Human(id) = &game.white {
            if id == &account_id {
                return true;
            }
        }
        if let Player::Human(id) = &game.black {
            if id == &account_id {
                return true;
            }
        }
        false
    }

    pub fn play_move(&mut self, mv: Move) -> Result<Option<GameOutcome>, ContractError> {
        let Game::V1(game) = self;
        let turn_color = game.board.get_turn_color();
        let event = ChessEvent::PlayMove {
            game_id: game.game_id.clone(),
            color: turn_color,
            mv: mv.to_string(),
        };
        event.emit();
        let outcome = match game.board.play_move(mv) {
            GameResult::Continuing(board) => {
                let (board, outcome) = if let Player::Ai(difficulty) = &game.black {
                    let depths = match difficulty {
                        Difficulty::Easy => vec![24],
                        Difficulty::Medium => vec![20, 16],
                        Difficulty::Hard => vec![16, 12, 8],
                    };
                    let (ai_mv, _, _) = board.get_next_move(&depths, env::random_seed_array());
                    log!("Black: {}", ai_mv);
                    let event = ChessEvent::PlayMove {
                        game_id: game.game_id.clone(),
                        color: Color::Black,
                        mv: ai_mv.to_string(),
                    };
                    event.emit();
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
                game.board = board;
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

    pub fn get_board_state(&self) -> [String; 8] {
        let Game::V1(game) = self;
        (0..8)
            .map(|row| -> String {
                (0..8)
                    .map(move |col| -> char {
                        if let Some(piece) = game.board.get_piece(Position::new(row, col)) {
                            match piece {
                                Piece::King(Color::Black, _) => 'k',
                                Piece::King(Color::White, _) => 'K',
                                Piece::Queen(Color::Black, _) => 'q',
                                Piece::Queen(Color::White, _) => 'Q',
                                Piece::Rook(Color::Black, _) => 'r',
                                Piece::Rook(Color::White, _) => 'R',
                                Piece::Bishop(Color::Black, _) => 'b',
                                Piece::Bishop(Color::White, _) => 'B',
                                Piece::Knight(Color::Black, _) => 'n',
                                Piece::Knight(Color::White, _) => 'N',
                                Piece::Pawn(Color::Black, _) => 'p',
                                Piece::Pawn(Color::White, _) => 'P',
                            }
                        } else {
                            ' '
                        }
                    })
                    .collect()
            })
            .collect::<Vec<_>>()
            .try_into()
            .unwrap()
    }

    pub fn render_board(&self) -> String {
        let Game::V1(game) = self;
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
                    } else if let Some(piece) = game.board.get_piece(Position::new(row, col)) {
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
