#!/usr/bin/env python3
"""Generate static opening book data for the WASM binary.

Two-phase generation:
  Phase 1: Walk ~350 hand-curated opening lines (depth 20, single PV).
  Phase 2: Tree-expand every book position with Stockfish multiPV=2
           to cover opponent deviations (depth 16).

Run from repo root with:
    PYTHONPATH=scripts/.pydeps/chess-1.11.2 python3 scripts/generate_static_data.py
"""

import sys, subprocess, time
from pathlib import Path

_PYPATH = Path(__file__).parent / ".pydeps" / "chess-1.11.2"
if str(_PYPATH) not in sys.path:
    sys.path.insert(0, str(_PYPATH))

import chess, chess.engine
from zobrist import board_hash

ROOT = Path(__file__).parent.parent
ENGINE = ROOT / "scripts" / ".pydeps" / "stockfish" / "stockfish-ubuntu-x86-64"
OUT = ROOT / "crates" / "chess-engine" / "src" / "static_book.rs"

LINES_DEPTH = 12
TREE_DEPTH = 10
TREE_MULTIPV = 3
BOOK_MULTIPV = 3
MAX_ENTRIES = 8000
MAX_EXPANSION_PLY = 18


def ensure_stockfish():
    if not ENGINE.exists():
        subprocess.run(["bash", str(ROOT / "scripts" / "setup.sh")], check=True)


def encode_move_uci(move):
    if move is None: return 0
    if move.from_square == chess.E1 and move.to_square == chess.G1: return 5<<12
    if move.from_square == chess.E1 and move.to_square == chess.C1: return 6<<12
    if move.from_square == chess.E8 and move.to_square == chess.G8: return 5<<12
    if move.from_square == chess.E8 and move.to_square == chess.C8: return 6<<12
    pr = 0
    if move.promotion: pr = {chess.KNIGHT:1, chess.BISHOP:2, chess.ROOK:3, chess.QUEEN:4}[move.promotion]
    return (move.from_square&0x3F) | ((move.to_square&0x3F)<<6) | (pr<<12)


