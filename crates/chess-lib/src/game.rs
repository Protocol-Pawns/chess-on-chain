use crate::{
    Account, Chess, ChessEvent, ContractError, Wager, AI_EASY_GAS, AI_HARD_GAS, AI_MEDIUM_GAS,
    AI_VERY_HARD_GAS,
};
use chess_engine::{
    get_endgame_move, static_book::lookup_opening, Board, Color, GameResult, Move, Piece, Position,
    FLAG_CHECK_EXTENSIONS, FLAG_ENDGAME_HEURISTICS, FLAG_ITERATIVE_DEEPENING,
    FLAG_KILLER_HEURISTIC, FLAG_LATE_MOVE_REDUCTION, FLAG_MOVE_ORDERING, FLAG_NULL_MOVE_PRUNING,
    FLAG_OPENING_BOOK, FLAG_QUIESCENCE,
};
use near_sdk::{
    borsh::{BorshDeserialize, BorshSerialize},
    env,
    serde::{Deserialize, Serialize},
    AccountId, NearSchema,
};

#[cfg(not(feature = "integration-test"))]
const AI_MAX_DEPTHS_EASY: &[u8] = &[12, 8];
#[cfg(feature = "integration-test")]
const AI_MAX_DEPTHS_EASY: &[u8] = &[1];
const AI_MAX_DEPTHS_MEDIUM: &[u8] = &[14, 14];
const AI_MAX_DEPTHS_HARD: &[u8] = &[10, 12, 8];
const AI_MAX_DEPTHS_VERY_HARD: &[u8] = &[9, 10, 8, 6];
const AI_PIECE_COUNT_CLAMP_MIN: f64 = 4.0;
const AI_PIECE_COUNT_CLAMP_MAX: f64 = 32.0;
const AI_PIECE_SCALE_DIVISOR: f64 = 16.0;

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
    NearSchema,
)]
#[serde(crate = "near_sdk::serde")]
#[borsh(crate = "near_sdk::borsh")]
pub struct GameId(pub u64, pub AccountId, pub Option<AccountId>);

#[derive(
    BorshDeserialize,
    BorshSerialize,
    Clone,
    Debug,
    PartialEq,
    Eq,
    Serialize,
    Deserialize,
    NearSchema,
)]
#[serde(crate = "near_sdk::serde", tag = "type", content = "value")]
#[borsh(crate = "near_sdk::borsh")]
pub enum Player {
    Human(AccountId),
    Ai(Difficulty),
}

#[derive(BorshDeserialize, BorshSerialize, Serialize, Deserialize)]
#[serde(crate = "near_sdk::serde")]
#[borsh(crate = "near_sdk::borsh")]
pub enum PlayerOld {
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
/// - Easy: ~5TGas
/// - Medium: ~35TGas
/// - Hard: ~70TGas
/// - VeryHard: ~145TGas
///
/// Each difficulty has a gas cap (enforced as a soft cutoff during search):
/// - Easy: 15 TGas
/// - Medium: 40 TGas
/// - Hard: 75 TGas
/// - VeryHard: 150 TGas
#[derive(
    BorshDeserialize,
    BorshSerialize,
    Clone,
    Debug,
    Deserialize,
    Serialize,
    NearSchema,
    PartialEq,
    Eq,
)]
#[serde(crate = "near_sdk::serde")]
#[borsh(crate = "near_sdk::borsh")]
pub enum Difficulty {
    Easy,
    Medium,
    Hard,
    VeryHard,
}

impl Difficulty {
    pub fn to_flags(&self) -> u16 {
        #[cfg(feature = "integration-test")]
        if matches!(self, Self::Easy) {
            return 0;
        }
        match self {
            // Easy:       check extensions + move ordering (MVV-LVA)
            // Medium:     + opening book + null-move pruning + quiescence search
            // Hard:       + endgame heuristics + iterative deepening
            // Very Hard:  + killer heuristic + late-move reduction
            Self::Easy => FLAG_CHECK_EXTENSIONS | FLAG_MOVE_ORDERING,
            Self::Medium => {
                FLAG_CHECK_EXTENSIONS
                    | FLAG_NULL_MOVE_PRUNING
                    | FLAG_MOVE_ORDERING
                    | FLAG_QUIESCENCE
                    | FLAG_OPENING_BOOK
            }
            Self::Hard => {
                FLAG_CHECK_EXTENSIONS
                    | FLAG_NULL_MOVE_PRUNING
                    | FLAG_MOVE_ORDERING
                    | FLAG_QUIESCENCE
                    | FLAG_ITERATIVE_DEEPENING
                    | FLAG_OPENING_BOOK
                    | FLAG_ENDGAME_HEURISTICS
            }
            Self::VeryHard => {
                FLAG_CHECK_EXTENSIONS
                    | FLAG_NULL_MOVE_PRUNING
                    | FLAG_MOVE_ORDERING
                    | FLAG_QUIESCENCE
                    | FLAG_ITERATIVE_DEEPENING
                    | FLAG_KILLER_HEURISTIC
                    | FLAG_LATE_MOVE_REDUCTION
                    | FLAG_OPENING_BOOK
                    | FLAG_ENDGAME_HEURISTICS
            }
        }
    }
}

