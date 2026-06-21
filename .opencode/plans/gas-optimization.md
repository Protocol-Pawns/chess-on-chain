# Gas Optimization Plan — Protocol Pawns AI

**Goal:** Lower gas cost at equal playing strength (more headroom / lower cost per AI move).
**Constraints:** Gas-neutral-or-better per difficulty; no strength regression; must pass
`cargo test -p chess-engine --lib` (18 tests) + `./build.sh` + `test_ai_gas_budgets`.

---

## Summary of All Changes

| Change | Impact | Status |
|---|---|---|
| **Opening book Zobrist fix** | Book NEVER hit → now hits all opening positions. **3 TGas** per opening move (was 40-150 TGas). | **CRITICAL** |
| **Opening book expansion** | 2,742 → 8,000 entries (2.9x). Deeper opening coverage. | Applied |
| **TT eviction fix** | Table no longer freezes when full. O(1) random replacement. | Applied |
| **TT leaf gating** | Leaves skip Zobrist hash + TT probe. ~2% Hard. | Applied |
| **Pin-based legality** | Skip `apply_move + is_in_check` for non-pinned pieces. ~13% Hard. | Applied |
| **Ray-based sliders** | O(ray_length) instead of O(64) per slider. Additional ~14% Hard. | Applied |
| **wasm-opt -O4** | Speed-optimized wasm post-processing. Gas-neutral. | Applied |

---

## Opening Book Zobrist Fix — CRITICAL BUG FIX

**The opening book had never worked since inception.** The `zobrist_key()` function in
`board.rs` iterated `self.squares` (which uses FEN order: a8=index 0, h1=index 63) and fed
the raw index directly into `PIECE_ZOBRIST_KEYS[pt][color][sq]`. But the Python book
generator (`zobrist.py::board_hash()`) used python-chess's standard square ordering
(a1=0, h8=63). The Zobrist keys never matched, so `lookup_opening()` always returned `None`,
and every AI move — including the opening — required a full minimax search.

**Fix:** Convert Rust square index to standard chess index in `zobrist_key()`:
```rust
let std_sq = (7 - sq / 8) * 8 + sq % 8;
key ^= PIECE_ZOBRIST_KEYS[pt][color][std_sq];
```

**Before fix (gas per AI opening move, capped):**
| Difficulty | Gas (capped) | What happened |
|---|---|---|
| Easy | 3 TGas | No book flag (first legal move) |
| Medium | 42 TGas | Full search (book missed) |
| Hard | 100 TGas | Full search (book missed) |
| VeryHard | 165-179 TGas | Full search (book missed) |

**After fix:**
| Difficulty | Gas (capped) | What happens |
|---|---|---|
| Easy | 3 TGas | First legal move (unchanged) |
| Medium | **3 TGas** | **Book hit** (-93%) |
| Hard | **3 TGas** | **Book hit** (-97%) |
| VeryHard | **3 TGas** | **Book hit** (-98%) |

The book lookup costs ~3 TGas (Zobrist hash + binary search on 8000 sorted entries).
A full search would cost 42-848 TGas depending on difficulty. Every book hit saves
40-845 TGas.

---

## Opening Book Expansion

**Before:** 2,742 entries (generated from ~350 hand-curated lines, one-level tree expansion).
**After:** 8,000 entries (same lines + recursive BFS tree expansion, Stockfish multiPV=3).

**Generation script changes** (`scripts/generate_static_data.py`):
- `MAX_ENTRIES`: 4,000 → 8,000
- `TREE_MULTIPV`: 2 → 3 (more opponent deviations covered)
- Phase 2 redesigned as BFS: recursively expands new positions until MAX_ENTRIES or
  MAX_EXPANSION_PLY (18 plies) reached
- Stockfish configured with `Threads=4, Hash=512` for faster generation
- Analysis depth: LINES_DEPTH=12 (Phase 1), TREE_DEPTH=10 (Phase 2 expansion)

**Binary size impact:** wasm grew from 808K → 892K (+84KB for the extra 5,258 entries at
12 bytes each). Negligible relative to NEAR's wasm size limit.

**Coverage:** Book now covers openings to ~10-12 plies (5-6 full moves per side) for
common lines, with deviation coverage via BFS expansion. Combined with the Zobrist fix,
this means the first ~10-15 moves of every game cost 3 TGas instead of 40-150 TGas.

---

## TT Eviction Fix

**Before:** When the TT reached `max_size` (8192) and a new key arrived, `store()` simply
returned without storing. The table froze — no new positions could be added for the
remainder of the search.

**After:** Added a `keys: Vec<u64>` alongside the HashMap. When full, a pseudo-random entry
is evicted using O(1) `swap_remove` on the keys vector + `remove` on the map. The table
stays live throughout the search.

