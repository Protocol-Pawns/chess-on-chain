use super::transposition_table::*;
use super::zobrist_keys::*;
use super::*;
use either::Either;
use near_sdk::{
    borsh::{BorshDeserialize, BorshSerialize},
    env, Gas,
};
use rand::{seq::IndexedRandom, SeedableRng};
use rand_chacha::ChaCha20Rng;
use std::cmp::Ordering;

pub const FLAG_CHECK_EXTENSIONS: u16 = 0b0000_0001;
pub const FLAG_NULL_MOVE_PRUNING: u16 = 0b0000_0010;
pub const FLAG_MOVE_ORDERING: u16 = 0b0000_0100;
pub const FLAG_QUIESCENCE: u16 = 0b0000_1000;
pub const FLAG_KILLER_HEURISTIC: u16 = 0b0001_0000;
pub const FLAG_LATE_MOVE_REDUCTION: u16 = 0b0010_0000;
pub const FLAG_ITERATIVE_DEEPENING: u16 = 0b1000_0000;
pub const FLAG_OPENING_BOOK: u16 = 0b0000_0001_0000_0000;
pub const FLAG_ENDGAME_HEURISTICS: u16 = 0b0000_0010_0000_0000;

const MAX_PLY: usize = 64;

/// Mate score. Chosen well above any reachable material/positional eval (both
/// kings are always on the board in legal play, so their ~999990 weighted
/// values cancel out and `value_for` stays in the low thousands). Mate scores
/// are encoded as `MATE - ply` (sooner mates score higher), and draws as 0.
const MATE: f64 = 1_000_000.0;
/// Sentinel "worst/best possible, no move considered yet" values. These sit
/// beyond any real score (including mate) so they never falsely match.
const NEG_INFINITY: f64 = -1_000_000_000.0;
const POS_INFINITY: f64 = 1_000_000_000.0;

pub struct BoardBuilder {
    board: Board,
}

impl From<Board> for BoardBuilder {
    fn from(board: Board) -> Self {
        Self { board }
    }
}

impl Default for BoardBuilder {
    fn default() -> Self {
        let mut board = Board::empty();
        board.white_castling_rights.disable_all();
        board.black_castling_rights.disable_all();
        Self { board }
    }
}

impl BoardBuilder {
    pub fn row(mut self, piece: Piece) -> Self {
        let mut pos = piece.get_pos();
        while pos.get_col() > 0 {
            pos = pos.next_left()
        }

        for _ in 0..8 {
            *self.board.get_square(pos) = Square::from(piece.move_to(pos));
            pos = pos.next_right();
        }

        self
    }

    pub fn column(mut self, piece: Piece) -> Self {
        let mut pos = piece.get_pos();
        while pos.get_row() > 0 {
            pos = pos.next_below()
        }

        for _ in 0..8 {
            *self.board.get_square(pos) = Square::from(piece.move_to(pos));
            pos = pos.next_above();
        }

        self
    }

    pub fn piece(mut self, piece: Piece) -> Self {
        let pos = piece.get_pos();
        *self.board.get_square(pos) = Square::from(piece);
        self
    }

    pub fn enable_castling(mut self) -> Self {
        self.board.black_castling_rights.enable_all();
        self.board.white_castling_rights.enable_all();
        self
    }

    pub fn disable_castling(mut self) -> Self {
        self.board.black_castling_rights.disable_all();
        self.board.white_castling_rights.disable_all();
        self
    }

    pub fn enable_queenside_castle(mut self, color: Color) -> Self {
        match color {
            WHITE => self.board.white_castling_rights.enable_queenside(),
            BLACK => self.board.black_castling_rights.enable_queenside(),
        }
        self
    }

    pub fn disable_queenside_castle(mut self, color: Color) -> Self {
        match color {
            WHITE => self.board.white_castling_rights.disable_queenside(),
            BLACK => self.board.black_castling_rights.disable_queenside(),
        }
        self
    }

    pub fn enable_kingside_castle(mut self, color: Color) -> Self {
        match color {
            WHITE => self.board.white_castling_rights.enable_kingside(),
            BLACK => self.board.black_castling_rights.enable_kingside(),
        }
        self
    }

    pub fn disable_kingside_castle(mut self, color: Color) -> Self {
        match color {
            WHITE => self.board.white_castling_rights.disable_kingside(),
            BLACK => self.board.black_castling_rights.disable_kingside(),
        }
        self
    }

    pub fn set_en_passant(mut self, position: Option<Position>) -> Self {
        self.board.en_passant = position;
        self
    }

    pub fn set_turn(mut self, color: Color) -> Self {
        self.board = self.board.set_turn(color);
        self
    }

    pub fn build(self) -> Board {
        self.board
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, BorshDeserialize, BorshSerialize)]
#[borsh(crate = "near_sdk::borsh")]
pub struct CastlingRights {
    kingside: bool,
    queenside: bool,
}

impl Default for CastlingRights {
    fn default() -> Self {
        Self {
            kingside: true,
            queenside: true,
        }
    }
}

impl CastlingRights {
    pub fn can_kingside_castle(&self) -> bool {
        self.kingside
    }

    pub fn can_queenside_castle(&self) -> bool {
        self.queenside
    }

    fn disable_kingside(&mut self) {
        self.kingside = false
    }

    fn disable_queenside(&mut self) {
        self.queenside = false
    }

    fn disable_all(&mut self) {
        self.disable_kingside();
        self.disable_queenside()
    }

    fn enable_kingside(&mut self) {
        self.kingside = true
    }

    fn enable_queenside(&mut self) {
        self.queenside = true
    }

    fn enable_all(&mut self) {
        self.enable_kingside();
        self.enable_queenside()
    }
}

impl Default for Board {
    fn default() -> Self {
        BoardBuilder::default()
            .piece(Piece::Rook(BLACK, A8))
            .piece(Piece::Knight(BLACK, B8))
            .piece(Piece::Bishop(BLACK, C8))
            .piece(Piece::Queen(BLACK, D8))
            .piece(Piece::King(BLACK, E8))
            .piece(Piece::Bishop(BLACK, F8))
            .piece(Piece::Knight(BLACK, G8))
            .piece(Piece::Rook(BLACK, H8))
            .row(Piece::Pawn(BLACK, A7))
            .row(Piece::Pawn(WHITE, A2))
            .piece(Piece::Rook(WHITE, A1))
            .piece(Piece::Knight(WHITE, B1))
            .piece(Piece::Bishop(WHITE, C1))
            .piece(Piece::Queen(WHITE, D1))
            .piece(Piece::King(WHITE, E1))
            .piece(Piece::Bishop(WHITE, F1))
            .piece(Piece::Knight(WHITE, G1))
            .piece(Piece::Rook(WHITE, H1))
            .enable_castling()
            .build()
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, BorshDeserialize, BorshSerialize)]
#[borsh(crate = "near_sdk::borsh")]
pub struct Board {
    squares: [Square; 64],

    en_passant: Option<Position>,

    white_castling_rights: CastlingRights,
    black_castling_rights: CastlingRights,

    turn: Color,
}

