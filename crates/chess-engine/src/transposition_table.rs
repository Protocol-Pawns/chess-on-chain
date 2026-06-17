use crate::Move;
use std::collections::HashMap;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum TtFlag {
    Exact,
    LowerBound,
    UpperBound,
}

#[derive(Clone, Copy, Debug)]
pub struct TtEntry {
    pub depth: u8,
    pub flag: TtFlag,
    pub value: f64,
    pub best_move: Option<Move>,
}

/// Simple transposition table with a fixed size ceiling.
/// Entries are keyed by a position hash combined with the search-depth context.
pub struct TranspositionTable {
    map: HashMap<u64, TtEntry>,
    max_size: usize,
}

impl TranspositionTable {
    pub fn new(max_size: usize) -> Self {
        Self {
            map: HashMap::with_capacity(max_size.min(4096)),
            max_size,
        }
    }

    pub fn get(&self, key: u64) -> Option<&TtEntry> {
        self.map.get(&key)
    }

    pub fn store(
        &mut self,
        key: u64,
        depth: u8,
        flag: TtFlag,
        value: f64,
        best_move: Option<Move>,
    ) {
        if self.map.len() >= self.max_size {
            // Evict only if the same position is already stored with lower depth.
            if let Some(existing) = self.map.get(&key) {
                if existing.depth > depth {
                    return;
                }
            } else {
                return;
            }
        }
        self.map.insert(
            key,
            TtEntry {
                depth,
                flag,
                value,
                best_move,
            },
        );
    }
}

/// Build a key that includes the remaining search context.
/// Because the engine samples a variable number of moves per ply,
/// the same board reached with different remaining width lists is not
/// safely interchangeable, so we fold the width list into the key.
pub fn tt_context_key(
    zobrist: u64,
    depth: &either::Either<u8, (&[u8], rand_chacha::ChaCha20Rng)>,
) -> u64 {
    let ctx = match depth {
        either::Either::Left(d) => {
            // 64 distinguishes a fixed-depth search from a width-list search.
            64u64.wrapping_shl(56) | (*d as u64)
        }
        either::Either::Right((widths, _)) => {
            let mut h: u64 = widths.len() as u64;
            for &w in *widths {
                h = h.wrapping_mul(31).wrapping_add(w as u64);
            }
            h
        }
    };
    zobrist ^ ctx.wrapping_mul(0x9E3779B97F4A7C15)
}
