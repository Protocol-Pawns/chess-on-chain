use crate::{Account, Chess, ChessEvent, ContractError, Wager};
use chess_engine::{Board, Color, GameResult, Move, Piece, Position};
use near_sdk::{
    borsh::{BorshDeserialize, BorshSerialize},
    env, near_bindgen,
    serde::{Deserialize, Serialize},
    AccountId,
};

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
#[borsh(crate = "near_sdk::borsh")]
pub struct GameId(pub u64, pub AccountId, pub Option<AccountId>);

#[derive(BorshDeserialize, BorshSerialize, Clone, Debug, Serialize, Deserialize)]
#[serde(crate = "near_sdk::serde")]
#[borsh(crate = "near_sdk::borsh")]
pub enum Player {
    Human(AccountId),
    Ai(Difficulty),
}

impl Player {
    pub fn get_account_id(&self) -> Option<AccountId> {
        match self {
            Player::Human(account_id) => Some(account_id.clone()),
            Player::Ai(_) => None,
        }
    }

    pub fn as_account_mut<'a>(&self, chess: &'a mut Chess) -> Option<&'a mut Account> {
        match self {
            Player::Human(account_id) => Some(chess.accounts.get_mut(account_id).unwrap()),
            Player::Ai(_) => None,
        }
    }

    pub fn is_human(&self) -> bool {
        matches!(self, Self::Human(_))
    }
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
#[borsh(crate = "near_sdk::borsh")]

pub enum Difficulty {
    Easy,
    Medium,
    Hard,
}

#[derive(BorshDeserialize, BorshSerialize)]
#[borsh(crate = "near_sdk::borsh")]
pub enum Game {
    V1(()),
    V2(()),
    V3(GameV3),
    V4(GameV4),
}

#[near_bindgen]
#[derive(BorshDeserialize, BorshSerialize)]
#[borsh(crate = "near_sdk::borsh")]
pub struct GameV4 {
    game_id: GameId,
    white: Player,
    black: Player,
    board: Board,
    wager: Wager,
    last_move_block_height: u64,
    has_bets: bool,
}

#[near_bindgen]
#[derive(BorshDeserialize, BorshSerialize)]
#[borsh(crate = "near_sdk::borsh")]
pub struct GameV3 {
    game_id: GameId,
    white: Player,
    black: Player,
    board: Board,
    wager: Wager,
    last_move_block_height: u64,
}

#[derive(Deserialize, Serialize)]
#[serde(crate = "near_sdk::serde")]
pub struct GameInfo {
    pub white: Player,
    pub black: Player,
    pub turn_color: Color,
    pub last_block_height: u64,
    pub has_bets: bool,
}

#[derive(BorshDeserialize, BorshSerialize, Clone, Debug, Deserialize, Serialize, PartialEq, Eq)]
#[serde(crate = "near_sdk::serde")]
#[borsh(crate = "near_sdk::borsh")]
pub enum GameOutcome {
    Victory(Color),
    Stalemate,
}

impl Game {
    pub fn new(white: Player, black: Player, wager: Wager, has_bets: bool) -> Self {
        let block_height = env::block_height();
        let game_id = GameId(
            block_height,
            white.get_account_id().unwrap(),
            black.get_account_id(),
        );
        Game::V4(GameV4 {
            game_id,
            white,
            black,
            board: Board::default(),
            wager,
            last_move_block_height: env::block_height(),
            has_bets,
        })
    }

    pub fn migrate(self) -> Self {
        if let Self::V3(GameV3 {
            game_id,
            white,
            black,
            board,
            last_move_block_height,
            wager,
        }) = self
        {
            Self::V4(GameV4 {
                game_id,
                white,
                black,
                board,
                wager,
                last_move_block_height,
                has_bets: false,
            })
        } else {
            self
        }
    }

    pub fn get_game_id(&self) -> &GameId {
        let Game::V4(game) = self else {
            panic!("migration required")
        };
        &game.game_id
    }

    pub fn get_white(&self) -> &Player {
        let Game::V4(game) = self else {
            panic!("migration required")
        };
        &game.white
    }

    pub fn get_black(&self) -> &Player {
        let Game::V4(game) = self else {
            panic!("migration required")
        };
        &game.black
    }

    pub fn get_board(&self) -> &Board {
        let Game::V4(game) = self else {
            panic!("migration required")
        };
        &game.board
    }

    pub fn get_wager(&self) -> &Wager {
        let Game::V4(game) = self else {
            panic!("migration required")
        };
        &game.wager
    }

    pub fn get_last_block_height(&self) -> u64 {
        let Game::V4(game) = self else {
            panic!("migration required")
        };
        game.last_move_block_height
    }