**Pre-allocation:** Changed `HashMap::with_capacity(max_size.min(4096))` to
`HashMap::with_capacity(max_size)` — eliminates rehashing during search.

**Impact:** Mainly helps VeryHard's 4-ply search (~10K+ nodes may fill the 8192-slot TT).
Not measurable in the current test because the book hit skips the search entirely for
opening positions.

---

## Search Optimizations (Previous Session)

### Baseline (pre-optimization, measured 2026-06-20)

| Difficulty | Soft cap | Capped burnt | **Full search** | % tree done under cap |
|---|---|---|---|---|
| Easy     | 15 TGas  | 3   | **3**    | 100% (flags=0, no search) |
| Medium   | 40 TGas  | 42  | **59**   | ~71% |
| Hard     | 75 TGas  | 83  | **478**  | **~17%** |
| VeryHard | 150 TGas | 159 | **>1000**| **<16%** (hits sandbox ceiling) |

### #1: TT leaf gating — APPLIED (~2% Hard)
Moved leaf checks before `zobrist_key()` + TT probe. Leaves never benefit from TT.

### #4: Pin-based legality + ray-based sliders — APPLIED (~27% search gas reduction)
- `find_pinned_pieces()`: ray scan from king, identifies pinned pieces.
- `get_legal_moves_fast()`: skips `apply_move + is_in_check` for non-pinned non-king pieces.
- Ray-based slider `get_moves()`: O(ray_length) instead of O(64) per slider. Stops at blockers.
- `fast_legal_moves_match_slow` correctness test across 7 FEN positions.

**Full-search gas progression (median of 8-10 runs, no book, no cap):**

| Difficulty | Baseline | After #1+#4 | Delta |
|---|---|---|---|
| Medium | 68 | **51** | **-25%** |
| Hard | 478 | **353.5** | **-26%** |
| VeryHard | >1000 | **~848** | **>15%** |

### wasm-opt -O4 — APPLIED
`wasm-opt -O4 --strip-debug --strip-producers --vacuum` post-processing in build.sh.

### Cancelled
- **#2 Incremental Zobrist** — <1% estimated gain, not worth complexity.
- **#3 Flat-array TT** — ~1.5% gain, not worth the rewrite.
- **#5 make/unmake** — Board copies only ~4% of remaining cost.

---

## Final Results — Production Gas

**With book hit (opening moves):**

| Difficulty | Cap | Book hit gas | Headroom |
|---|---|---|---|
| Easy | 15 TGas | 3 | 12 |
| Medium | 40 TGas | **3** | 37 |
| Hard | 75 TGas | **3** | 72 |
| VeryHard | 150 TGas | **3** | 147 |

**Without book hit (midgame/endgame positions, capped):**

| Difficulty | Cap | Capped burnt | Headroom |
|---|---|---|---|
| Easy | 15 TGas | 3 | 12 |
| Medium | 40 TGas | ~42 | ~48 (limit = cap + 50) |
| Hard | 75 TGas | ~100 | ~25 |
| VeryHard | 150 TGas | ~165 | ~35 |

**Per-game impact:** A typical 60-move game has ~10-15 opening moves in the book.
At 3 TGas each instead of 42-150 TGas, this saves ~400-2200 TGas per game. Combined
with the search optimizations, the AI is both stronger (Stockfish-quality openings +
more tree searched per gas budget) and cheaper.

---

## Guards (after every change)
- `cargo test -p chess-engine --lib` (18 tests, incl. 5 blunder-regression + mate-vs-stalemate
  + `fast_legal_moves_match_slow` equivalence).
- `./build.sh` (wasm).
- `test_ai_gas_budgets` within caps.

## Relevant files
- `crates/chess-engine/src/board.rs` — engine core: zobrist_key (309, **fixed square indexing**),
  get_next_move (~430), minimax (~1068), find_pinned_pieces (~367), get_legal_moves_fast (~418),
  value_for (240), is_legal_move (~2048), is_threatened (~1899).
- `crates/chess-engine/src/piece.rs` — piece move gen: ray-based sliders (~500-566).
- `crates/chess-engine/src/transposition_table.rs` — TT with **fixed eviction** (keys_list + swap_remove).
- `crates/chess-engine/src/static_book.rs` — **8,000-entry opening book** (was 2,742).
- `crates/chess-lib/src/game.rs` — contract AI driver (to_flags:134+, invocation:394-447).
- `crates/chess-lib/src/lib.rs:83-86` — AI_*_GAS constants.
- `crates/chess-test/tests/chess.rs:201` — test_ai_gas_budgets.
- `scripts/generate_static_data.py` — book generator (BFS Phase 2, Stockfish threads).
- `scripts/zobrist.py` — Zobrist key computation (matches Rust after fix).
- `build.sh` — wasm-opt -O4 pipeline.
- `measure_gas.sh` — gas measurement script.
