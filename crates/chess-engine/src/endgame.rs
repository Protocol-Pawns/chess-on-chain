use crate::{Board, Color, Move, Position, BLACK, WHITE};

enum EndgameType {
    Krvk,
    Kqvk,
    Kpvk,
    Kbbvk,
    Kbnvk,
}

/// Detect which endgame type is on the board for the side that has
/// mating material. The side with extra material is the "attacker".
fn detect_endgame(board: &Board) -> Option<(EndgameType, Color)> {
    let mut wk = false;
    let mut wq = false;
    let mut wr = false;
    let mut wb = 0u32;
    let mut wn = 0u32;
    let mut wp = 0u32;
    let mut bk = false;
    let mut bq = false;
    let mut br = false;
    let mut bb = 0u32;
    let mut bn = 0u32;
    let mut bp = 0u32;

    for row in 0..8 {
        for col in 0..8 {
            let pos = Position::new(row, col);
            if let Some(piece) = board.get_piece(pos) {
                let c = piece.get_color();
                match piece.get_name() {
                    "king" => {
                        if c == WHITE {
                            wk = true
                        } else {
                            bk = true
                        }
                    }
                    "queen" => {
                        if c == WHITE {
                            wq = true
                        } else {
                            bq = true
                        }
                    }
                    "rook" => {
                        if c == WHITE {
                            wr = true
                        } else {
                            br = true
                        }
                    }
                    "bishop" => {
                        if c == WHITE {
                            wb += 1
                        } else {
                            bb += 1
                        }
                    }
                    "knight" => {
                        if c == WHITE {
                            wn += 1
                        } else {
                            bn += 1
                        }
                    }
                    "pawn" => {
                        if c == WHITE {
                            wp += 1
                        } else {
                            bp += 1
                        }
                    }
                    _ => {}
                }
            }
        }
    }

    let white_only = wk && !wq && !wr && wb == 0 && wn == 0 && wp == 0;
    let black_only = bk && !bq && !br && bb == 0 && bn == 0 && bp == 0;

    if wr && white_only && bk && !bq && !br && bb == 0 && bn == 0 && bp == 0 {
        return Some((EndgameType::Krvk, WHITE));
    }
    if br && black_only && wk && !wq && !wr && wb == 0 && wn == 0 && wp == 0 {
        return Some((EndgameType::Krvk, BLACK));
    }
    if wq && white_only && bk && !bq && !br && bb == 0 && bn == 0 && bp == 0 {
        return Some((EndgameType::Kqvk, WHITE));
    }
    if bq && black_only && wk && !wq && !wr && wb == 0 && wn == 0 && wp == 0 {
        return Some((EndgameType::Kqvk, BLACK));
    }
    if wp == 1 && white_only && bk && !bq && !br && bb == 0 && bn == 0 && bp == 0 {
        return Some((EndgameType::Kpvk, WHITE));
    }
    if bp == 1 && black_only && wk && !wq && !wr && wb == 0 && wn == 0 && wp == 0 {
        return Some((EndgameType::Kpvk, BLACK));
    }
    if wb >= 2 && white_only && bk && !bq && !br && bb == 0 && bn == 0 && bp == 0 {
        return Some((EndgameType::Kbbvk, WHITE));
    }
    if bb >= 2 && black_only && wk && !wq && !wr && wb == 0 && wn == 0 && wp == 0 {
        return Some((EndgameType::Kbbvk, BLACK));
    }
    if wb == 1 && wn == 1 && white_only && bk && !bq && !br && bb == 0 && bn == 0 && bp == 0 {
        return Some((EndgameType::Kbnvk, WHITE));
    }
    if bb == 1 && bn == 1 && black_only && wk && !wq && !wr && wb == 0 && wn == 0 && wp == 0 {
        return Some((EndgameType::Kbnvk, BLACK));
    }
    None
}

// ── Distance helpers ──

fn king_distance(a: Position, b: Position) -> i32 {
    (a.get_row() - b.get_row()).abs() + (a.get_col() - b.get_col()).abs()
}

/// Chebyshev distance (king-move distance) — more accurate than Manhattan for kings.
fn king_move_dist(a: Position, b: Position) -> i32 {
    let dr = (a.get_row() - b.get_row()).abs();
    let dc = (a.get_col() - b.get_col()).abs();
    dr.max(dc)
}

/// Distance from the nearest edge. 0 = on edge, 3 = center.
fn edge_dist(pos: Position) -> i32 {
    let r = pos.get_row();
    let c = pos.get_col();
    r.min(7 - r).min(c).min(7 - c)
}