impl Board {
    pub fn value_for(&self, ally_color: Color) -> f64 {
        // Build two cheap "attacked by a pawn" maps. A piece sitting on a
        // square attacked by an enemy pawn is tactically fragile — most
        // famously, a knight grabbed by a pawn (the exact blunder we want the
        // eval to dislike even when the search is too shallow to see the
        // recapture). This is O(64) and keeps `value_for` cheap enough for
        // every leaf / quiescence stand-pat.
        let mut wp_atk = [false; 64];
        let mut bp_atk = [false; 64];
        for square in &self.squares {
            if let Some(Piece::Pawn(c, pos)) = square.get_piece() {
                let row = pos.get_row();
                let col = pos.get_col();
                // White pawns advance toward rank 8 (increasing row), so they
                // attack row + 1. Black pawns advance toward rank 1, attacking
                // row - 1.
                let target_row = if c == WHITE { row + 1 } else { row - 1 };
                if (0..=7).contains(&target_row) {
                    for target_col in [col - 1, col + 1] {
                        if (0..=7).contains(&target_col) {
                            let idx = ((7 - target_row) * 8 + target_col) as usize;
                            if c == WHITE {
                                wp_atk[idx] = true;
                            } else {
                                bp_atk[idx] = true;
                            }
                        }
                    }
                }
            }
        }

        self.squares
            .iter()
            .enumerate()
            .map(|(i, square)| match square.get_piece() {
                Some(piece) => {
                    let sign = if piece.get_color() == ally_color {
                        1.0
                    } else {
                        -1.0
                    };
                    let mut v = sign * piece.get_weighted_value();
                    // Soft penalty for knights/bishops/rooks/queens (not pawns,
                    // not kings) on a square attacked by an enemy pawn. Defended
                    // or not, the search + quiescence resolves the real tactics;
                    // this term just biases equal-looking lines away from
                    // leaving such pieces en-prise.
                    let mat = piece.get_material_value();
                    if (3..=9).contains(&mat) {
                        let enemy_pawn_atk = if piece.get_color() == WHITE {
                            bp_atk[i]
                        } else {
                            wp_atk[i]
                        };
                        if enemy_pawn_atk {
                            v -= sign * (mat as f64) * 10.0 * 0.25;
                        }
                    }
                    v
                }
                None => 0.0,
            })
            .sum()
    }

    /// Compute a Zobrist hash for the current position.
    /// This is computed on demand to avoid changing the `Board` layout (which is
    /// stored in contract state).
    pub fn zobrist_key(&self) -> u64 {
        let mut key: u64 = 0;
        for (sq, square) in self.squares.iter().enumerate() {
            if let Some(piece) = square.get_piece() {
                let (pt, color) = piece.zobrist_indices();
                key ^= PIECE_ZOBRIST_KEYS[pt][color][sq];
            }
        }
        if self.turn == Color::Black {
            key ^= BLACK_TO_MOVE_ZOBRIST_KEY;
        }
        if self.white_castling_rights.can_kingside_castle() {
            key ^= CASTLING_ZOBRIST_KEYS[0];
        }
        if self.white_castling_rights.can_queenside_castle() {
            key ^= CASTLING_ZOBRIST_KEYS[1];
        }
        if self.black_castling_rights.can_kingside_castle() {
            key ^= CASTLING_ZOBRIST_KEYS[2];
        }
        if self.black_castling_rights.can_queenside_castle() {
            key ^= CASTLING_ZOBRIST_KEYS[3];
        }
        if let Some(ep) = self.en_passant {
            key ^= EN_PASSANT_FILE_ZOBRIST_KEYS[ep.get_col() as usize];
        }
        key
    }

    #[inline]
    pub fn get_current_player_color(&self) -> Color {
        self.turn
    }

    #[inline]
    pub fn apply_eval_move(&self, m: Move) -> Self {
        self.apply_move(m).change_turn()
    }