LINES = [
    # ===== e4 e5 (Open Games) =====
    # Ruy Lopez - Main Line
    "e2e4 e7e5 g1f3 b8c6 f1b5 a7a6 b5a4 g8f6 e1g1 f6e4 d2d4 b7b5 a4b3 d7d5 d4e5 c6e5 f3e5 f8e7",
    "e2e4 e7e5 g1f3 b8c6 f1b5 a7a6 b5a4 g8f6 e1g1 f6e4 d2d4 d7d6 f1e1 f8e7 d4e5 d6e5",
    "e2e4 e7e5 g1f3 b8c6 f1b5 a7a6 b5a4 g8f6 e1g1 f6e4 d2d4 b7b5 a4b3 d7d5 e4d5 c6e7 d1d5 e7d5",
    "e2e4 e7e5 g1f3 b8c6 f1b5 a7a6 b5a4 g8f6 e1g1 f8e7 f1e1 e7f8 e1e1 f8e7",
    # Ruy Lopez - Exchange
    "e2e4 e7e5 g1f3 b8c6 f1b5 a7a6 b5c6 d7c6 e1g1 g8f6 d2d4 f6e4 d1d4 f8b4 d4d3 b4d6",
    "e2e4 e7e5 g1f3 b8c6 f1b5 a7a6 b5c6 d7c6 e1g1 f8c5 c2c3 f8e7 d2d4 e5d4 c3d4",
    # Ruy Lopez - Closed
    "e2e4 e7e5 g1f3 b8c6 f1b5 a7a6 b5a4 d7d6 d2d4 b7b5 a4b3 c7c6 c2c3 f8e7 e1g1",
    "e2e4 e7e5 g1f3 b8c6 f1b5 a7a6 b5a4 d7d6 d2d4 b7b5 a4b3 c7c6 c2c3 f8e7 e1g1 e7f6 f1e1 e8g8",
    # Ruy Lopez - Archangelsk / Moller
    "e2e4 e7e5 g1f3 b8c6 f1b5 a7a6 b5a4 g8f6 e1g1 f8c5 c2c3 f8b4",
    # Italian / Giuoco Piano
    "e2e4 e7e5 g1f3 b8c6 f1c4 f8c5 c2c3 g8f6 d2d4 e5d4 c3d4 f8b4 d4d5 b4d5 c4d5 c6a5 d1d5 d8d5",
    "e2e4 e7e5 g1f3 b8c6 f1c4 f8c5 c2c3 g8f6 d2d4 e5d4 c3d4 f8b4 f1d3 b4c3 b2c3 d7d5 e4d5 f6d5 c4d5 c6d5",
    "e2e4 e7e5 g1f3 b8c6 f1c4 f8c5 e1g1 g8f6 d2d3 d7d6 c2c3 f8e7 f1e1 e8g8",
    "e2e4 e7e5 g1f3 b8c6 f1c4 f8c5 b1c3 g8f6 d2d3 d7d6 f1g5 h7h6 g5f6 d8f6",
    # Italian Quiet / Giuco Pianissimo
    "e2e4 e7e5 g1f3 b8c6 f1c4 f8c5 d2d3 g8f6 b1c3 d7d6 f1g3 a7a6 a2a3 f8a7 c2c3 e8g8 e1g1",
    "e2e4 e7e5 g1f3 b8c6 f1c4 g8f6 d2d3 f8e7 c2c3 e8g8 e1g1 d7d6",
    # Two Knights Defense
    "e2e4 e7e5 g1f3 b8c6 f1c4 g8f6 d2d4 e5d4 e1g1 f6e4 d1e2 d7d5 c4d5 f7f5 d5e4 f5e4 e2e4 d8e7",
    "e2e4 e7e5 g1f3 b8c6 f1c4 g8f6 d2d4 e5d4 e1g1 d7d5 c4d5 c6a5 d1d4 c7c6 d5c6 b7c6",
    "e2e4 e7e5 g1f3 b8c6 f1c4 g8f6 d2d3 f8e7 e1g1 e8g8 c2c3 d7d5",
    "e2e4 e7e5 g1f3 b8c6 f1c4 g8f6 b1c3 g7g6 d2d3 f8g7 e1g1 e8g8",
    # Evans Gambit
    "e2e4 e7e5 g1f3 b8c6 f1c4 f8c5 b2b4 c5b4 c2c3 b4c5 d2d4 e5d4 c3d4 c5b4 c1b2",
    # Scotch
    "e2e4 e7e5 g1f3 b8c6 d2d4 e5d4 f3d4 g8f6 b1c3 f8b4 d4c6 b7c6 f1e2 e8g8 e1g1 d7d6",
    "e2e4 e7e5 g1f3 b8c6 d2d4 e5d4 f3d4 f8c5 d4b3 c5b6 b1c3 g8f6 c2c3 e8g8 f1e2 d7d6",
    "e2e4 e7e5 g1f3 b8c6 d2d4 e5d4 c2c3 d4c3 b1c3 d7d5 e4d5 d8d5 d1d5 c6d5",
    "e2e4 e7e5 g1f3 b8c6 d2d4 e5d4 f3d4 g8f6 e4e5 f6d5 b1c3 d5b4 d4b5 a7a6 b5c3",
    # Four Knights
    "e2e4 e7e5 g1f3 b8c6 b1c3 g8f6 d2d4 e5d4 f3d4 f8b4 d4c6 b7c6 f1d3 d7d5 e4d5 c6d5 e1g1 e8g8",
    "e2e4 e7e5 g1f3 b8c6 b1c3 g8f6 f1b5 f8b4 e1g1 e1g1 c2c3 f8e7 d2d4",
    "e2e4 e7e5 g1f3 b8c6 b1c3 g8f6 f1c4 f8c5 d2d3 d7d6 c2c3",
    "e2e4 e7e5 g1f3 b8c6 b1c3 g8f6 f1c4 f8e7 d2d3 d7d6 e1g1",
    # Petroff
    "e2e4 e7e5 g1f3 g8f6 f3e5 d7d6 e5f3 f6e4 d2d4 d6d5 f1d3 f8c5 e1g1 e1g1",
    "e2e4 e7e5 g1f3 g8f6 f3e5 d7d6 e5f3 f6e4 d2d4 d6d5 f1d3 e4d6 d1d3 d7d5",
    "e2e4 e7e5 g1f3 g8f6 d2d4 f6e4 d4e5 d7d5 f1d3 e4d6 e5d6 d8d6 e1g1",
    # Philidor
    "e2e4 e7e5 g1f3 d7d6 d2d4 e5d4 f3d4 g8f6 b1c3 f8e7 f1e2 e8g8 e1g1 b8c6",
    "e2e4 e7e5 g1f3 d7d6 d2d4 e5d4 f3d4 g8f6 b1c3 a7a6 f1e2 c7c5 d4b5 d6d5",
    # King's Gambit
    "e2e4 e7e5 f2f4 e5f4 g1f3 g7g5 f1c4 g5g4 e1g1 g4f3 d1f3 f8g7 f3f7 e8f7 c4f7 d8d1",
    "e2e4 e7e5 f2f4 e5f4 g1f3 g7g5 f1c4 g5g4 e1g1 g4f3 d1f3 d7d6 f3f7 e8f7",
    "e2e4 e7e5 f2f4 d7d5 e4d5 g8f6 b1c3 f6d5 g1f3 f8e7 d5c3 b2c3",
    "e2e4 e7e5 f2f4 e5f4 f1c4 d8h4 e1g1 f8c5 d1e1 h4h5 c4f7 e8f7",
    "e2e4 e7e5 f2f4 d5e4 g1f3 f8d6 d2d4 e4e3 f1e2 e3e2 d1e2",
    # Vienna
    "e2e4 e7e5 b1c3 g8f6 f2f4 d7d5 f4e5 f6e4 d1f3 e4f6 c3d5 f8e7 f1d3",
    "e2e4 e7e5 b1c3 g8f6 f1c4 f6e4 c3e4 d7d5 e4g3 d5c4 d1d8 e8d8 g3c7",
    "e2e4 e7e5 b1c3 b8c6 f2f4 e5f4 g1f3 g8f6 d1d4 f4f3 d4d8 f8d8",
    # Bishop's Opening
    "e2e4 e7e5 f1c4 g8f6 d2d3 c7c6 b1c3 f8c5 f2f4 d7d5 e4d5 c6d5 c3d5 f6d5",
    "e2e4 e7e5 f1c4 g8f6 g1f3 f6e4 d2d3 e4d6 c4b5 d6b4 c2c3 b4a5 d1a4",
    # Ponziani
    "e2e4 e7e5 g1f3 b8c6 c2c3 g8f6 d2d4 e5d4 c3d4 f8c5 b1c3 f6e4 f1d3",
    "e2e4 e7e5 g1f3 b8c6 c2c3 g8f6 d2d4 e5d4 c3d4 d7d5 e4d5 f6d5 b1f3",
    # Center Game / Danish
    "e2e4 e7e5 d2d4 e5d4 d1d4 d8d4 e4e5 b8c6 d1d1 d7d5 e5d6 c8g4",
    "e2e4 e7e5 d2d4 e5d4 c1g5 d8g5 d1d4 g5g4 d4g4 h7h5 g4g7 h5h4 g7h4 c8e6",
    "e2e4 e7e5 d2d4 e5d4 c1g5 d8g5 e4e5 c6a5 b1a3 d7d5 e5d6 g5g6 h2h4",
    # Latvian / King's Gambit Declined
    "e2e4 e7e5 f2f4 f7f5 e4f5 d7d5 g1f3 d5d4 f3e5 d4e2 f1e2",
    "e2e4 e7e5 f2f4 c7c6 g1f3 d7d5 e4d5 c6d5 f4e5 f8c5",

    # ===== Sicilian Defense =====
    # Najdorf
    "e2e4 c7c5 g1f3 d7d6 d2d4 c5d4 f3d4 g8f6 b1c3 a7a6 f1e2 e7e5 d4b3 f8e7 e1g1 e8g8 f2f4 b8c6",
    "e2e4 c7c5 g1f3 d7d6 d2d4 c5d4 f3d4 g8f6 b1c3 a7a6 f1e2 e7e5 d4b3 f8e7 e1g1 f8e8 f2f4 b7b5 f1d3",
    "e2e4 c7c5 g1f3 d7d6 d2d4 c5d4 f3d4 g8f6 b1c3 a7a6 f1g5 e7e6 f2f4 f8e7 g5f6 g7f6 e1g1 h7h5",
    "e2e4 c7c5 g1f3 d7d6 d2d4 c5d4 f3d4 g8f6 b1c3 a7a6 f3e2 d6d5 e4d5 f6d5 c3d5 e6d5 f1d3",
    "e2e4 c7c5 g1f3 d7d6 d2d4 c5d4 f3d4 g8f6 b1c3 a7a6 f1c4 e7e6 a2a4 f8e7 e1g1 e8g8",
    # Classical Sicilian
    "e2e4 c7c5 g1f3 d7d6 d2d4 c5d4 f3d4 g8f6 b1c3 b8c6 f1e2 e7e5 d4b3 a7a6 f1g5 f8e7 e1g1",
    "e2e4 c7c5 g1f3 d7d6 d2d4 c5d4 f3d4 g8f6 b1c3 b8c6 f1c4 e7e6 b3a4 d7d5",
    # Dragon
    "e2e4 c7c5 g1f3 d7d6 d2d4 c5d4 f3d4 g8f6 b1c3 g7g6 f1e3 f8g7 f2f3 b8c6 d1d2 e8g8 e1g1 e8c7",
    "e2e4 c7c5 g1f3 d7d6 d2d4 c5d4 f3d4 g8f6 b1c3 g7g6 f1e3 f8g7 f2f3 d7d5 e4d5 f6d5 c3d5 g7a1",
    "e2e4 c7c5 g1f3 d7d6 d2d4 c5d4 f3d4 g8f6 b1c3 g7g6 c2c3 f8g7 f1e2 e7e6 e1g1 b8c6",
    # Scheveningen
    "e2e4 c7c5 g1f3 e7e6 d2d4 c5d4 f3d4 g8f6 b1c3 d7d6 f1e2 a7a6 e1g1 e8g8 f2f4 f8e7",
    "e2e4 c7c5 g1f3 e7e6 d2d4 c5d4 f3d4 g8f6 b1c3 d7d6 f1e3 f8e7 c2c3 e8g8 d4b3",
    # Kan
    "e2e4 c7c5 g1f3 e7e6 d2d4 c5d4 f3d4 a7a6 f1d3 g8f6 e1g1 d7d6 c2c3 b8d7",
    "e2e4 c7c5 g1f3 e7e6 d2d4 c5d4 f3d4 a7a6 f1c4 g8f6 d1f3 d7d5 e4d5 f6d5 c4d5 e6d5",
    # Taimanov
    "e2e4 c7c5 g1f3 e7e6 d2d4 c5d4 f3d4 b8c6 b1c3 g8f6 f1e3 f8b4 d4c6 b7c6",
    "e2e4 c7c5 g1f3 e7e6 d2d4 c5d4 f3d4 b8c6 b1c3 a7a6 f1e2 e7e5 d4b5",
    # Sveshnikov
    "e2e4 c7c5 g1f3 b8c6 d2d4 c5d4 f3d4 g8f6 b1c3 e7e5 d4b5 d7d6 g1e2 e5e4 c3d5 f6d5",
    "e2e4 c7c5 g1f3 b8c6 d2d4 c5d4 f3d4 g8f6 b1c3 e7e5 d4b5 d7d6 f1e2 a7a6 d5b6 c8b7 e2g3",
    "e2e4 c7c5 g1f3 b8c6 d2d4 c5d4 f3d4 g8f6 b1c3 e7e5 d4b5 d7d6 a2a4 a7a6 b5a3 f8e7 f1e2",
    # Accelerated Dragon
    "e2e4 c7c5 g1f3 b8c6 d2d4 c5d4 f3d4 g7g6 c2c4 f8g7 b1c3 g8f6 f1e3 f7f6 f3e2",
    "e2e4 c7c5 g1f3 b8c6 d2d4 c5d4 f3d4 g7g6 b1c3 f8g7 f1e3 d7d6 f2f3 e8g8 d1d2 a7a6",
    # Alapin (c3 Sicilian)
    "e2e4 c7c5 c2c3 d7d5 e4d5 d8d5 d2d4 g8f6 g1f3 f8g4 f1e2 e7e6 e1g1 e8g8",
    "e2e4 c7c5 c2c3 d7d5 e4d5 d8d5 d2d4 g8f6 g1f3 c5d4 c3d4 b8c6 b1c3 d5a5 f1c4",
    "e2e4 c7c5 c2c3 g8f6 e4e5 f6d5 d2d4 c5d4 c3d4 d7d6 f1d3 d6d5",
    # Closed Sicilian / Grand Prix
    "e2e4 c7c5 b1c3 b8c6 g2g3 g7g6 f1g2 f8g7 d2d3 d7d6 c1e3 f7f6 g1e2 e8g8 d1d2 a7a6",
    "e2e4 c7c5 b1c3 b8c6 f2f4 e7e6 g1f3 d7d5 e4d5 e6d5 f1b5 f8d6 f3e5",
    "e2e4 c7c5 b1c3 e7e6 g2g3 d7d5 e4e5 d5d4 c3d5 e6d5 d1d4 c8d7",
    # Rossolimo
    "e2e4 c7c5 g1f3 b8c6 f1b5 g7g6 c2c3 f8g7 d2d4 c5d4 c3d4 g8f6 b5c6 b7c6 d4d5",
    "e2e4 c7c5 g1f3 b8c6 f1b5 e7e5 b5c6 d7c6 d2d3 g8f6 b1a3 f8e7 c2c3 e8g8",
    # O'Kelly / Nimzowitsch
    "e2e4 c7c5 g1f3 a7a6 e4e5 g7g6 d2d4 f8g7 c2c4 g8h6 b1c3 h6g4 f1e2",
    "e2e4 c7c5 b1c3 d7d6 f2f4 g8f6 g1f3 a7a6 f1d3 e7e5",

    # ===== French Defense =====
    # Winawer
    "e2e4 e7e6 d2d4 d7d5 b1c3 f8b4 e4e5 c7c5 a2a3 b4c3 b2c3 g8e7 d1g4 e7g6 g4g5 h7h5",
    "e2e4 e7e6 d2d4 d7d5 b1c3 f8b4 e4e5 c7c5 a2a3 b4f8 d1g4 g8e7 g4g5 c8e6",
    # Classical / Tarrasch
    "e2e4 e7e6 d2d4 d7d5 b1c3 g8f6 f1g5 f8e7 e4e5 f6d7 g5e7 d7e7 f1d3 c7c5 c2c3",
    "e2e4 e7e6 d2d4 d7d5 b1c3 g8f6 f1g5 f8e7 e4e5 f6d7 g5e7 d7e7 f1d3 c7c5 c2c3 c5c4 d3e2 b8c6 g1e2",
    # Advance
    "e2e4 e7e6 d2d4 d7d5 e4e5 c7c5 c2c3 b8c6 g1f3 d8b6 a2a3 c5c4 b1a2 f8e7",
    "e2e4 e7e6 d2d4 d7d5 e4e5 c7c5 c2c3 b8c6 g1f3 d8b6 f1d3 c5c4 d3c2",
    # Exchange
    "e2e4 e7e6 d2d4 d7d5 e4d5 e6d5 f1d3 g8f6 g1f3 f8d6 e1g1 e8g8 c2c3 c7c6",
    "e2e4 e7e6 d2d4 d7d5 e4d5 e6d5 b1c3 g8f6 f1g5 f8e7 e1g1 e8g8 f1e2 h7h6",
    # Tarrasch
    "e2e4 e7e6 d2d4 d7d5 b1d2 c7c5 e4d5 d8d5 g1f3 f8e7 c2c4 d5d6 f1e2",
    "e2e4 e7e6 d2d4 d7d5 b1d2 g8f6 e4e5 f6d7 c2c3 c7c5 f1e2 b8c6 g1f3 f8e7",
    "e2e4 e7e6 d2d4 d7d5 b1d2 c7c5 g1f3 b8c6 c2c3 f8e7 f1e2 g8e7 e1g1",
    # Steinitz
    "e2e4 e7e6 d2d4 d7d5 b1d2 g8f6 e4e5 f6d7 c2c3 c7c5 f1e2 b8c6 g1f3 f8e7",

    # ===== Caro-Kann =====
    # Classical
    "e2e4 c7c6 d2d4 d7d5 b1c3 d5e4 c3e4 f8f5 e4g3 f5g6 h2h4 h7h6 f1e2 e7e6 g1f3 f8e7 e1g1",
    "e2e4 c7c6 d2d4 d7d5 b1c3 d5e4 c3e4 g8f6 e4g3 f8f5 f1g2 f5e6 g3e4 e6c8",
    "e2e4 c7c6 d2d4 d7d5 b1c3 d5e4 c3e4 f8f5 e4g3 f5g6 f1g2 e7e6 g1f3 f8e7",
    # Advance
    "e2e4 c7c6 d2d4 d7d5 e4e5 f7f6 f1d3 f6f5 c2c4 b8c6 g1f3 f5f4 b1c3 f8e7",
    "e2e4 c7c6 d2d4 d7d5 e4e5 f7f6 f1d3 b8c6 c2c3 f6f5 e5f6 e7f6 d3f5",
    "e2e4 c7c6 d2d4 d7d5 e4e5 f7f6 f1d3 f6f5 c2c4 b8c6 b1c3 g8e7 g1f3 e8g7",
    # Exchange / Panov
    "e2e4 c7c6 d2d4 d7d5 e4d5 c6d5 c2c4 g8f6 b1c3 e7e6 c4d5 e6d5 f1d3 f8e7 g1f3 e8g8 e1g1",
    "e2e4 c7c6 d2d4 d7d5 e4d5 c6d5 c4c5 g8f6 b1c3 e7e6 g1f3 f8e7 f1g5 e8g8 e1g1",
    "e2e4 c7c6 d2d4 d7d5 b1d2 d5e4 d2e4 f8f5 e4g3 f5g6 h2h4 h7h6",
    # Two Knights
    "e2e4 c7c6 d2d4 d7d5 b1c3 d5e4 c3e4 g8f6 e4f6 d8f6 d1d8 e8d8 f1c4 f8e7",
    "e2e4 c7c6 d2d4 d7d5 e4d5 c6d5 c2c4 d5c4 g1f3 e7e6 f1c4 b8c6 e1g1 g8f6",

    # ===== Pirc / Modern =====
    "e2e4 d7d6 d2d4 g8f6 b1c3 g7g6 f1e3 f8g7 f2f3 e7e6 d1d2 d7d5 e4d5 e6d5 e1g1 b8c6",
    "e2e4 d7d6 d2d4 g8f6 b1c3 g7g6 g1f3 f8g7 h2h3 e7e6 f1e2 e8g8 e1g1 b8c6 d1c2 a7a6",
    "e2e4 d7d6 d2d4 g8f6 b1c3 g7g6 f2f4 f8g7 g1f3 e7e6 e1g1 d7d5 e4d5 e6d5",
    "e2e4 g7g6 d2d4 f8g7 b1c3 d7d6 f1e3 a7a6 g1f3 b8c6 d1d2 e7e5",
    "e2e4 g7g6 d2d4 f8g7 b1c3 d7d6 f1e3 a7a6 h2h4 h7h5 f3f2 b8c6",
    "e2e4 g7g6 c2c4 f8g7 b1c3 d7d6 g1f3 b8c6 d2d4 e7e6 f1e2",

    # ===== Alekhine =====
    "e2e4 g8f6 e4e5 f6d5 d2d4 d7d6 g1f3 f8g5 h2h4 g5f6 c2c4 c7c6 c4d5 c6d5",
    "e2e4 g8f6 e4e5 f6d5 d2d4 d7d6 c2c4 d5b6 f1d3 c7c6 e1g1 e7e5 f3e5",
    "e2e4 g8f6 e4e5 f6d5 d2d4 d7d6 g1f3 f8g4 f1e2 e7e6 e1g1 f8e7 f3e5",
    "e2e4 g8f6 e4e5 f6d5 c2c4 d5b6 d2d4 d7d6 b1c3 e7e6 f1d3 c7c5",

    # ===== Scandinavian =====
    "e2e4 d7d5 e4d5 d8d5 b1c3 d5a5 d2d4 g8f6 g1f3 f8f5 f1c4 e7e6 e1g1 e8g8",
    "e2e4 d7d5 e4d5 d8d5 b1c3 d5d8 d2d4 g8f6 g1f3 f8g4 c2c3 e7e6 f1e2 d8d6",
    "e2e4 d7d5 e4d5 g8f6 d2d4 f6d5 g1f3 f8g4 h2h3 g4h5 c2c4 d5b6 f1e2 e7e6",
    "e2e4 d7d5 e4d5 g8f6 c2c4 f6d5 d2d4 d5b4 c1d2 b4c3 d2c3 g7g6 e1g1 f8g7",

    # ===== d4 d5 (Closed Games) =====
    # Queen's Gambit Declined - Main Line
    "d2d4 d7d5 c2c4 e7e6 b1c3 g8f6 g1f3 f8e7 f1g5 e1g1 e2e3 h7h6 f3e2 e8g8 c4d5 e6d5 f1d3 b7b6",
    "d2d4 d7d5 c2c4 e7e6 b1c3 g8f6 g1f3 f8e7 f1g5 e1g1 e2e3 e8g8 d1c2 h7h6 f3e2 b7b5",
    "d2d4 d7d5 c2c4 e7e6 b1c3 g8f6 g1f3 f8e7 f1g5 e1g1 e2e3 e8g8 d1c2 h7h6 f1h4 f6d7",
    # QGD Exchange
    "d2d4 d7d5 c2c4 e7e6 b1c3 g8f6 c4d5 e6d5 f1g5 c7c6 e2e3 f8e7 g1f3 e8g8 e1g1 b8d7 f1d3",
    "d2d4 d7d5 c2c4 e7e6 b1c3 g8f6 c4d5 e6d5 f1g5 c7c6 e2e3 h7h6 f1h4 f8e7 g1f3 e8g8 e1g1 b8d7",
    # Semi-Tarrasch
    "d2d4 d7d5 c2c4 e7e6 b1c3 g8f6 c4d5 e6d5 g1f3 c7c5 c2c3 b8c6 f1g5 f8e7 e2e3",
    # Queen's Gambit Accepted
    "d2d4 d7d5 c2c4 d5c4 g1f3 a7a6 e2e3 b7b5 a2a4 c7c6 b1c3 b5b4 f1c4 e7e6 e1g1 f8b7",
    "d2d4 d7d5 c2c4 d5c4 g1f3 a7a6 e2e3 f8g4 f1c4 e7e6 e1g1 g8f6 d1e1 b7b5 c4d3 c8d7",
    "d2d4 d7d5 c2c4 d5c4 e2e3 e7e5 f1c4 e5d4 e3d4 f8c5 d1f3",
    "d2d4 d7d5 c2c4 d5c4 g1f3 g8f6 e2e3 e7e6 a2a4 c7c5 b1c3 c5d4 e3d4 b8c6 f1c4 f8d6",
    # Slav
    "d2d4 d7d5 c2c4 c7c6 g1f3 g8f6 b1c3 d5c4 a2a4 f8f5 e2e3 e7e6 f1c4 b8d7 e1g1 f8e7",
    "d2d4 d7d5 c2c4 c7c6 g1f3 g8f6 b1c3 a7a6 c4d5 c6d5 f1g5 f8g7 e2e3 e7e6 f1d3",
    "d2d4 d7d5 c2c4 c7c6 g1f3 g8f6 e2e3 e7e6 f1d3 d5c4 d3c4 b7b5 c4d3 a7a5",
    "d2d4 d7d5 c2c4 c7c6 g1f3 g8f6 c4d5 c6d5 b1c3 b8c6 g1e5 f8d6 f1g5 e7e6",
    "d2d4 d7d5 c2c4 c7c6 g1f3 g8f6 d1c2 f8g4 f1g5 d7d6 c4d5 c6d5 e2e3 e7e6",
    # Semi-Slav / Meran / Botvinnik
    "d2d4 d7d5 c2c4 c7c6 g1f3 g8f6 b1c3 e7e6 e2e3 b8d7 f1d3 d5c4 d3c4 f8d6 e1g1 e8g8",
    "d2d4 d7d5 c2c4 c7c6 g1f3 g8f6 b1c3 e7e6 e2e3 b8d7 f1d3 d5c4 d3c4 f8d6 e1g1 e8g8 e3e4 e6e5 b3b2 d6e7 f1e2 d7b6",
    "d2d4 d7d5 c2c4 c7c6 g1f3 g8f6 b1c3 e7e6 f1g5 d5c4 e3e4 b7b5 a2a4 c8b7 e4e5 f6h5 f1e2 h5g3",
    "d2d4 d7d5 c2c4 c7c6 g1f3 g8f6 e2e3 e7e6 b1d2 b8d7 f1d3 d5c4 d3c4 f8d6 e1g1 e8g8",
    # Chigorin / Albin
    "d2d4 d7d5 c2c4 b8c6 b1c3 e7e6 g1f3 f8b4 e2e3 g8f6 f1d3",
    "d2d4 d7d5 c2c4 e7e5 d4e5 d7d6 e5d6 c8g4 g1f3 g4f3 e1f3 d8d6 f3g1 e8e7",
    # Catalan
    "d2d4 d7d5 c2c4 e7e6 g1f3 g8f6 g2g3 f8e7 f1g2 e1g1 e8g8 e1g1 d5c4 d1c2 a7a6 c2c4 b7b5",
    "d2d4 d7d5 c2c4 e7e6 g1f3 g8f6 g2g3 d5c4 f1g2 c8e6 e1g1 e8g8 b1d2 f8d5",
    "d2d4 g8f6 c2c4 e7e6 g2g3 d7d5 f1g2 f8e7 g1f3 e1g1 e8g8 e1g1 c7c6 d1c2 a7a6",
    "d2d4 g8f6 c2c4 e7e6 g2g3 d7d5 f1g2 f8e7 g1f3 e1g1 e8g8 e1g1 d5c4 d1c2 b7b5",

    # ===== d4 Nf6 (Indian Defenses) =====
    # Nimzo-Indian
    "d2d4 g8f6 c2c4 e7e6 b1c3 f8b4 e2e3 b7b6 f1d3 f8b7 g1f3 f8e7 e1g1 e8g8 e3e4 d7d5",
    "d2d4 g8f6 c2c4 e7e6 b1c3 f8b4 g1f3 b8c6 e1g1 f8b4 f1e2 e1g1 a2a3 f8e7",
    "d2d4 g8f6 c2c4 e7e6 b1c3 f8b4 d1c2 d7d5 a2a3 f8e7 f8c6 c4d5 e6d5 g1f3 e1g1",
    "d2d4 g8f6 c2c4 e7e6 b1c3 f8b4 f2f3 d7d5 a1c1 f8c6 c4d5 e6d5 f1d3 b8a5 e1g1",
    "d2d4 g8f6 c2c4 e7e6 b1c3 f8b4 e2e3 c7c5 f1d3 b8c6 g1f3 e7e5 e1g1 e8g8",
    "d2d4 g8f6 c2c4 e7e6 b1c3 f8b4 d1c2 d7d5 a2a3 f8c6 b1a4 b7b6 c4d5 c6d5",
    # Queen's Indian
    "d2d4 g8f6 c2c4 e7e6 g1f3 b7b6 g2g3 f8a6 b2b3 f8b4 f1b2 e7e5 d4e5 f6e5 f3e5 b6e5",
    "d2d4 g8f6 c2c4 e7e6 g1f3 b7b6 a2a3 f8a6 d1a4 c7c5 a4a6 c8a6 c4c5 b6c5 d4c5 f8c5",
    "d2d4 g8f6 c2c4 e7e6 g1f3 b7b6 f1g5 h7h6 g5f6 d8f6 b1c3 f8b7 e2e3 d7d5 c4d5 e6d5",
    "d2d4 g8f6 c2c4 e7e6 g1f3 b7b6 b1c3 f8b4 f1g5 h7h6 g5h4 g7g5 h4g3 f8e7",
    # Bogo-Indian
    "d2d4 g8f6 c2c4 e7e6 g1f3 f8b4 f1d2 d8e7 a2a3 f8e7 g2g3 d7d5 f1g2 d5c4 e1g1",
    "d2d4 g8f6 c2c4 e7e6 g1f3 f8b4 f1d2 b4e7 a2a3 d7d5 c4d5 e6d5 f1g5 c7c6 e2e3 b8d7",
    # King's Indian
    "d2d4 g8f6 c2c4 g7g6 b1c3 f8g7 e2e4 d7d6 g1f3 e1g1 e1g1 b8c6 f1e2 a7a6 e1g1 d7d5",
    "d2d4 g8f6 c2c4 g7g6 b1c3 f8g7 e2e4 d7d6 f1e2 e7e5 d4e5 d6e5 d1d8 f8d8 g1f3 b8c6",
    "d2d4 g8f6 c2c4 g7g6 b1c3 f8g7 e2e4 d7d6 f1e2 e7e5 d4d5 b8c6 g1g2 a7a5 f3h4 g8h6",
    "d2d4 g8f6 c2c4 g7g6 g1f3 f8g7 b1c3 e7e6 e2e4 e1g1 d7d5 c4d5 e6d5 e4d5 g8d7 f1d3",
    "d2d4 g8f6 c2c4 g7g6 b1c3 f8g7 e2e4 d7d6 g1f3 e1g1 e1g1 b8a6 c3d5 b8d7 f1e2 c7c5",
    "d2d4 g8f6 c2c4 g7g6 b1c3 f8g7 e2e4 d7d6 g1f3 e1g1 e1g1 c7c5 c1e3 c8g4 d1d2 b8c6 d4c5 d6c5",
    # Grünfeld
    "d2d4 g8f6 c2c4 g7g6 b1c3 d7d5 g1f3 f8g7 d1b3 d5c4 b3c4 e7e6 b3b3 e1g1 e8g8 f3e5 b8d7",
    "d2d4 g8f6 c2c4 g7g6 b1c3 d7d5 c4d5 f6d5 e2e4 d5c3 b2c3 f8g7 f1c4 c7c6 g1f3 b8d7 e1g1",
    "d2d4 g8f6 c2c4 g7g6 b1c3 d7d5 c4d5 f6d5 e2e4 d5c3 b2c3 f8g7 c1g5 c8g4 f1e2 e8g8 g1f3",
    "d2d4 g8f6 c2c4 g7g6 g1f3 f8g7 c2c4 e1g1 e8g8 b1c3 d7d5 c4d5 f6d5 c3d5 g7a1 d5d3",
    "d2d4 g8f6 c2c4 g7g6 b1c3 d7d5 f1g5 f6e4 c3e4 d5e4 g5e7 f8g7 g1f3 e7e6 e1g1",
    # Budapest
    "d2d4 g8f6 c2c4 e7e5 d4e5 f6g4 f2f4 b8c6 g1f3 g4h6 f1g2 f8b4 d1d3 d7d5",
    "d2d4 g8f6 c2c4 e7e5 d4e5 f6g4 b1c3 g4e5 e2e3 d8h4 g1h3 f8b4 f1e2",
    # Benoni
    "d2d4 g8f6 c2c4 c7c5 d4d5 e7e6 b1c3 e6d5 c4d5 d7d6 e2e4 g7g6 f1e2 f8g7 g1f3 e1g1",
    "d2d4 g8f6 c2c4 c7c5 d4d5 e7e6 e2e4 e6d5 c4d5 d7d6 f1d3 g7g6 g1f3 f8g7 e1g1",
    "d2d4 g8f6 c2c4 e7e6 g1f3 c7c5 d4d5 e6d5 c4d5 d7d6 b1c3 g7g6 e2e4 f8g7 f1e2 e8g8",
    # Benko Gambit
    "d2d4 g8f6 c2c4 c7c5 d4d5 b7b5 c4b5 a7a6 b5a6 f8a6 b1c3 d7d6 e2e4 g7g6 f1g2 f8g7 g1f3 e8g8",
    "d2d4 g8f6 c2c4 c7c5 d4d5 b7b5 c4b5 a7a6 b5c4 g7g6 b1c3 f8g7 e2e4 d7d6 g1f3 e8g8",
    "d2d4 g8f6 c2c4 c7c5 d4d5 b7b5 a2a4 b5b4 g1f3 g7g6 g5d2 d7d6 c1h6",

    # ===== Dutch Defense =====
    "d2d4 f7f5 c2c4 g8f6 g2g3 e7e6 f1g2 f8e7 g1f3 e1g1 e8g8 e1g1 d7d5 b1c3 c7c6",
    "d2d4 f7f5 c2c4 g8f6 g2g3 d7d5 f1g2 c7c6 e1g1 e7e6 b1c3 d5c4 d1c2 b8d7",
    "d2d4 f7f5 c2c4 g8f6 b1c3 d7d5 e2e3 c7c6 f1d3 f8e7 g1f3 e8g8 e1g1",
    "d2d4 f7f5 g1f3 g8f6 f1g5 g7g6 e2e3 f8g7 h2h4 h7h6 g5f6 e7f6",
    "d2d4 f7f5 e2e4 d7d5 e4d5 d8d5 c2c4 d5d8 b1c3 g8f6 g1f3 e7e6",
    # Leningrad Dutch
    "d2d4 f7f5 c2c4 g8f6 g2g3 g7g6 f1g2 f8g7 g1f3 e1g1 d7d6 e1g1 e8g8",
    "d2d4 f7f5 g1f3 g8f6 g2g3 g7g6 f1g2 f8g7 e1g1 e8g8 c2c4 d7d6 b1c3 e7e5",

    # ===== Queen's Pawn / System Openings =====
    # London System
    "d2d4 d7d5 f1f4 c7c6 e2e3 f8f5 g1f3 e7e6 f1e2 g8f6 e1g1 e8g8 h2h3 b8d7 c2c3",
    "d2d4 d7d5 f1f4 g8f6 e2e3 e7e6 g1f3 f8d6 f4g3 d6g3 h2g3 c7c5 c2c3 b8c6 f1d3",
    "d2d4 g8f6 f1f4 d7d5 e2e3 e7e6 g1f3 c7c5 c2c3 b8c6 f1d3 f8d6",
    "d2d4 g8f6 f1f4 g7g6 e2e3 f8g7 g1f3 e1g1 d7d6 c2c3 e8g8 f1e2 b8c6",
    # Colle System
    "d2d4 d7d5 e2e3 g8f6 f1d3 c7c5 c2c3 b8c6 g1f3 e7e6 e1g1 f8d6 f3e5",
    "d2d4 d7d5 e2e3 g8f6 f1d3 e7e6 c2c3 b8d7 g1f3 f8d7 b1d2 e8g8 e1g1",
    # Trompowsky
    "d2d4 g8f6 f1g5 e7e6 e2e3 f8e7 b1d2 d7d5 g1f3 h7h6 g5h4 e8g8 c2c3 b8d7 f1d3",
    "d2d4 g8f6 f1g5 c7c5 d4d5 d7d6 f1d2 e7e5 g1f3 f8e7 e2e3 e8g8",
    "d2d4 g8f6 f1g5 f6e4 h2h4 d7d5 h4h5 c7c6 f1f4 d8a5 d1d3",
    # Torre Attack
    "d2d4 g8f6 g1f3 e7e6 f1g5 d7d5 e2e3 f8e7 b1d2 e8g8 c2c3 b8d7 f1d3 c7c5",
    # Veresov / Richtor
    "d2d4 d7d5 b1c3 g8f6 f1g5 b8d7 g1f3 c7c6 e2e3 e7e6 f1d3 f8e7 e1g1",
    "d2d4 d7d5 b1c3 g8f6 f1g5 b8d7 e2e3 e7e6 g1f3 f8e7 f1d3 e8g8 e1g1 c7c6",

    # ===== English Opening =====
    "c2c4 e7e5 g1f3 g8f6 g2g3 d7d5 c4d5 f6d5 f1g2 d5b6 b1c3 b8c6 d2d4 e5d4 f3d4 f8e7",
    "c2c4 e7e5 g1f3 b8c6 g2g3 f7f5 f1g2 g8f6 g1f3 d7d5 c4d5 f6d5 e1g1 e8g7",
    "c2c4 e7e5 g1f3 g8f6 b1c3 d7d5 c4d5 f6d5 g2g3 d5b6 f1g2 b8c6 e1g1 e7e5",
    "c2c4 g8f6 g1f3 e7e6 g2g3 d7d5 f1g2 f8e7 e1g1 e8g8 b1c3 d5c4 d1c2 a7a6 c2c4 b7b5",
    "c2c4 c7c5 g1f3 g8f6 d2d4 c5d4 f3d4 e7e6 g2g3 f8b4 d4c6 b7c6 f1g2 e8g8 e1g1 d7d5",
    "c2c4 c7c5 b1c3 g8f6 g2g3 d7d5 c4d5 f6d5 f1g2 d5b6 e1g1 e7e6 g1f3 b8c6 d2d3 f8e7",
    "c2c4 e7e6 g1f3 d7d5 d2d4 c7c6 e2e3 g8f6 b1d2 b8d7 f1d3 f8d6 e1g1",
    "c2c4 e7e5 b1c3 g8f6 g1f3 b8c6 e2e3 f8b4 d2d3 e7e6 f1e2 d7d5 c4d5 f6d5",
    "c2c4 g8f6 b1c3 d7d5 c4d5 f6d5 g2g3 g7g6 f1g2 d5b6 e1g1 f8g7 g1f3 e8g8",
    "c2c4 c7c6 e2e4 d7d5 e4d5 c6d5 d2d4 g8f6 b1c3 e7e6 g1f3 f8e7 c1g5 e8g8",
    "c2c4 e7e5 g2g3 g8f6 f1g2 d7d5 c4d5 f6d5 b1c3 d5b6 g1f3 b8c6 e1g1 e8g8",
    "c2c4 c7c5 g1f3 b8c6 b1c3 g7g6 g2g3 f8g7 f1g2 e7e6 e1g1 g8ge6",
    "c2c4 b7b6 g1f3 g7g6 d2d4 f8g7 b1c3 b8c6 f1e2 e7e6 e1g1 g8f6 d1c2 f8b7",

    # ===== Reti / Zukertort / KIA =====
    "g1f3 d7d5 c2c4 d5c4 g2g3 b7b5 f1g2 f8b7 e1g1 e7e6 g1f3 g8f6",
    "g1f3 d7d5 c2c4 d5d4 e2e3 b8c6 e3d4 c6d4 d1d4 e7e5 d4e3 f8b4",
    "g1f3 g8f6 c2c4 e7e6 g2g3 d7d5 f1g2 f8e7 e1g1 e8g8 b1c3 d5c4 d1c2 a7a6 c2c4 b7b5",
    "g1f3 g8f6 d2d4 e7e6 c2c4 f8b4 c1d2 d8e7 d1c2 b4d2 c2d2 d7d5",
    "g1f3 c7c5 c2c4 b8c6 d2d4 c5d4 f3d4 g8f6 b1c3 e7e6 e2e3 f8b4",
    "g1f3 g8f6 g2g3 b7b6 f1g2 f8b7 e1g1 e7e6 c2c4 f8e7 g1f3 d7d6",
    "g1f3 d7d5 g2g3 g8f6 f1g2 c7c6 e1g1 f8g4 d2d3 e7e6 e1g1 e8g8",
    "g1f3 c7c5 g2g3 b8c6 f1g2 g8f6 e1g1 e7e5 d2d3 f8e7",
    "g1f3 d7d5 b2b3 f8g4 f1b2 e7e6 e2e3 g8f6 b1d2 b8d7 f1d3",
    # King's Indian Attack
    "e2e4 e7e6 d2d3 d7d5 g1f3 d5e4 d3e4 g8f6 f1d3 c7c5 e1g1 g8e7",
    "e2e4 e7e6 d2d3 d7d5 b1d2 g8f6 g1e2 f8e7 c2c3 e8g8 f1e2 b8c6",
    "g1f3 d7d5 g2g3 g8f6 f1g2 c7c6 e1g1 f8g4 d2d3 e7e6 h2h3 g4h5",

    # ===== Bird / Other Flank =====
    "f2f4 d7d5 g1f3 g8f6 e2e3 e7e6 f1e2 f8e7 e1g1 e8g8 d2d3 b8c6 b1d2 e8g8",
    "f2f4 e7e5 f4e5 d7d6 e5d6 f8d6 g1f3 g8f6 f1c4 e8g8 e1g1",
    "f2f4 c7c5 g1f3 b8c6 f1c4 g8f6 d2d3 e7e6 e1g1 e8g8",
    "b1c3 d7d5 e2e4 d5e4 c3e4 f8e6 g1f3 g8f6 f1d3 c7c5 e1g1 e8g8",
    "b1c3 d7d5 d2d4 g8f6 f1g5 f6d5 e2e3 f8e7 f1d3 e7e6 g1f3 e8g8",
    "b1c3 g8f6 e2e4 d7d5 e4e5 f6d5 c1g4 d5b6 f1c4 e7e6 g1f3 c7c5",
    "g1f3 f7f5 d2d4 g8f6 c2c4 e7e6 g2g3 f8e7 f1g2 e8g8 e1g1 d7d6",
    "b2b3 d7d5 f1b2 g8f6 e2e3 e7e6 c2c4 c7c6 b1c3 d5c4 b3c4 b7b6",
    "b2b3 e7e5 f1b2 b8c6 e2e3 d7d5 f1e2 f8d6 c2c3 g8f6",
    "g2g3 d7d5 f1g2 c7c5 g1f3 b8c6 d2d3 g8f6 e1g1 e7e6",
    "g2g3 g7g6 f1g2 f8g7 g1f3 d7d6 e1g1 e8g8 d2d3 b8c6 c2c3 e7e5",
    "b1a3 d7d5 e2e4 d5e4 g1f3 e7e5 b3c4 b8c6 d1e2 f8d6",

    # ===== Deeper lines from critical positions =====
    # Sicilian Najdorf 6. Bg5 (deep)
    "e2e4 c7c5 g1f3 d7d6 d2d4 c5d4 f3d4 g8f6 b1c3 a7a6 f1g5 e7e6 f2f4 d8b6 d4b3 f8e7 e1g1 e8g8",
    "e2e4 c7c5 g1f3 d7d6 d2d4 c5d4 f3d4 g8f6 b1c3 a7a6 f1g5 e7e6 f2f4 b8bd7 e1g1 h7h6 g5h4 g7g5",
    # Sicilian English Attack
    "e2e4 c7c5 g1f3 d7d6 d2d4 c5d4 f3d4 g8f6 b1c3 a7a6 f3e2 e7e5 g2g3 f8e7 e1g1 e8g8",
    # Ruy Lopez Marshall
    "e2e4 e7e5 g1f3 b8c6 f1b5 a7a6 b5a4 g8f6 e1g1 f6e4 d2d4 d7d5 d4e5 f8e7 f1e1 e7f6 e5f6 d8f6",
    # King's Indian Bayonet Attack
    "d2d4 g8f6 c2c4 g7g6 b1c3 f8g7 e2e4 d7d6 g1f3 e1g1 e1g1 e8g8 h2h5 c7c6 e1g1 b7b5 c4b5 c6b5",
    # Grünfeld Exchange (deep)
    "d2d4 g8f6 c2c4 g7g6 b1c3 d7d5 c4d5 f6d5 e2e4 d5c3 b2c3 f8g7 f1c4 c7c6 g1f3 b8d7 e1g1 d7b6 c4b3",
    # Nimzo Rubinstein (deep)
    "d2d4 g8f6 c2c4 e7e6 b1c3 f8b4 e2e3 c7c5 g1f3 b8c6 f1d3 d7d5 c4d5 e6d5 e1g1 e8g8",
    "d2d4 g8f6 c2c4 e7e6 b1c3 f8b4 e2e3 b7b6 f1d3 f8b7 e1g1 f8e7 d1e1 d7d5",
    # QGD Tartakower (deep)
    "d2d4 d7d5 c2c4 e7e6 b1c3 g8f6 g1f3 f8e7 f1g5 e1g1 e2e3 e8g8 d1c2 h7h6 f3e2 b7b5 c4b5 e6b5",
    # Caro-Kann Tartakower
    "e2e4 c7c6 d2d4 d7d5 b1c3 d5e4 c3e4 f8f5 e4g3 f5g6 h2h4 h7h6 f1e2 e7e6 g1f3 f8e7 e1g1 e8g8",
    # Slav exchange (deep)
    "d2d4 d7d5 c2c4 c7c6 c4d5 c6d5 b1c3 g8f6 f1g5 e7e6 e2e3 f8e7 g1f3 e8g8 e1g1 b8d7 f1d3 f8e8",
    # Catalan Open Catalan (deep)
    "d2d4 g8f6 c2c4 e7e6 g2g3 d7d5 f1g2 f8e7 g1f3 e1g1 e8g8 c4d5 e6d5 e1g1 b8c6 c2c3 d7d6",
    # French Winawer Poisoned Pawn
    "e2e4 e7e6 d2d4 d7d5 b1c3 f8b4 e4e5 c7c5 a2a3 b4c3 b2c3 g8e7 d1g4 e7g6 g4g3 c5d4 c3d4 d8c7",
]