/// Manhattan distance to the nearest corner. 0 = in corner, ~6-7 = center.
fn corner_dist(pos: Position) -> i32 {
    let r = pos.get_row();
    let c = pos.get_col();
    (r + c)
        .min(r + (7 - c))
        .min((7 - r) + c)
        .min((7 - r) + (7 - c))
}

// ── Opposition & key squares ──

/// Returns true if the attacker holds the opposition after moving.
/// Opposition: kings on the same line with an even number of squares
/// between them. The side NOT to move has the opposition.
fn has_opposition(ak: Position, dk: Position) -> bool {
    let dr = (ak.get_row() - dk.get_row()).abs();
    let dc = (ak.get_col() - dk.get_col()).abs();

    if ak.get_row() == dk.get_row() && dc > 0 && dc % 2 == 0 {
        return true;
    }
    if ak.get_col() == dk.get_col() && dr > 0 && dr % 2 == 0 {
        return true;
    }
    if dr == dc && dr > 0 && dr % 2 == 0 {
        return true;
    }
    false
}

/// Returns true if `king` is on a key square for `pawn` (attacker perspective).
/// Key squares are the three squares two ranks ahead of the pawn (or one rank
/// ahead once the pawn reaches the 5th rank).
fn on_key_square(king: Position, pawn: Position, attacker: Color) -> bool {
    let pr = pawn.get_row();
    let pc = pawn.get_col();
    let kr = king.get_row();
    let kc = king.get_col();

    let target_row = if attacker == WHITE {
        if pr <= 3 {
            pr + 2
        } else {
            pr + 1
        }
    } else {
        if pr >= 4 {
            pr - 2
        } else {
            pr - 1
        }
    };

    kr == target_row && kc >= (pc - 1).max(0) && kc <= (pc + 1).min(7)
}

/// Find the position of the attacker's non-king piece (rook, queen, etc.).
fn find_attacker_piece(board: &Board, attacker: Color, name: &str) -> Option<Position> {
    for row in 0..8 {
        for col in 0..8 {
            let pos = Position::new(row, col);
            if let Some(p) = board.get_piece(pos) {
                if p.get_color() == attacker && p.get_name() == name {
                    return Some(pos);
                }
            }
        }
    }
    None
}

/// Check if the attacker's piece is safe (not hanging to the defender's king).
fn piece_is_safe(nb: &Board, pos: Position, attacker: Color) -> bool {
    !nb.is_threatened(pos, attacker)
}

// ── Endgame scorers ──

fn krvk_move(board: &Board, attacker: Color, defender: Color) -> Option<Move> {
    let dk = board.get_king_pos(defender)?;
    let ak = board.get_king_pos(attacker)?;
    let rook_sq = find_attacker_piece(board, attacker, "rook")?;

    let mut best: Option<Move> = None;
    let mut best_score = f64::NEG_INFINITY;

    for mv in board.get_legal_moves() {
        let nb = board.apply_eval_move(mv);
        let ndk = nb.get_king_pos(defender).unwrap_or(dk);
        let nak = nb.get_king_pos(attacker).unwrap_or(ak);
        let nrook = find_attacker_piece(&nb, attacker, "rook").unwrap_or(rook_sq);

        if nb.is_checkmate() {
            return Some(mv);
        }

        let mut score = 0.0;

        if nb.is_stalemate() || !nb.has_sufficient_material(attacker) {
            score -= 1000.0;
        }

        // REWARD driving defender toward edge (was inverted before)
        score += (3 - edge_dist(ndk)) as f64 * 12.0;

        // REWARD bringing kings together for opposition / restriction
        score -= king_move_dist(nak, ndk) as f64 * 3.0;

        // REWARD opposition (attacker holds it after this move)
        if has_opposition(nak, ndk) {
            score += 20.0;
        }

        // REWARD checks, especially when kings are close (rook can force king back)
        if nb.is_in_check(defender) {
            score += 15.0;
            if king_move_dist(nak, ndk) <= 2 {
                score += 25.0;
            }
        }

        // Don't hang the rook to the enemy king
        if !piece_is_safe(&nb, nrook, attacker) {
            score -= 200.0;
        }

        // Slightly prefer rook moves that restrict the enemy king's box
        if let Move::Piece(from, _) = mv {
            if let Some(p) = board.get_piece(from) {
                if p.is_rook() {
                    // Count how many files/ranks the rook cuts off
                    let rr = nrook.get_row();
                    let rc = nrook.get_col();
                    let dk_r = ndk.get_row();
                    let dk_c = ndk.get_col();
                    // Rook on same rank as defender king but farther = restricts to a smaller box
                    if rr == dk_r || rc == dk_c {
                        score += 5.0;
                    }
                    // Rook restricts defender to one side
                    if rc != dk_c {
                        score += 2.0;
                    }
                }
            }
        }

        if score > best_score {
            best_score = score;
            best = Some(mv);
        }
    }
    best
}