    pub fn get_legal_moves(&self) -> impl Iterator<Item = Move> + '_ {
        let color = self.get_current_player_color();
        self.squares
            .iter()
            .filter_map(move |square| {
                if let Some(piece) = square.get_piece() {
                    if piece.get_color() == color {
                        Some(piece.get_legal_moves(self))
                    } else {
                        None
                    }
                } else {
                    None
                }
            })
            .flatten()
    }

    /// Get the best move for the current player with `depth` number of moves
    /// of lookahead.
    ///
    /// This method returns
    /// 1. The best move
    /// 2. The number of boards evaluated to come to a conclusion
    /// 3. The rating of the best move
    ///
    /// It's best not to use the rating value by itself for anything, as it
    /// is relative to the other player's move ratings as well.
    pub fn get_best_next_move(&self, depth: u8) -> (Move, u64, f64) {
        let legal_moves = self.get_legal_moves();
        let mut best_move_value = NEG_INFINITY;
        let mut best_move = Move::Resign;

        let color = self.get_current_player_color();

        let mut board_count = 0;
        let mut killers = [[None; 2]; MAX_PLY];
        let mut tt = TranspositionTable::new(0);
        for m in legal_moves {
            let child_board_value = self.apply_eval_move(m).minimax(
                &mut tt,
                Either::Left(depth),
                NEG_INFINITY,
                POS_INFINITY,
                false,
                color,
                &mut board_count,
                0,
                1,
                &mut killers,
            );
            if child_board_value > best_move_value {
                best_move = m;
                best_move_value = child_board_value;
            }
        }

        (best_move, board_count, best_move_value)
    }

    pub fn get_next_move(
        &self,
        depths: &[u8],
        seed: [u8; 32],
        gas_budget: Gas,
        flags: u16,
    ) -> (Move, u64, f64) {
        let rng = ChaCha20Rng::from_seed(seed);
        let mut legal_moves: Vec<Move> = self.get_legal_moves().collect();
        let mut tt = TranspositionTable::new(8192);
        if legal_moves.is_empty() {
            return (Move::Resign, 0, 0.0);
        }
        if (flags & FLAG_MOVE_ORDERING) != 0 {
            self.order_moves(&mut legal_moves, 0, flags, &[[None; 2]; MAX_PLY]);
        }
        let mut best_move = legal_moves[0];
        let mut best_move_value = NEG_INFINITY;

        let color = self.get_current_player_color();

        let mut board_count = 0;

        if (flags & FLAG_ITERATIVE_DEEPENING) != 0 {
            let max_depth = depths.len().saturating_sub(1).max(1);
            let mut last_best: Option<Move> = None;
            for iter in 1..=max_depth {
                if env::used_gas() >= gas_budget {
                    break;
                }
                // Search the previous iteration's best move first.
                if let Some(prev) = last_best {
                    if let Some(pos) = legal_moves.iter().position(|&m| m == prev) {
                        legal_moves.swap(0, pos);
                    }
                }
                let mut iter_best = best_move;
                let mut iter_best_value = NEG_INFINITY;
                let mut killers = [[None; 2]; MAX_PLY];
                let iter_depths = &depths[..=iter];
                for m in &legal_moves {
                    if env::used_gas() >= gas_budget {
                        break;
                    }
                    let child_board_value = self.apply_eval_move(*m).minimax(
                        &mut tt,
                        Either::Right((&iter_depths[1..], rng.clone())),
                        NEG_INFINITY,
                        POS_INFINITY,
                        false,
                        color,
                        &mut board_count,
                        flags,
                        1,
                        &mut killers,
                    );
                    if child_board_value > iter_best_value {
                        iter_best = *m;
                        iter_best_value = child_board_value;
                    }
                }
                // Only commit this iteration's result if it completed without
                // hitting the gas budget.
                if env::used_gas() < gas_budget {
                    best_move = iter_best;
                    best_move_value = iter_best_value;
                    last_best = Some(iter_best);
                }
            }
        } else {
            let mut killers = [[None; 2]; MAX_PLY];
            for m in legal_moves {
                if env::used_gas() >= gas_budget {
                    break;
                }
                let child_board_value = self.apply_eval_move(m).minimax(
                    &mut tt,
                    Either::Right((&depths[1..], rng.clone())),
                    NEG_INFINITY,
                    POS_INFINITY,
                    false,
                    color,
                    &mut board_count,
                    flags,
                    1,
                    &mut killers,
                );
                if child_board_value > best_move_value {
                    best_move = m;
                    best_move_value = child_board_value;
                }
            }
        }

        (best_move, board_count, best_move_value)
    }

    /// Get the best move for the current player with `depth` number of moves
    /// of lookahead.
    ///
    /// This method returns
    /// 1. The best move
    /// 2. The number of boards evaluated to come to a conclusion
    /// 3. The rating of the best move
    ///
    /// It's best not to use the rating value by itself for anything, as it
    /// is relative to the other player's move ratings as well.
    pub fn get_worst_next_move(&self, depth: u8) -> (Move, u64, f64) {
        let legal_moves = self.get_legal_moves();
        let mut best_move_value = NEG_INFINITY;
        let mut best_move = Move::Resign;

        let color = self.get_current_player_color();

        let mut board_count = 0;
        let mut killers = [[None; 2]; MAX_PLY];
        let mut tt = TranspositionTable::new(0);
        for m in legal_moves {
            let child_board_value = self.apply_eval_move(m).minimax(
                &mut tt,
                Either::Left(depth),
                NEG_INFINITY,
                POS_INFINITY,
                true,
                !color,
                &mut board_count,
                0,
                1,
                &mut killers,
            );

            if child_board_value >= best_move_value {
                best_move = m;
                best_move_value = child_board_value;
            }
        }

        (best_move, board_count, best_move_value)
    }

    /// Score a move for ordering (higher = search first).
    /// Captures/promotions first by MVV-LVA, then killer moves, then others.
    fn score_move_for_ordering(
        &self,
        m: Move,
        ply: u8,
        flags: u16,
        killers: &[[Option<Move>; 2]; MAX_PLY],
    ) -> i32 {
        match m {
            Move::Piece(from, to) => {
                if let Some(victim) = self.get_piece(to) {
                    let attacker = self.get_piece(from).unwrap();
                    victim.get_material_value() * 1000 - attacker.get_material_value()
                } else if (flags & FLAG_KILLER_HEURISTIC) != 0 && (ply as usize) < MAX_PLY {
                    if killers[ply as usize][0] == Some(m) {
                        500
                    } else if killers[ply as usize][1] == Some(m) {
                        400
                    } else {
                        -1
                    }
                } else {
                    -1
                }
            }
            Move::Promotion(_, to, piece) => {
                let base = piece.get_material_value() * 1000;
                if self.has_enemy_piece(to, self.turn) {
                    base + 100
                } else {
                    base
                }
            }
            Move::KingSideCastle | Move::QueenSideCastle => 50,
            _ => 0,
        }
    }

    /// Sort moves in-place so alpha-beta cuts early.
    fn order_moves(
        &self,
        moves: &mut [Move],
        ply: u8,
        flags: u16,
        killers: &[[Option<Move>; 2]; MAX_PLY],
    ) {
        moves.sort_by(|a, b| {
            let sa = self.score_move_for_ordering(*a, ply, flags, killers);
            let sb = self.score_move_for_ordering(*b, ply, flags, killers);
            sb.cmp(&sa)
        });
    }

    /// Sample up to `max_moves` moves from an already-ordered move list, always
    /// keeping captures and promotions. The shallow, randomly-sampled search
    /// used on-chain otherwise routinely drops the single capturing refutation
    /// (e.g. a pawn recapturing a hung knight), which is how the AI blunders
    /// pieces in the opening.
    fn sample_moves(&self, moves: &[Move], rng: &mut ChaCha20Rng, max_moves: usize) -> Vec<Move> {
        if moves.len() <= max_moves {
            return moves.to_vec();
        }
        let mut kept: Vec<Move> = Vec::with_capacity(max_moves);
        let mut quiet: Vec<Move> = Vec::new();
        for &m in moves {
            if self.is_capture(m) || matches!(m, Move::Promotion(..)) {
                if kept.len() < max_moves {
                    kept.push(m);
                }
            } else {
                quiet.push(m);
            }
        }
        let need = max_moves.saturating_sub(kept.len());
        if need > 0 {
            let mut chosen: Vec<Move> = quiet.sample(rng, need).copied().collect();
            kept.append(&mut chosen);
        }
        kept
    }

    /// Is `pos` attacked by a pawn of `attacker_color`? O(2) — used by the
    /// hanging-piece book guard below.
    pub fn square_attacked_by_pawn(&self, pos: Position, attacker_color: Color) -> bool {
        let row = pos.get_row();
        let col = pos.get_col();
        // A white pawn attacks the square one rank above it (toward rank 8),
        // so it sits one rank below the square it attacks (row - 1). Black is
        // mirrored (row + 1).
        let from_row = if attacker_color == WHITE {
            row - 1
        } else {
            row + 1
        };
        if !(0..=7).contains(&from_row) {
            return false;
        }
        for from_col in [col - 1, col + 1] {
            if !(0..=7).contains(&from_col) {
                continue;
            }
            if let Some(Piece::Pawn(c, _)) = self.get_piece(Position::new(from_row, from_col)) {
                if c == attacker_color {
                    return true;
                }
            }
        }
        false
    }

    /// Would playing `m` leave a knight/bishop/rook/queen hanging to an enemy
    /// pawn (capturable for free next ply)? Used to reject blundering opening
    /// book replies in favor of a real search.
    pub fn move_hangs_piece_to_pawn(&self, m: Move) -> bool {
        let (from, to) = match m {
            Move::Piece(f, t) | Move::Promotion(f, t, _) => (f, t),
            _ => return false,
        };
        let piece = match self.get_piece(from) {
            Some(p) => p,
            None => return false,
        };
        let color = piece.get_color();
        let enemy = !color;
        let nb = self.apply_eval_move(m);
        let landed = match nb.get_piece(to) {
            Some(p) => p,
            None => return false,
        };
        let mat = landed.get_material_value();
        // Only "heavy enough to be a blunder" pieces (knight and up, not king).
        if !(3..=9).contains(&mat) {
            return false;
        }
        if !nb.square_attacked_by_pawn(to, enemy) {
            return false;
        }
        // Hanging only if no friendly piece defends the landing square.
        // `is_threatened(to, enemy)` is true when a *friend* attacks `to`.
        !nb.is_threatened(to, enemy)
    }

    /// Is this move a capture (including en-passant)?
    fn is_capture(&self, m: Move) -> bool {
        match m {
            Move::Piece(from, to) => {
                if let Some(en_passant) = self.en_passant {
                    if let Some(Piece::Pawn(color, _)) = self.get_piece(from) {
                        if color == self.turn && to == en_passant {
                            return true;
                        }
                    }
                }
                self.has_enemy_piece(to, self.turn)
            }
            Move::Promotion(_, to, _) => self.has_enemy_piece(to, self.turn),
            _ => false,
        }
    }

    /// Quiet stand-pat: evaluate then search only captures.
    fn quiesce(
        &self,
        mut alpha: f64,
        mut beta: f64,
        is_maximizing: bool,
        getting_move_for: Color,
        board_count: &mut u64,
        depth: u8,
        ply: u8,
        flags: u16,
        killers: &[[Option<Move>; 2]; MAX_PLY],
    ) -> f64 {
        *board_count += 1;

        let stand_pat = self.value_for(getting_move_for);

        if is_maximizing {
            if stand_pat >= beta {
                return beta;
            }
            if stand_pat > alpha {
                alpha = stand_pat;
            }
        } else {
            if stand_pat <= alpha {
                return alpha;
            }
            if stand_pat < beta {
                beta = stand_pat;
            }
        }

        if depth >= 4 {
            return stand_pat;
        }

        let mut captures: Vec<Move> = self
            .get_legal_moves()
            .filter(|m| self.is_capture(*m))
            .collect();
        self.order_moves(&mut captures, ply, flags, killers);

        let mut best = if is_maximizing {
            NEG_INFINITY
        } else {
            POS_INFINITY
        };

        // Delta-pruning margin (weighted units). If even winning the captured
        // piece for free can't reach alpha / beat beta, skip the capture.
        const DELTA_MARGIN: f64 = 200.0;

        for m in &captures {
            let to = match m {
                Move::Piece(_, t) | Move::Promotion(_, t, _) => *t,
                _ => continue,
            };
            let victim_weighted = match self.get_piece(to) {
                Some(v) => v.get_material_value() as f64 * 10.0,
                // en-passant: the captured pawn isn't on `to`.
                None if self.is_capture(*m) => 10.0,
                None => 0.0,
            };

            // Delta pruning: bail out of captures that obviously can't matter.
            if is_maximizing {
                if stand_pat + victim_weighted + DELTA_MARGIN <= alpha {
                    continue;
                }
            } else if stand_pat - victim_weighted - DELTA_MARGIN >= beta {
                continue;
            }

            let child = self.apply_eval_move(*m);
            let val = child.quiesce(
                alpha,
                beta,
                !is_maximizing,
                getting_move_for,
                board_count,
                depth + 1,
                ply + 1,
                flags,
                killers,
            );

            if is_maximizing {
                if val > best {
                    best = val;
                }
                if best > alpha {
                    alpha = best;
                }
            } else {
                if val < best {
                    best = val;
                }
                if best < beta {
                    beta = best;
                }
            }

            if beta <= alpha {
                break;
            }
        }

        if is_maximizing {
            if best == NEG_INFINITY {
                stand_pat
            } else {
                best
            }
        } else {
            if best == POS_INFINITY {
                stand_pat
            } else {
                best
            }
        }
    }

    /// Perform minimax on a certain position with alpha-beta pruning.
    ///
    /// `flags` enables optional features per difficulty (check extensions,
    /// null-move pruning, move ordering, quiescence search, etc.).
    /// `ply` is the distance from the root (0 = root).
    /// `killers` stores quiet moves that caused beta cutoffs per ply.
    pub fn minimax(
        &self,
        tt: &mut TranspositionTable,
        depth: Either<u8, (&[u8], ChaCha20Rng)>,
        mut alpha: f64,
        mut beta: f64,
        is_maximizing: bool,
        getting_move_for: Color,
        board_count: &mut u64,
        flags: u16,
        ply: u8,
        killers: &mut [[Option<Move>; 2]; MAX_PLY],
    ) -> f64 {
        *board_count += 1;

        let zobrist = self.zobrist_key();
        let tt_key = tt_context_key(zobrist, &depth);
        let tt_depth: u8 = match &depth {
            Either::Left(d) => *d,
            Either::Right((d, _)) => d.len() as u8,
        };
        let original_alpha = alpha;
        let original_beta = beta;
        if let Some(entry) = tt.get(tt_key) {
            if entry.depth >= tt_depth {
                match entry.flag {
                    TtFlag::Exact => return entry.value,
                    TtFlag::LowerBound if entry.value >= beta => return beta,
                    TtFlag::UpperBound if entry.value <= alpha => return alpha,
                    _ => {}
                }
            }
        }

        let (mut next_depth, max_moves) = match depth {
            Either::Left(0) => {
                if (flags & FLAG_QUIESCENCE) != 0 {
                    return self.quiesce(
                        alpha,
                        beta,
                        is_maximizing,
                        getting_move_for,
                        board_count,
                        0,
                        ply,
                        flags,
                        killers,
                    );
                }
                return self.value_for(getting_move_for);
            }
            Either::Right(([], _)) => {
                if (flags & FLAG_QUIESCENCE) != 0 {
                    return self.quiesce(
                        alpha,
                        beta,
                        is_maximizing,
                        getting_move_for,
                        board_count,
                        0,
                        ply,
                        flags,
                        killers,
                    );
                }
                return self.value_for(getting_move_for);
            }
            Either::Left(d) => {
                if (flags & FLAG_CHECK_EXTENSIONS) != 0
                    && self.is_in_check(self.get_current_player_color())
                {
                    (Either::Left(d), None)
                } else {
                    (Either::Left(d - 1), None)
                }
            }
            Either::Right((d, rng)) => {
                if (flags & FLAG_CHECK_EXTENSIONS) != 0
                    && self.is_in_check(self.get_current_player_color())
                {
                    (Either::Right((d, rng)), Some(d[0]))
                } else {
                    (Either::Right((&d[1..], rng)), Some(d[0]))
                }
            }
        };

        // Terminal detection: distinguish checkmate from stalemate. Previously
        // an empty move list just caused the loop below to return its init
        // sentinel (+/-999999), so the search scored a drawing stalemate
        // identically to a winning checkmate — and the AI happily stalemated
        // won games. Also precompute & order the move list once so we don't
        // regenerate legal moves four times below.
        let side_to_move = self.get_current_player_color();
        let mut legal_moves: Vec<Move> = self.get_legal_moves().collect();
        if legal_moves.is_empty() {
            if self.is_in_check(side_to_move) {
                // `side_to_move` is checkmated. Prefer faster mates via ply.
                if side_to_move == getting_move_for {
                    return -MATE + ply as f64;
                } else {
                    return MATE - ply as f64;
                }
            }
            // Stalemate: a draw for both sides.
            return 0.0;
        }
        if (flags & FLAG_MOVE_ORDERING) != 0 {
            self.order_moves(&mut legal_moves, ply, flags, killers);
        }

        // Null-move pruning (skip turn, see if position is still crushing)
        if (flags & FLAG_NULL_MOVE_PRUNING) != 0
            && self.count_pieces() >= 6
            && !self.is_in_check(self.get_current_player_color())
        {
            let enough_depth = match next_depth {
                Either::Left(d) => d >= 3,
                Either::Right((d, _)) => !d.is_empty() && d[0] >= 3,
            };
            if enough_depth {
                let null_board = self.change_turn();
                let null_depth: Either<u8, (&[u8], ChaCha20Rng)> = match next_depth {
                    Either::Left(d) => Either::Left(d - 2),
                    Either::Right((d, ref rng)) => {
                        if d.len() <= 1 {
                            Either::Right((d, rng.clone()))
                        } else {
                            Either::Right((&d[1..], rng.clone()))
                        }
                    }
                };
                if is_maximizing {
                    let null_score = null_board.minimax(
                        tt,
                        null_depth,
                        beta - 1.0,
                        beta,
                        false,
                        getting_move_for,
                        board_count,
                        flags,
                        ply + 1,
                        killers,
                    );
                    if null_score >= beta {
                        return beta;
                    }
                } else {
                    let null_score = null_board.minimax(
                        tt,
                        null_depth,
                        alpha,
                        alpha + 1.0,
                        true,
                        getting_move_for,
                        board_count,
                        flags,
                        ply + 1,
                        killers,
                    );
                    if null_score <= alpha {
                        return alpha;
                    }
                }
            }
        }

        let mut best_move_value;

        if is_maximizing {
            best_move_value = NEG_INFINITY;

            if let Some(max_moves) = max_moves {
                let Either::Right((_, rng)) = &mut next_depth else {
                    panic!();
                };
                let sampled = self.sample_moves(&legal_moves, rng, max_moves as usize);
                for (move_idx, m) in sampled.iter().enumerate() {
                    let m = *m;
                    let child = self.apply_eval_move(m);
                    let child_board_value = if (flags & FLAG_LATE_MOVE_REDUCTION) != 0
                        && move_idx >= 4
                        && !self.is_capture(m)
                        && !matches!(m, Move::Promotion(_, _, _))
                        && !self.is_in_check(self.get_current_player_color())
                        && !child.is_in_check(child.get_current_player_color())
                        && Self::has_enough_depth_for_lmr(&next_depth)
                    {
                        let reduced = Self::reduce_depth(next_depth.clone());
                        let reduced_value = child.minimax(
                            tt,
                            reduced,
                            alpha,
                            beta,
                            !is_maximizing,
                            getting_move_for,
                            board_count,
                            flags,
                            ply + 1,
                            killers,
                        );
                        if reduced_value > alpha {
                            child.minimax(
                                tt,
                                next_depth.clone(),
                                alpha,
                                beta,
                                !is_maximizing,
                                getting_move_for,
                                board_count,
                                flags,
                                ply + 1,
                                killers,
                            )
                        } else {
                            reduced_value
                        }
                    } else {
                        child.minimax(
                            tt,
                            next_depth.clone(),
                            alpha,
                            beta,
                            !is_maximizing,
                            getting_move_for,
                            board_count,
                            flags,
                            ply + 1,
                            killers,
                        )
                    };

                    if child_board_value > best_move_value {
                        best_move_value = child_board_value;
                    }

                    if best_move_value > alpha {
                        alpha = best_move_value
                    }

                    if beta <= alpha {
                        if (flags & FLAG_KILLER_HEURISTIC) != 0
                            && (ply as usize) < MAX_PLY
                            && !self.is_capture(m)
                        {
                            let slot = &mut killers[ply as usize];
                            if slot[0] != Some(m) {
                                slot[1] = slot[0];
                                slot[0] = Some(m);
                            }
                        }
                        return best_move_value;
                    }
                }
            } else {
                for (move_idx, m) in legal_moves.iter().enumerate() {
                    let m = *m;
                    let child = self.apply_eval_move(m);
                    let child_board_value = if (flags & FLAG_LATE_MOVE_REDUCTION) != 0
                        && move_idx >= 4
                        && !self.is_capture(m)
                        && !matches!(m, Move::Promotion(_, _, _))
                        && !self.is_in_check(self.get_current_player_color())
                        && !child.is_in_check(child.get_current_player_color())
                        && Self::has_enough_depth_for_lmr(&next_depth)
                    {
                        let reduced = Self::reduce_depth(next_depth.clone());
                        let reduced_value = child.minimax(
                            tt,
                            reduced,
                            alpha,
                            beta,
                            !is_maximizing,
                            getting_move_for,
                            board_count,
                            flags,
                            ply + 1,
                            killers,
                        );
                        if reduced_value > alpha {
                            child.minimax(
                                tt,
                                next_depth.clone(),
                                alpha,
                                beta,
                                !is_maximizing,
                                getting_move_for,
                                board_count,
                                flags,
                                ply + 1,
                                killers,
                            )
                        } else {
                            reduced_value
                        }
                    } else {
                        child.minimax(
                            tt,
                            next_depth.clone(),
                            alpha,
                            beta,
                            !is_maximizing,
                            getting_move_for,
                            board_count,
                            flags,
                            ply + 1,
                            killers,
                        )
                    };

                    if child_board_value > best_move_value {
                        best_move_value = child_board_value;
                    }

                    if best_move_value > alpha {
                        alpha = best_move_value
                    }

                    if beta <= alpha {
                        if (flags & FLAG_KILLER_HEURISTIC) != 0
                            && (ply as usize) < MAX_PLY
                            && !self.is_capture(m)
                        {
                            let slot = &mut killers[ply as usize];
                            if slot[0] != Some(m) {
                                slot[1] = slot[0];
                                slot[0] = Some(m);
                            }
                        }
                        return best_move_value;
                    }
                }
            };
        } else {
            best_move_value = POS_INFINITY;

            if let Some(max_moves) = max_moves {
                let Either::Right((_, rng)) = &mut next_depth else {
                    panic!();
                };
                let sampled = self.sample_moves(&legal_moves, rng, max_moves as usize);
                for (move_idx, m) in sampled.iter().enumerate() {
                    let m = *m;
                    let child = self.apply_eval_move(m);
                    let child_board_value = if (flags & FLAG_LATE_MOVE_REDUCTION) != 0
                        && move_idx >= 4
                        && !self.is_capture(m)
                        && !matches!(m, Move::Promotion(_, _, _))
                        && !self.is_in_check(self.get_current_player_color())
                        && !child.is_in_check(child.get_current_player_color())
                        && Self::has_enough_depth_for_lmr(&next_depth)
                    {
                        let reduced = Self::reduce_depth(next_depth.clone());
                        let reduced_value = child.minimax(
                            tt,
                            reduced,
                            alpha,
                            beta,
                            !is_maximizing,
                            getting_move_for,
                            board_count,
                            flags,
                            ply + 1,
                            killers,
                        );
                        if reduced_value < beta {
                            child.minimax(
                                tt,
                                next_depth.clone(),
                                alpha,
                                beta,
                                !is_maximizing,
                                getting_move_for,
                                board_count,
                                flags,
                                ply + 1,
                                killers,
                            )
                        } else {
                            reduced_value
                        }
                    } else {
                        child.minimax(
                            tt,
                            next_depth.clone(),
                            alpha,
                            beta,
                            !is_maximizing,
                            getting_move_for,
                            board_count,
                            flags,
                            ply + 1,
                            killers,
                        )
                    };

                    if child_board_value < best_move_value {
                        best_move_value = child_board_value;
                    }

                    if best_move_value < beta {
                        beta = best_move_value
                    }

                    if beta <= alpha {
                        if (flags & FLAG_KILLER_HEURISTIC) != 0
                            && (ply as usize) < MAX_PLY
                            && !self.is_capture(m)
                        {
                            let slot = &mut killers[ply as usize];
                            if slot[0] != Some(m) {
                                slot[1] = slot[0];
                                slot[0] = Some(m);
                            }
                        }
                        return best_move_value;
                    }
                }
            } else {
                for (move_idx, m) in legal_moves.iter().enumerate() {
                    let m = *m;
                    let child = self.apply_eval_move(m);
                    let child_board_value = if (flags & FLAG_LATE_MOVE_REDUCTION) != 0
                        && move_idx >= 4
                        && !self.is_capture(m)
                        && !matches!(m, Move::Promotion(_, _, _))
                        && !self.is_in_check(self.get_current_player_color())
                        && !child.is_in_check(child.get_current_player_color())
                        && Self::has_enough_depth_for_lmr(&next_depth)
                    {
                        let reduced = Self::reduce_depth(next_depth.clone());
                        let reduced_value = child.minimax(
                            tt,
                            reduced,
                            alpha,
                            beta,
                            !is_maximizing,
                            getting_move_for,
                            board_count,
                            flags,
                            ply + 1,
                            killers,
                        );
                        if reduced_value < beta {
                            child.minimax(
                                tt,
                                next_depth.clone(),
                                alpha,
                                beta,
                                !is_maximizing,
                                getting_move_for,
                                board_count,
                                flags,
                                ply + 1,
                                killers,
                            )
                        } else {
                            reduced_value
                        }
                    } else {
                        child.minimax(
                            tt,
                            next_depth.clone(),
                            alpha,
                            beta,
                            !is_maximizing,
                            getting_move_for,
                            board_count,
                            flags,
                            ply + 1,
                            killers,
                        )
                    };

                    if child_board_value < best_move_value {
                        best_move_value = child_board_value;
                    }

                    if best_move_value < beta {
                        beta = best_move_value
                    }

                    if beta <= alpha {
                        if (flags & FLAG_KILLER_HEURISTIC) != 0
                            && (ply as usize) < MAX_PLY
                            && !self.is_capture(m)
                        {
                            let slot = &mut killers[ply as usize];
                            if slot[0] != Some(m) {
                                slot[1] = slot[0];
                                slot[0] = Some(m);
                            }
                        }
                        return best_move_value;
                    }
                }
            }
        }

        let flag = if best_move_value <= original_alpha {
            TtFlag::UpperBound
        } else if best_move_value >= original_beta {
            TtFlag::LowerBound
        } else {
            TtFlag::Exact
        };
        tt.store(tt_key, tt_depth, flag, best_move_value, None);
        best_move_value
    }

    /// Reduce depth by one ply for LMR.
    fn reduce_depth(depth: Either<u8, (&[u8], ChaCha20Rng)>) -> Either<u8, (&[u8], ChaCha20Rng)> {
        match depth {
            Either::Left(d) => Either::Left(d.saturating_sub(1).max(1)),
            Either::Right((d, rng)) => {
                if d.len() <= 1 {
                    Either::Right((d, rng))
                } else {
                    Either::Right((&d[1..], rng))
                }
            }
        }
    }

    /// Is there enough remaining depth to safely apply LMR?
    fn has_enough_depth_for_lmr(depth: &Either<u8, (&[u8], ChaCha20Rng)>) -> bool {
        match depth {
            Either::Left(d) => *d >= 3,
            Either::Right((d, _)) => !d.is_empty() && d[0] >= 3,
        }
    }
}