RUST_TEMPLATE = """// Auto-generated by scripts/generate_static_data.py
// DO NOT EDIT MANUALLY — run `./build.sh --regen-data` to regenerate.
use crate::{{Move, Piece, Position}};

pub const OPENING_BOOK: &[(u64, u16, u16, u16)] = &[
    {body}
];

/// Look up a position in the opening book. Returns a weighted-random move:
/// 50% best, 33% second, 17% third (falls back if fewer moves available).
pub fn lookup_opening(zobrist: u64, seed: u8) -> Option<Move> {{
    let idx = OPENING_BOOK.binary_search_by_key(&zobrist, |&(k, _, _, _)| k).ok()?;
    let (_, m1, m2, m3) = OPENING_BOOK[idx];
    if m1 == 0 {{
        return None;
    }}
    let r = seed % 100;
    let chosen = if r < 50 || m2 == 0 {{
        m1
    }} else if r < 83 || m3 == 0 {{
        m2
    }} else {{
        m3
    }};
    Some(decode_move(chosen))
}}

pub fn decode_move(code: u16) -> Move {{
    let special = (code >> 12) & 0xF;
    match special {{
        5 => Move::KingSideCastle,
        6 => Move::QueenSideCastle,
        _ => {{
            let fi = (code & 0x3F) as usize;
            let ti = ((code >> 6) & 0x3F) as usize;
            let f = Position::new((fi / 8) as i32, (fi % 8) as i32);
            let t = Position::new((ti / 8) as i32, (ti % 8) as i32);
            match special {{
                1 => Move::Promotion(f, t, Piece::Knight(crate::Color::White, t)),
                2 => Move::Promotion(f, t, Piece::Bishop(crate::Color::White, t)),
                3 => Move::Promotion(f, t, Piece::Rook(crate::Color::White, t)),
                4 => Move::Promotion(f, t, Piece::Queen(crate::Color::White, t)),
                _ => Move::Piece(f, t),
            }}
        }}
    }}
}}
"""