fn kqvk_move(board: &Board, attacker: Color, defender: Color) -> Option<Move> {
    let dk = board.get_king_pos(defender)?;
    let ak = board.get_king_pos(attacker)?;
    let queen_sq = find_attacker_piece(board, attacker, "queen")?;

    let mut best: Option<Move> = None;
    let mut best_score = f64::NEG_INFINITY;

    for mv in board.get_legal_moves() {
        let nb = board.apply_eval_move(mv);
        let ndk = nb.get_king_pos(defender).unwrap_or(dk);
        let nak = nb.get_king_pos(attacker).unwrap_or(ak);
        let nqueen = find_attacker_piece(&nb, attacker, "queen").unwrap_or(queen_sq);

        if nb.is_checkmate() {
            return Some(mv);
        }

        let mut score = 0.0;

        if nb.is_stalemate() || !nb.has_sufficient_material(attacker) {
            score -= 1000.0;
        }

        // REWARD driving defender toward edge
        score += (3 - edge_dist(ndk)) as f64 * 12.0;

        // REWARD bringing kings together
        score -= king_move_dist(nak, ndk) as f64 * 3.0;

        // REWARD opposition
        if has_opposition(nak, ndk) {
            score += 15.0;
        }

        // REWARD checks
        if nb.is_in_check(defender) {
            score += 10.0;
            if king_move_dist(nak, ndk) <= 2 {
                score += 20.0;
            }
        }

        // Don't hang the queen (stalemate risk is high with queen)
        if !piece_is_safe(&nb, nqueen, attacker) {
            score -= 200.0;
        }

        // EXTRA stalemate avoidance: queen covers many squares, so penalize
        // positions where the defender has very few legal moves AND is not in check
        if !nb.is_in_check(defender) {
            let defender_moves = nb.get_legal_moves().count();
            if defender_moves <= 1 {
                score -= 50.0; // likely stalemate next move
            }
        }

        if score > best_score {
            best_score = score;
            best = Some(mv);
        }
    }
    best
}

fn kpvk_move(board: &Board, attacker: Color, defender: Color) -> Option<Move> {
    let ak = board.get_king_pos(attacker)?;
    let dk = board.get_king_pos(defender)?;

    // Find the pawn
    let mut pawn_sq = None;
    for row in 0..8 {
        for col in 0..8 {
            let pos = Position::new(row, col);
            if let Some(p) = board.get_piece(pos) {
                if p.get_color() == attacker && p.is_pawn() {
                    pawn_sq = Some(pos);
                    break;
                }
            }
        }
    }
    let pawn_sq = pawn_sq?;

    let mut best: Option<Move> = None;
    let mut best_score = f64::NEG_INFINITY;

    let is_pawn_move = |mv: Move| -> bool {
        if let Move::Piece(from, _) = mv {
            board.get_piece(from).map_or(false, |p| p.is_pawn())
        } else {
            false
        }
    };

    for mv in board.get_legal_moves() {
        let nb = board.apply_eval_move(mv);
        let ndk = nb.get_king_pos(defender).unwrap_or(dk);
        let nak = nb.get_king_pos(attacker).unwrap_or(ak);

        if nb.is_checkmate() {
            return Some(mv);
        }

        let mut score = 0.0;

        if nb.is_stalemate() || !nb.has_sufficient_material(attacker) {
            score -= 1000.0;
        }

        // Find the pawn in the new position (it may have moved)
        let mut npawn_sq: Option<Position> = Some(pawn_sq);
        if is_pawn_move(mv) {
            if let Move::Piece(_, to) = mv {
                npawn_sq = Some(to);
            }
        }

        // Pawn advancement bonus (higher = closer to promotion)
        if let Some(np) = npawn_sq {
            let advance = if attacker == WHITE {
                np.get_row()
            } else {
                7 - np.get_row()
            };
            score += advance as f64 * 8.0;

            // If pawn is one square from promotion, strongly prefer pushing
            if advance == 6 {
                score += 100.0;
            }

            // REWARD king on a key square
            if on_key_square(nak, np, attacker) {
                score += 30.0;
            }

            // Keep king close to the pawn
            let kd = king_move_dist(nak, np);
            score -= kd as f64 * 2.0;
        }

        // REWARD opposition — critical for KPvK technique
        if has_opposition(nak, ndk) {
            score += 25.0;
        }

        // PENALIZE pushing the pawn when the king is NOT well-placed.
        // Pushing the pawn without key squares or opposition often draws.
        if is_pawn_move(mv) {
            let was_on_key = on_key_square(ak, pawn_sq, attacker);
            let had_opp = has_opposition(ak, dk);
            if !was_on_key && !had_opp {
                score -= 15.0;
            }
        }

        // REWARD driving the defender king away from the pawn's path
        if let Some(np) = npawn_sq {
            let pawn_file = np.get_col();
            // Defender king should not be blockading the pawn
            if ndk.get_col() == pawn_file {
                let block_row = if attacker == WHITE {
                    ndk.get_row() > np.get_row()
                } else {
                    ndk.get_row() < np.get_row()
                };
                if block_row {
                    score -= 10.0; // defender is blockading
                }
            }
        }

        if score > best_score {
            best_score = score;
            best = Some(mv);
        }
    }
    best
}