impl core::fmt::Display for Board {
    fn fmt(&self, f: &mut core::fmt::Formatter) -> Result<(), core::fmt::Error> {
        let rating_bar = self.rating_bar(16);
        let abc = if self.turn == WHITE {
            "abcdefgh"
        } else {
            "hgfedcba"
        };

        write!(f, "   {}\n  ╔════════╗", abc)?;
        let mut square_color = !self.turn;
        let height = 8;
        let width = 8;

        for row in 0..height {
            writeln!(f)?;

            let print_row = match self.turn {
                WHITE => height - row - 1,
                BLACK => row,
            };
            write!(f, "{} ║", print_row + 1)?;

            for col in 0..width {
                let print_col = match self.turn {
                    BLACK => width - col - 1,
                    WHITE => col,
                };

                let pos = Position::new(print_row, print_col);

                let s = if let Some(piece) = self.get_piece(pos) {
                    piece.to_string()
                } else {
                    String::from(match square_color {
                        WHITE => "░",
                        BLACK => "▓",
                    })
                };
                if Some(pos) == self.en_passant {
                    write!(f, "\x1b[34m{}\x1b[m\x1b[0m", s)?;
                } else if self.is_threatened(pos, self.turn) {
                    write!(f, "\x1b[31m{}\x1b[m\x1b[0m", s)?;
                } else if self.is_threatened(pos, !self.turn) {
                    write!(f, "\x1b[32m{}\x1b[m\x1b[0m", s)?;
                } else {
                    write!(f, "{}", s)?;
                }

                square_color = !square_color;
            }
            write!(f, "║")?;

            if row == 2 {
                let white_adv = self.get_material_advantage(WHITE);
                let black_adv = self.get_material_advantage(BLACK);

                match white_adv.cmp(&black_adv) {
                    Ordering::Equal => write!(f, " Both sides have equal material")?,
                    Ordering::Greater => write!(f, " White +{} points", white_adv)?,
                    Ordering::Less => write!(f, " Black +{} points", black_adv)?,
                }
            } else if row == 3 {
                write!(f, " {} to move", self.turn)?;
            } else if row == 4 {
                write!(f, " [{}]", rating_bar)?;
            }
            square_color = !square_color;
        }

        write!(f, "\n  ╚════════╝\n   {}\n", abc)
    }
}