def main():
    ensure_stockfish()
    eng = chess.engine.SimpleEngine.popen_uci(str(ENGINE))
    eng.configure({"Threads": 4, "Hash": 512})
    entries = []       # list of (zobrist_key, move1, move2, move3)
    seen = set()       # set of zobrist keys already added
    boards = {}        # zobrist_key -> board position (for Phase 2 expansion)

    def add(b, depth=LINES_DEPTH, multipv=1):
        """Analyse position, store top `multipv` moves. Returns False if cap hit."""
        if len(entries) >= MAX_ENTRIES:
            return False
        k = board_hash(b)
        if k in seen:
            return True
        try:
            results = eng.analyse(b, chess.engine.Limit(depth=depth), multipv=multipv)
        except Exception:
            results = []
        if not isinstance(results, list):
            results = [results]
        moves = []
        for r in results:
            pv = r.get("pv")
            if pv:
                moves.append(encode_move_uci(pv[0]))
            if len(moves) >= 3:
                break
        if moves:
            while len(moves) < 3:
                moves.append(0)
            entries.append((k, moves[0], moves[1], moves[2]))
            seen.add(k)
            boards[k] = b.copy()
        return True

    t_total = time.time()

    # ── Phase 1: Walk all hand-curated opening lines ──
    print(f"Phase 1: Processing {len(LINES)} opening lines (depth {LINES_DEPTH}, multiPV {BOOK_MULTIPV})...")
    t0 = time.time()
    add(chess.Board(), multipv=BOOK_MULTIPV)
    for i, line in enumerate(LINES):
        if len(entries) >= MAX_ENTRIES:
            break
        b = chess.Board()
        for uci in line.split():
            try:
                m = chess.Move.from_uci(uci)
            except chess.InvalidMoveError:
                print(f"  WARNING: malformed move '{uci}' in line {i}: {line}")
                break
            if m not in b.legal_moves:
                print(f"  WARNING: illegal move {uci} in line {i}: {line}")
                break
            b.push(m)
            if not add(b, multipv=BOOK_MULTIPV):
                break
        if (i + 1) % 50 == 0:
            print(f"  [{i+1}/{len(LINES)}] {len(entries)} entries, {time.time()-t0:.0f}s")
    phase1_count = len(entries)
    print(f"  Done: {phase1_count} entries in {time.time()-t0:.0f}s")

    # ── Phase 2: BFS tree expansion — recursively cover opponent deviations ──
    # Starting from all Phase 1 positions, run Stockfish multiPV to find the top
    # N moves. For each move, follow it to the child position; if the child is
    # new, add it to the book AND queue it for further expansion. This builds
    # a tree that covers opponent deviations several plies deep.
    from collections import deque
    print(f"Phase 2: BFS tree expansion (multiPV={TREE_MULTIPV}, depth {TREE_DEPTH}, "
          f"max ply {MAX_EXPANSION_PLY})...")
    t0 = time.time()
    expanded = set()
    queue = deque(boards.keys())
    while queue and len(entries) < MAX_ENTRIES:
        key = queue.popleft()
        if key in expanded:
            continue
        expanded.add(key)
        b = boards[key]
        if b.ply() >= MAX_EXPANSION_PLY:
            continue
        try:
            results = eng.analyse(b, chess.engine.Limit(depth=TREE_DEPTH),
                                  multipv=TREE_MULTIPV)
        except Exception:
            continue
        if not isinstance(results, list):
            results = [results]
        for r in results:
            pv = r.get("pv")
            if not pv:
                continue
            move = pv[0]
            if move not in b.legal_moves:
                continue
            b.push(move)
            child_key = board_hash(b)
            if child_key not in seen:
                add(b, depth=TREE_DEPTH, multipv=BOOK_MULTIPV)
            if child_key not in expanded:
                queue.append(child_key)
            b.pop()
        if len(expanded) % 500 == 0:
            print(f"  Expanded {len(expanded)} positions, {len(entries)} entries, "
                  f"{time.time()-t0:.0f}s")
    print(f"  Done: {len(entries)} entries (+{len(entries)-phase1_count} from expansion) "
          f"in {time.time()-t0:.0f}s")

    eng.close()

    # ── Write output ──
    entries.sort(key=lambda x: x[0])
    body = ",\n    ".join(f"({k}u64, {m1}u16, {m2}u16, {m3}u16)" for k, m1, m2, m3 in entries)
    OUT.write_text(RUST_TEMPLATE.format(body=body))
    print(f"\nTotal: {len(entries)} entries, {time.time()-t_total:.0f}s")
    print(f"Wrote {OUT}")


if __name__ == "__main__":
    main()