    pub fn is_turn(&self, account_id: &AccountId) -> bool {
        let Game::V4(game) = self else {
            panic!("migration required")
        };
        let player = match game.board.get_turn_color() {
            Color::White => &game.white,
            Color::Black => &game.black,
        };
        if let Player::Human(id) = player {
            id == account_id
        } else {
            false
        }
    }

    pub fn is_player(&self, account_id: &AccountId) -> bool {
        let Game::V4(game) = self else {
            panic!("migration required")
        };
        if let Player::Human(id) = &game.white {
            if id == account_id {
                return true;
            }
        }
        if let Player::Human(id) = &game.black {
            if id == account_id {
                return true;
            }
        }
        false
    }

    pub fn has_bets(&self) -> bool {
        let Game::V4(game) = self else {
            panic!("migration required")
        };
        game.has_bets
    }

    #[allow(clippy::type_complexity)]
    pub fn play_move(
        &mut self,
        mv: Move,
    ) -> Result<(Option<(GameOutcome, [String; 8])>, Color), ContractError> {
        let Game::V4(game) = self else {
            panic!("migration required")
        };
        let turn_color = game.board.get_turn_color();
        let event = ChessEvent::PlayMove {
            game_id: game.game_id.clone(),
            color: turn_color,
            mv: mv.to_string(),
        };
        event.emit();
        let outcome_with_board = match game.board.play_move(mv) {
            GameResult::Continuing(board) => {
                let event = ChessEvent::ChangeBoard {
                    game_id: game.game_id.clone(),
                    board: Self::_get_board_state(&board),
                };
                event.emit();
                let (board, outcome) = if let Player::Ai(difficulty) = &game.black {
                    let depths = match difficulty {
                        Difficulty::Easy => vec![24],
                        Difficulty::Medium => vec![20, 16],
                        Difficulty::Hard => vec![16, 12, 8],
                    };
                    let (ai_mv, _, _) = board.get_next_move(&depths, env::random_seed_array());
                    let event = ChessEvent::PlayMove {
                        game_id: game.game_id.clone(),
                        color: Color::Black,
                        mv: ai_mv.to_string(),
                    };
                    event.emit();
                    match board.play_move(ai_mv) {
                        GameResult::Continuing(board) => {
                            let event = ChessEvent::ChangeBoard {
                                game_id: game.game_id.clone(),
                                board: Self::_get_board_state(&board),
                            };
                            event.emit();
                            (board, None)
                        }
                        GameResult::Victory(color) => {
                            let board_state = Self::_get_board_state(&board.apply_eval_move(ai_mv));
                            let event = ChessEvent::FinishGame {
                                game_id: game.game_id.clone(),
                                outcome: GameOutcome::Victory(color),
                                board: board_state.clone(),
                            };
                            event.emit();
                            (board, Some((GameOutcome::Victory(color), board_state)))
                        }
                        GameResult::Stalemate => {
                            let board_state = Self::_get_board_state(&board.apply_eval_move(ai_mv));
                            let event = ChessEvent::FinishGame {
                                game_id: game.game_id.clone(),
                                outcome: GameOutcome::Stalemate,
                                board: board_state.clone(),
                            };
                            event.emit();
                            (board, Some((GameOutcome::Stalemate, board_state)))
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
                let board_state = Self::_get_board_state(&game.board.apply_eval_move(mv));
                let event = ChessEvent::FinishGame {
                    game_id: game.game_id.clone(),
                    outcome: GameOutcome::Victory(color),
                    board: board_state.clone(),
                };
                event.emit();
                Some((GameOutcome::Victory(color), board_state))
            }
            GameResult::Stalemate => {
                let board_state = Self::_get_board_state(&game.board.apply_eval_move(mv));
                let event = ChessEvent::FinishGame {
                    game_id: game.game_id.clone(),
                    outcome: GameOutcome::Stalemate,
                    board: board_state.clone(),
                };
                event.emit();
                Some((GameOutcome::Stalemate, board_state))
            }
            GameResult::IllegalMove(_) => return Err(ContractError::IllegalMove),
        };
        game.last_move_block_height = env::block_height();
        Ok((outcome_with_board, game.board.get_turn_color()))
    }

    pub fn get_board_state(&self) -> [String; 8] {
        let Game::V4(game) = self else {
            panic!("migration required")
        };
        Self::_get_board_state(&game.board)
    }

    pub fn _get_board_state(board: &Board) -> [String; 8] {
        (0..8)
            .map(|row| -> String {
                (0..8)
                    .map(move |col| -> char {
                        if let Some(piece) = board.get_piece(Position::new(row, col)) {
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
        let Game::V4(game) = self else {
            panic!("migration required")
        };
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