impl Board {
    /// Create the default board for the Horde variant
    pub fn horde() -> Self {
        BoardBuilder::from(Board::default())
            .row(Piece::Pawn(WHITE, A1))
            .row(Piece::Pawn(WHITE, A2))
            .row(Piece::Pawn(WHITE, A3))
            .row(Piece::Pawn(WHITE, A4))
            .piece(Piece::Pawn(WHITE, F5))
            .piece(Piece::Pawn(WHITE, G5))
            .piece(Piece::Pawn(WHITE, B5))
            .piece(Piece::Pawn(WHITE, C5))
            .build()
    }

    pub fn empty() -> Self {
        Self {
            squares: [EMPTY_SQUARE; 64],
            en_passant: None,

            white_castling_rights: CastlingRights::default(),
            black_castling_rights: CastlingRights::default(),

            turn: WHITE,
        }
    }

    pub fn rating_bar(&self, len: usize) -> String {
        let (best_m, _, your_best_val) = self.get_best_next_move(2);
        let (_, _, your_lowest_val) = self.get_worst_next_move(2);
        let mut your_val = your_best_val + your_lowest_val;
        let (_, _, their_best_val) = self.apply_move(best_m).change_turn().get_best_next_move(2);
        let (_, _, their_lowest_val) = self.apply_move(best_m).change_turn().get_worst_next_move(2);
        let mut their_val = their_best_val + their_lowest_val;

        if your_val < 0.0 {
            your_val = -your_val;
            their_val += your_val * 2.0;
        }

        if their_val < 0.0 {
            their_val = -their_val;
            your_val += their_val * 2.0;
        }

        let your_percentage = your_val / (your_val + their_val);
        let their_percentage = their_val / (your_val + their_val);

        let (your_color, their_color) = match self.turn {
            WHITE => ("▓", "░"),
            BLACK => ("░", "▓"),
        };

        let white = match self.turn {
            WHITE => your_color.repeat((your_percentage * len as f64) as usize),
            BLACK => their_color.repeat((their_percentage * len as f64) as usize),
        };

        let black = match self.turn {
            BLACK => your_color.repeat((your_percentage * len as f64) as usize),
            WHITE => their_color.repeat((their_percentage * len as f64) as usize),
        };

        white + &black
    }