#[derive(BorshDeserialize, BorshSerialize)]
#[borsh(crate = "near_sdk::borsh")]
pub enum Game {
    V1(()),
    V2(()),
    V3(()),
    V4(GameV4),
    V5(GameV5),
}

macro_rules! access_v4_v5 {
    ($self:expr, $var:ident, $body:expr) => {
        match $self {
            Game::V4($var) => $body,
            Game::V5($var) => $body,
            _ => panic!("migration required"),
        }
    };
}

#[derive(BorshDeserialize, BorshSerialize)]
#[borsh(crate = "near_sdk::borsh")]
pub struct GameV5 {
    game_id: GameId,
    white: Player,
    black: Player,
    board: Board,
    wager: Wager,
    last_move_block_height: u64,
    has_bets: bool,
    move_count: u32,
}

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

#[derive(Deserialize, Serialize, NearSchema)]
#[serde(crate = "near_sdk::serde")]
pub struct GameInfo {
    pub white: Player,
    pub black: Player,
    pub turn_color: Color,
    pub last_block_height: u64,
    pub has_bets: bool,
}

#[derive(
    BorshDeserialize,
    BorshSerialize,
    Clone,
    Debug,
    Deserialize,
    Serialize,
    PartialEq,
    Eq,
    NearSchema,
)]
#[serde(crate = "near_sdk::serde", tag = "result", content = "color")]
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
        Game::V5(GameV5 {
            game_id,
            white,
            black,
            board: Board::default(),
            wager,
            last_move_block_height: env::block_height(),
            has_bets,
            move_count: 0,
        })
    }

    pub fn migrate(self) -> Self {
        match self {
            Self::V4(GameV4 {
                game_id,
                white,
                black,
                board,
                wager,
                last_move_block_height,
                has_bets,
            }) => Self::V5(GameV5 {
                game_id,
                white,
                black,
                board,
                wager,
                last_move_block_height,
                has_bets,
                move_count: 0,
            }),
            other => other,
        }
    }

    pub fn get_game_id(&self) -> &GameId {
        access_v4_v5!(self, game, &game.game_id)
    }

    pub fn get_white(&self) -> &Player {
        access_v4_v5!(self, game, &game.white)
    }

    pub fn get_black(&self) -> &Player {
        access_v4_v5!(self, game, &game.black)
    }

    pub fn get_board(&self) -> &Board {
        access_v4_v5!(self, game, &game.board)
    }

    pub fn get_wager(&self) -> &Wager {
        access_v4_v5!(self, game, &game.wager)
    }

    pub fn get_last_block_height(&self) -> u64 {
        access_v4_v5!(self, game, game.last_move_block_height)
    }

    pub fn is_turn(&self, account_id: &AccountId) -> bool {
        access_v4_v5!(self, game, {
            let player = match game.board.get_turn_color() {
                Color::White => &game.white,
                Color::Black => &game.black,
            };
            if let Player::Human(id) = player {
                id == account_id
            } else {
                false
            }
        })
    }

    pub fn is_player(&self, account_id: &AccountId) -> bool {
        access_v4_v5!(self, game, {
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
        })
    }

    pub fn has_bets(&self) -> bool {
        access_v4_v5!(self, game, game.has_bets)
    }

    pub fn get_move_count(&self) -> u32 {
        match self {
            Game::V5(game) => game.move_count,
            _ => 0,
        }
    }

    #[allow(clippy::type_complexity)]
    pub fn play_move(
        &mut self,
        mv: Move,
    ) -> Result<(Option<(GameOutcome, [String; 8])>, Color), ContractError> {
        let Game::V5(game) = self else {
            panic!("migration required")
        };

        let turn_color = game.board.get_turn_color();
        let (outcome, board_state, board) = match game.board.play_move(mv) {
            GameResult::Continuing(board) => {
                game.board = board;
                (None, Self::_get_board_state(&board), Some(board))
            }
            GameResult::Victory(color) => {
                let board_state = Self::_get_board_state(&game.board.apply_eval_move(mv));
                (Some(GameOutcome::Victory(color)), board_state, None)
            }
            GameResult::Stalemate => {
                let board_state = Self::_get_board_state(&game.board.apply_eval_move(mv));
                (Some(GameOutcome::Stalemate), board_state, None)
            }
            GameResult::IllegalMove(_) => return Err(ContractError::IllegalMove),
        };
        let event = ChessEvent::PlayMove {
            game_id: game.game_id.clone(),
            color: turn_color,
            mv: mv.to_string(),
            board: board_state.clone(),
            outcome: outcome.clone(),
        };
        event.emit();

        let mut outcome_with_board = outcome.map(|outcome| (outcome, board_state));
        if let (Player::Ai(difficulty), Some(board)) = (&game.black, board) {
            let max_depths: &[u8] = match difficulty {
                Difficulty::Easy => AI_MAX_DEPTHS_EASY,
                Difficulty::Medium => AI_MAX_DEPTHS_MEDIUM,
                Difficulty::Hard => AI_MAX_DEPTHS_HARD,
                Difficulty::VeryHard => AI_MAX_DEPTHS_VERY_HARD,
            };

            let piece_count = (board.count_pieces() as f64)
                .clamp(AI_PIECE_COUNT_CLAMP_MIN, AI_PIECE_COUNT_CLAMP_MAX);
            let scale = (AI_PIECE_SCALE_DIVISOR / piece_count).max(1.0);
            let depths: Vec<u8> = max_depths
                .iter()
                .map(|d| (*d as f64 * scale).round().max(1.0) as u8)
                .collect();

            let gas_budget = match difficulty {
                Difficulty::Easy => AI_EASY_GAS,
                Difficulty::Medium => AI_MEDIUM_GAS,
                Difficulty::Hard => AI_HARD_GAS,
                Difficulty::VeryHard => AI_VERY_HARD_GAS,
            };

            let flags = difficulty.to_flags();

            let seed = env::random_seed_array();

            let book_move = if (flags & FLAG_OPENING_BOOK) != 0 {
                lookup_opening(board.zobrist_key(), seed[0])
            } else {
                None
            };

            let endgame_move = if (flags & FLAG_ENDGAME_HEURISTICS) != 0 {
                get_endgame_move(&board)
            } else {
                None
            };

            let turn_color = game.board.get_turn_color();
            let (ai_mv, _, _) = if let Some(mv) = endgame_move {
                (mv, 0, 0.0)
            } else if let Some(mv) = book_move {
                if board.move_blunders_material(mv) {
                    board.get_next_move(&depths, seed, gas_budget, flags)
                } else {
                    (mv, 0, 0.0)
                }
            } else if flags == 0 {
                let mv = board.get_legal_moves().next().unwrap_or(Move::Resign);
                (mv, 0, 0.0)
            } else {
                board.get_next_move(&depths, seed, gas_budget, flags)
            };
            let (outcome, board_state) = match board.play_move(ai_mv) {
                GameResult::Continuing(board) => {
                    game.board = board;
                    (None, Self::_get_board_state(&board))
                }
                GameResult::Victory(color) => {
                    let board_state = Self::_get_board_state(&board.apply_eval_move(ai_mv));
                    (Some(GameOutcome::Victory(color)), board_state)
                }
                GameResult::Stalemate => {
                    let board_state = Self::_get_board_state(&board.apply_eval_move(ai_mv));
                    (Some(GameOutcome::Stalemate), board_state)
                }
                GameResult::IllegalMove(_) => return Err(ContractError::IllegalMove),
            };
            let event = ChessEvent::PlayMove {
                game_id: game.game_id.clone(),
                color: turn_color,
                mv: ai_mv.to_string(),
                board: board_state.clone(),
                outcome: outcome.clone(),
            };
            event.emit();

            outcome_with_board = outcome.map(|outcome| (outcome, board_state));
        }

        game.move_count += 1;
        game.last_move_block_height = env::block_height();
        Ok((outcome_with_board, game.board.get_turn_color()))
    }

    pub fn get_board_state(&self) -> [String; 8] {
        access_v4_v5!(self, game, Self::_get_board_state(&game.board))
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
        access_v4_v5!(self, game, {
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
        })
    }
}