fn kbbvk_move(board: &Board, attacker: Color, defender: Color) -> Option<Move> {
    let dk = board.get_king_pos(defender)?;
    let ak = board.get_king_pos(attacker)?;

    let mut best: Option<Move> = None;
    let mut best_score = f64::NEG_INFINITY;

    for mv in board.get_legal_moves() {
        let nb = board.apply_eval_move(mv);
        let ndk = nb.get_king_pos(defender).unwrap_or(dk);
        let nak = nb.get_king_pos(attacker).unwrap_or(ak);

        if nb.is_checkmate() {
            return Some(mv);
        }

        let mut score = 0.0;

        if nb.is_stalemate() || !nb.has_sufficient_material(attacker) {
            score -= 1000.0;
        }

        // REWARD driving defender toward CORNER (not just edge)
        // corner_dist: 0 = in corner, ~6-7 = center
        score -= corner_dist(ndk) as f64 * 8.0;

        // REWARD bringing kings together to herd the defender
        score -= king_move_dist(nak, ndk) as f64 * 4.0;

        // REWARD opposition to squeeze the king
        if has_opposition(nak, ndk) {
            score += 15.0;
        }

        // REWARD checks
        if nb.is_in_check(defender) {
            score += 10.0;
        }

        if score > best_score {
            best_score = score;
            best = Some(mv);
        }
    }
    best
}

fn kbnvk_move(board: &Board, attacker: Color, defender: Color) -> Option<Move> {
    let dk = board.get_king_pos(defender)?;
    let ak = board.get_king_pos(attacker)?;

    // Find bishop to determine the "right" corner color
    let bishop_sq = find_attacker_piece(board, attacker, "bishop");
    let bishop_color = bishop_sq.map(|p| (p.get_row() + p.get_col()) % 2);

    let mut best: Option<Move> = None;
    let mut best_score = f64::NEG_INFINITY;

    for mv in board.get_legal_moves() {
        let nb = board.apply_eval_move(mv);
        let ndk = nb.get_king_pos(defender).unwrap_or(dk);
        let nak = nb.get_king_pos(attacker).unwrap_or(ak);

        if nb.is_checkmate() {
            return Some(mv);
        }

        let mut score = 0.0;

        if nb.is_stalemate() || !nb.has_sufficient_material(attacker) {
            score -= 1000.0;
        }

        // REWARD driving defender toward the correct corner (bishop's color).
        // For KBNvK, mate must be delivered in a corner matching the bishop's diagonal color.
        // Corners: a1=(0,0) sum=0 even, h1=(0,7) sum=7 odd, a8=(7,0) sum=7 odd, h8=(7,7) sum=14 even
        let cd = corner_dist(ndk) as f64;
        score -= cd * 6.0;

        // Extra reward if defender is near the correct-color corner
        if let Some(bcolor) = bishop_color {
            let corner_color_matches = (ndk.get_row() + ndk.get_col()) % 2 == bcolor;
            if corner_color_matches {
                score += 10.0;
            }
        }

        // REWARD bringing kings together
        score -= king_move_dist(nak, ndk) as f64 * 4.0;

        // REWARD opposition
        if has_opposition(nak, ndk) {
            score += 15.0;
        }

        // REWARD checks
        if nb.is_in_check(defender) {
            score += 10.0;
        }

        if score > best_score {
            best_score = score;
            best = Some(mv);
        }
    }
    best
}

pub fn get_endgame_move(board: &Board) -> Option<Move> {
    let (egtype, attacker) = detect_endgame(board)?;
    let defender = if attacker == WHITE { BLACK } else { WHITE };

    if board.get_turn_color() != attacker {
        return None;
    }

    match egtype {
        EndgameType::Krvk => krvk_move(board, attacker, defender),
        EndgameType::Kqvk => kqvk_move(board, attacker, defender),
        EndgameType::Kpvk => kpvk_move(board, attacker, defender),
        EndgameType::Kbbvk => kbbvk_move(board, attacker, defender),
        EndgameType::Kbnvk => kbnvk_move(board, attacker, defender),
    }
}