    /// Get the color of the current player
    #[inline]
    pub fn get_turn_color(&self) -> Color {
        self.turn
    }

    /// Get the position of the En-Passant square
    pub fn get_en_passant(&self) -> Option<Position> {
        self.en_passant
    }

    /// Remove all of the pieces for a given player
    pub fn remove_all(&self, color: Color) -> Self {
        let mut result = *self;
        for square in &mut result.squares {
            if let Some(piece) = square.get_piece() {
                if piece.get_color() == color {
                    *square = EMPTY_SQUARE
                }
            }
        }

        result
    }

    /// Convert all of a given players pieces to queens
    pub fn queen_all(&self, color: Color) -> Self {
        let mut result = *self;
        for square in &mut result.squares {
            if let Some(piece) = square.get_piece() {
                if !piece.is_king() && piece.get_color() == color {
                    *square = Square::from(Piece::Queen(color, piece.get_pos()))
                }
            }
        }

        result
    }

    /// Make the game a certain player's turn
    #[inline]
    pub fn set_turn(&self, color: Color) -> Self {
        let mut result = *self;
        result.turn = color;
        result
    }

    /// Count all pieces on the board
    #[inline]
    pub fn count_pieces(&self) -> u32 {
        self.squares.iter().filter(|s| !s.is_empty()).count() as u32
    }

    /// Get the value of the material advantage of a certain player
    #[inline]
    pub fn get_material_advantage(&self, color: Color) -> i32 {
        self.squares
            .iter()
            .map(|square| match square.get_piece() {
                Some(piece) => {
                    if piece.get_color() == color {
                        piece.get_material_value()
                    } else {
                        -piece.get_material_value()
                    }
                }
                None => 0,
            })
            .sum()
    }

    #[inline]
    fn get_square(&mut self, pos: Position) -> &mut Square {
        &mut self.squares[((7 - pos.get_row()) * 8 + pos.get_col()) as usize]
    }

    #[inline]
    fn add_piece(&mut self, piece: Piece) {
        let pos = piece.get_pos();
        *self.get_square(pos) = Square::from(piece);
    }

    /// Does a square have any piece?
    #[inline]
    pub fn get_piece(&self, pos: Position) -> Option<Piece> {
        if pos.is_off_board() {
            return None;
        }
        self.squares[((7 - pos.get_row()) * 8 + pos.get_col()) as usize].get_piece()
    }

    /// Does a square have an ally piece?
    #[inline]
    pub fn has_ally_piece(&self, pos: Position, ally_color: Color) -> bool {
        if let Some(piece) = self.get_piece(pos) {
            piece.get_color() == ally_color
        } else {
            false
        }
    }

    /// If a square at a given position has an enemy piece from a given
    /// ally color, return true. Otherwise, return false.
    ///
    /// For example, if a square has a black piece, and this method is called
    /// upon it with an `ally_color` of `Color::White`, then it will return true.
    /// If called with `Color::Black` upon the same square, however, it will return false.
    #[inline]
    pub fn has_enemy_piece(&self, pos: Position, ally_color: Color) -> bool {
        if let Some(piece) = self.get_piece(pos) {
            piece.get_color() == !ally_color
        } else {
            false
        }
    }

    /// If a square at a given position has any piece, return true.
    /// Otherwise, return false.
    #[inline]
    pub fn has_piece(&self, pos: Position) -> bool {
        self.get_piece(pos).is_some()
    }

    /// If a square at a given position has no piece, return true.
    /// Otherwise, return false.
    #[inline]
    pub fn has_no_piece(&self, pos: Position) -> bool {
        self.get_piece(pos).is_none()
    }

    /// If there is a king on the board, return the position that it sits on.
    pub fn get_king_pos(&self, color: Color) -> Option<Position> {
        let mut king_pos = None;
        for square in &self.squares {
            if let Some(Piece::King(c, pos)) = square.get_piece() {
                if c == color {
                    king_pos = Some(pos);
                }
            }
        }
        king_pos
    }

    /// Is a square threatened by an enemy piece?
    pub fn is_threatened(&self, pos: Position, ally_color: Color) -> bool {
        for (i, square) in self.squares.iter().enumerate() {
            let row = 7 - i / 8;
            let col = i % 8;
            let square_pos = Position::new(row as i32, col as i32);
            if !square_pos.is_orthogonal_to(pos)
                && !square_pos.is_diagonal_to(pos)
                && !square_pos.is_knight_move(pos)
            {
                continue;
            }

            if let Some(piece) = square.get_piece() {
                if piece.get_color() == ally_color {
                    continue;
                }

                if piece.is_legal_attack(pos, self) {
                    return true;
                }
            }
        }

        false
    }

    /// Get whether or not the king of a given color is in check.
    #[inline]
    pub fn is_in_check(&self, color: Color) -> bool {
        if let Some(king_pos) = self.get_king_pos(color) {
            self.is_threatened(king_pos, color)
        } else {
            false
        }
    }

    fn move_piece(&self, from: Position, to: Position, promotion: Option<Piece>) -> Self {
        let mut result = *self;
        result.en_passant = None;

        if from.is_off_board() || to.is_off_board() {
            return result;
        }

        let from_square = result.get_square(from);
        if let Some(mut piece) = from_square.get_piece() {
            *from_square = EMPTY_SQUARE;

            if piece.is_pawn() && (to.get_row() == 0 || to.get_row() == 7) {
                piece = match promotion {
                    // promotion only required to specify piece type
                    Some(promotion) => {
                        if promotion.is_king() || promotion.is_pawn() {
                            // invalid promotion, use default
                            Piece::Queen(piece.get_color(), piece.get_pos())
                        } else {
                            promotion
                                .with_color(piece.get_color())
                                .move_to(piece.get_pos())
                        }
                    }
                    // queen by default
                    None => Piece::Queen(piece.get_color(), piece.get_pos()),
                }
            }

            if piece.is_starting_pawn() && (from.get_row() - to.get_row()).abs() == 2 {
                result.en_passant = Some(to.pawn_back(piece.get_color()))
            }

            result.add_piece(piece.move_to(to));

            let castling_rights = match piece.get_color() {
                WHITE => &mut result.white_castling_rights,
                BLACK => &mut result.black_castling_rights,
            };

            if piece.is_king() {
                castling_rights.disable_all();
            } else if piece.is_queenside_rook() {
                castling_rights.disable_queenside();
            } else if piece.is_kingside_rook() {
                castling_rights.disable_kingside();
            }
        }

        result
    }

    /// Can a given player castle kingside?
    pub fn can_kingside_castle(&self, color: Color) -> bool {
        let right_of_king = Position::king_pos(color).next_right();
        match color {
            WHITE => {
                self.has_no_piece(Position::new(0, 5))
                    && self.has_no_piece(Position::new(0, 6))
                    && self.get_piece(Position::new(0, 7))
                        == Some(Piece::Rook(color, Position::new(0, 7)))
                    && self.white_castling_rights.can_kingside_castle()
                    && !self.is_in_check(color)
                    && !self.is_threatened(right_of_king, color)
                    && !self.is_threatened(right_of_king.next_right(), color)
            }
            BLACK => {
                self.has_no_piece(Position::new(7, 5))
                    && self.has_no_piece(Position::new(7, 6))
                    && self.get_piece(Position::new(7, 7))
                        == Some(Piece::Rook(color, Position::new(7, 7)))
                    && self.black_castling_rights.can_kingside_castle()
                    && !self.is_in_check(color)
                    && !self.is_threatened(right_of_king, color)
                    && !self.is_threatened(right_of_king.next_right(), color)
            }
        }
    }

    /// Can a given player castle queenside?
    pub fn can_queenside_castle(&self, color: Color) -> bool {
        match color {
            WHITE => {
                self.has_no_piece(Position::new(0, 1))
                    && self.has_no_piece(Position::new(0, 2))
                    && self.has_no_piece(Position::new(0, 3))
                    && self.get_piece(Position::new(0, 0))
                        == Some(Piece::Rook(color, Position::new(0, 0)))
                    && self.white_castling_rights.can_queenside_castle()
                    && !self.is_in_check(color)
                    && !self.is_threatened(Position::queen_pos(color), color)
            }
            BLACK => {
                self.has_no_piece(Position::new(7, 1))
                    && self.has_no_piece(Position::new(7, 2))
                    && self.has_no_piece(Position::new(7, 3))
                    && self.get_piece(Position::new(7, 0))
                        == Some(Piece::Rook(color, Position::new(7, 0)))
                    && self.black_castling_rights.can_queenside_castle()
                    && !self.is_in_check(color)
                    && !self.is_threatened(Position::queen_pos(color), color)
            }
        }
    }

    pub fn get_castling_rights(&self, color: Color) -> CastlingRights {
        match color {
            WHITE => self.white_castling_rights,
            BLACK => self.black_castling_rights,
        }
    }

    pub(crate) fn is_legal_move(&self, m: Move, player_color: Color) -> bool {
        match m {
            Move::KingSideCastle => self.can_kingside_castle(player_color),
            Move::QueenSideCastle => self.can_queenside_castle(player_color),
            Move::Piece(from, to) => match self.get_piece(from) {
                Some(Piece::Pawn(c, pos)) => {
                    let piece = Piece::Pawn(c, pos);
                    ((if let Some(en_passant) = self.en_passant {
                        ((en_passant == from.pawn_up(player_color).next_left()
                            || en_passant == from.pawn_up(player_color).next_right())
                            && en_passant == to)
                            && c == player_color
                    } else {
                        false
                    }) || piece.is_legal_move(to, self) && piece.get_color() == player_color)
                        && !self.apply_move(m).is_in_check(player_color)
                }
                Some(piece) => {
                    piece.is_legal_move(to, self)
                        && piece.get_color() == player_color
                        && !self.apply_move(m).is_in_check(player_color)
                }
                _ => false,
            },
            Move::Promotion(from, to, promotion) => {
                match self.get_piece(from) {
                    Some(piece) => {
                        // promotion specific checks
                        piece.is_pawn()
                            && (to.get_row() == 0 || to.get_row() == 7)
                            && !(promotion.is_king() || promotion.is_pawn())
                            // regular piece checks
                            && piece.is_legal_move(to, self)
                            && piece.get_color() == player_color
                            && !self.apply_move(m).is_in_check(player_color)
                    }
                    _ => false,
                }
            }
            Move::Resign => true,
        }
    }

    /// Does the respective player have sufficient material?
    pub fn has_sufficient_material(&self, color: Color) -> bool {
        let mut pieces = vec![];
        for square in &self.squares {
            if let Some(piece) = square.get_piece() {
                if piece.get_color() == color {
                    pieces.push(piece);
                }
            }
        }

        pieces.sort();

        if pieces.is_empty()
            || pieces.len() == 1 && pieces[0].is_king()
            || pieces.len() == 2 && pieces[0].is_king() && pieces[1].is_knight()
            || pieces.len() == 2 && pieces[0].is_king() && pieces[1].is_bishop()
            || pieces.len() == 3
                && pieces[0].is_king()
                && pieces[1].is_knight()
                && pieces[2].is_knight()
        {
            false
        } else {
            !(pieces.len() == 3
                && pieces[0].is_king()
                && pieces[1].is_bishop()
                && pieces[2].is_bishop())
        }
    }

    /// Does the respective player have insufficient material?
    #[inline]
    pub fn has_insufficient_material(&self, color: Color) -> bool {
        !self.has_sufficient_material(color)
    }

    /// Is the current player in stalemate?
    pub fn is_stalemate(&self) -> bool {
        (self.get_legal_moves().next().is_none()
            && !self.is_in_check(self.get_current_player_color()))
            || (self.has_insufficient_material(self.turn)
                && self.has_insufficient_material(!self.turn))
    }

    /// Is the current player in checkmate?
    pub fn is_checkmate(&self) -> bool {
        self.is_in_check(self.get_current_player_color()) && self.get_legal_moves().next().is_none()
    }

    /// Change the current turn to the next player.
    #[inline]
    pub fn change_turn(mut self) -> Self {
        self.turn = !self.turn;
        self
    }

    fn apply_move(&self, m: Move) -> Self {
        match m {
            Move::KingSideCastle => {
                if let Some(king_pos) = self.get_king_pos(self.turn) {
                    let rook_pos = match self.turn {
                        WHITE => Position::new(0, 7),
                        BLACK => Position::new(7, 7),
                    };
                    self.move_piece(king_pos, rook_pos.next_left(), None)
                        .move_piece(rook_pos, king_pos.next_right(), None)
                } else {
                    *self
                }
            }
            Move::QueenSideCastle => {
                if let Some(king_pos) = self.get_king_pos(self.turn) {
                    let rook_pos = match self.turn {
                        WHITE => Position::new(0, 0),
                        BLACK => Position::new(7, 0),
                    };
                    self.move_piece(king_pos, king_pos.next_left().next_left(), None)
                        .move_piece(rook_pos, king_pos.next_left(), None)
                } else {
                    *self
                }
            }

            Move::Piece(from, to) => {
                let mut result = self.move_piece(from, to, None);

                if let (Some(en_passant), Some(Piece::Pawn(player_color, _))) =
                    (self.en_passant, self.get_piece(from))
                {
                    if (en_passant == from.pawn_up(player_color).next_left()
                        || en_passant == from.pawn_up(player_color).next_right())
                        && en_passant == to
                    {
                        result.squares[((7 - en_passant.pawn_back(player_color).get_row()) * 8
                            + en_passant.get_col())
                            as usize] = EMPTY_SQUARE;
                    }
                }

                result
            }
            Move::Promotion(from, to, promotion) => self.move_piece(from, to, Some(promotion)),
            Move::Resign => self.remove_all(self.turn).queen_all(!self.turn),
        }
    }

    /// Play a move and confirm it is legal.
    pub fn play_move(&self, m: Move) -> GameResult {
        let current_color = self.get_turn_color();

        if m == Move::Resign {
            GameResult::Victory(!current_color)
        } else if self.is_legal_move(m, current_color) {
            let next_turn = self.apply_move(m).change_turn();
            if next_turn.is_checkmate() {
                GameResult::Victory(current_color)
            } else if next_turn.is_stalemate() {
                GameResult::Stalemate
            } else {
                GameResult::Continuing(next_turn)
            }
        } else {
            GameResult::IllegalMove(m)
        }
    }
}

#[cfg(test)]
mod ai_tests {
    use super::*;
    use crate::{get_endgame_move, parse_fen, GameResult, Move, Position, BLACK, WHITE};

    /// A "winning but stalemate-prone" position: White Kb6, Qc1 vs lone Black
    /// Ka8. White to move can mate with Qc8# or can *stalemate* with Qc7.
    /// This is the exact class of blunder the AI used to make.
    fn winning_but_stalemate_prone() -> Board {
        parse_fen("k7/8/1K6/8/8/8/8/2Q5 w - - 0 1").unwrap()
    }

    /// The search must prefer the mate (Qc8#) over the stalemate (Qc7).
    /// Previously both scored the same sentinel, so the AI would stalemate.
    #[test]
    fn search_prefers_mate_over_stalemate() {
        let board = winning_but_stalemate_prone();
        let (mv, _, _) = board.get_best_next_move(3);
        let result = board.play_move(mv);
        // Must be a win, never a stalemate.
        assert!(
            matches!(result, GameResult::Victory(WHITE)),
            "expected checkmate, got {:?} (move {:?})",
            result,
            mv
        );
    }

    /// Regression for "stalemate when winning": the generic LoneKing endgame
    /// picker must never return a move that stalemates the lone defender king,
    /// even for material signatures the specialised pickers don't recognise
    /// (here K + Queen + Rok vs K).
    #[test]
    fn lone_king_picker_does_not_stalemate() {
        let board = parse_fen("k7/8/1K6/8/8/8/8/Q1R5 w - - 0 1").unwrap();
        let mv = get_endgame_move(&board).expect("LoneKing endgame should be detected");
        let nb = board.apply_eval_move(mv);
        assert!(
            !nb.is_stalemate(),
            "LoneKing picker stalemated the defender with move {:?}",
            mv
        );
        assert!(
            nb.has_sufficient_material(WHITE),
            "LoneKing picker threw away material with move {:?}",
            mv
        );
    }

    /// Driving a won KQ+R-vs-K endgame to checkmate (not stalemate) by playing
    /// the LoneKing picker for the attacker and a greedy king move for the
    /// defender.
    #[test]
    fn lone_king_endgame_mates_not_stalemates() {
        let mut board = parse_fen("k7/8/1K6/8/8/8/8/Q1R5 w - - 0 1").unwrap();
        for _ in 0..60 {
            let mv = if board.get_turn_color() == WHITE {
                get_endgame_move(&board).unwrap_or_else(|| board.get_legal_moves().next().unwrap())
            } else {
                board.get_legal_moves().next().unwrap()
            };
            match board.play_move(mv) {
                GameResult::Victory(_) => return, // success: checkmated the lone king
                GameResult::Stalemate => panic!("endgame picker stalemated a won game"),
                GameResult::Continuing(b) => board = b,
                GameResult::IllegalMove(_) => panic!("illegal move in playout"),
            }
        }
        panic!("endgame did not conclude in 60 plies");
    }

    /// Regression for the opening blunder: after 1.e4 Nf6 2.d3 the AI (Black)
    /// used to grab the e4 pawn with the knight (Nfxe4), losing the knight to
    /// dxe4. With captures never dropped from the sample + the en-prise-to-pawn
    /// eval term, the search must avoid Nxe4.
    #[test]
    fn does_not_blunder_knight_for_pawn_in_alekhine() {
        let board =
            parse_fen("rnbqkb1r/pppppppp/8/8/4P3/3P4/PPP2PPP/RNBQKBNR b KQkq - 0 2").unwrap();
        let bad = Move::Piece(Position::pgn("f6").unwrap(), Position::pgn("e4").unwrap());
        let (mv, _, _) = board.get_best_next_move(2);
        assert_ne!(mv, bad, "AI blundered the knight with Nfxe4");
    }

    /// The en-prise-to-pawn eval helper: a knight on c4 is attacked by a black
    /// d5 pawn; a knight on a4 is not.
    #[test]
    fn square_attacked_by_pawn_detects_pawn_captures() {
        let board = parse_fen("4k3/8/8/3p4/8/8/8/4K3 b - - 0 1").unwrap();
        let c4 = Position::pgn("c4").unwrap();
        let a4 = Position::pgn("a4").unwrap();
        let e4 = Position::pgn("e4").unwrap();
        assert!(board.square_attacked_by_pawn(c4, BLACK));
        assert!(board.square_attacked_by_pawn(e4, BLACK));
        assert!(!board.square_attacked_by_pawn(a4, BLACK));
    }

    /// Faster mates must score higher than slower ones (mate-distance scoring),
    /// so the engine prefers the quickest kill.
    #[test]
    fn value_for_penalises_knight_en_prise_to_pawn() {
        let attacked = parse_fen("4k3/8/8/3p4/2N5/8/8/4K3 w - - 0 1").unwrap();
        let safe = parse_fen("4k3/8/8/3p4/N7/8/8/4K3 w - - 0 1").unwrap();
        // Same material; only difference is the knight sits on a pawn-attacked
        // square in `attacked`. Its eval for White must be lower.
        assert!(
            attacked.value_for(WHITE) < safe.value_for(WHITE),
            "en-prise knight should score lower"
        );
    }
}
