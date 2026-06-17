"""Zobrist key generation shared between Rust code generation and static data generation.

The piece-type ordering matches crates/chess-engine/src/piece.rs::zobrist_indices:
  0 = King, 1 = Queen, 2 = Rook, 3 = Bishop, 4 = Knight, 5 = Pawn
Color ordering: 0 = White, 1 = Black
"""

import random
import chess

SEED = 0x70726F746F636F6C207061776E73  # "protocol pawns"

PIECE_TYPE_INDEX = {
    chess.KING: 0,
    chess.QUEEN: 1,
    chess.ROOK: 2,
    chess.BISHOP: 3,
    chess.KNIGHT: 4,
    chess.PAWN: 5,
}
COLOR_INDEX = {chess.WHITE: 0, chess.BLACK: 1}


def _make_keys():
    rng = random.Random(SEED)
    piece_keys = [
        [[rng.getrandbits(64) for _ in range(64)] for _ in range(2)]
        for _ in range(6)
    ]
    black_to_move = rng.getrandbits(64)
    castling_keys = [rng.getrandbits(64) for _ in range(4)]
    en_passant_file_keys = [rng.getrandbits(64) for _ in range(8)]
    return piece_keys, black_to_move, castling_keys, en_passant_file_keys


PIECE_KEYS, BLACK_TO_MOVE_KEY, CASTLING_KEYS, EN_PASSANT_FILE_KEYS = _make_keys()


def board_hash(board: chess.Board, ignore_castling: bool = False, ignore_ep: bool = False) -> int:
    """Compute the same Zobrist key as the Rust Board::zobrist_key()."""
    key = 0
    for sq in chess.SQUARES:
        piece = board.piece_at(sq)
        if piece is not None:
            pt = PIECE_TYPE_INDEX[piece.piece_type]
            color = COLOR_INDEX[piece.color]
            key ^= PIECE_KEYS[pt][color][sq]
    if board.turn == chess.BLACK:
        key ^= BLACK_TO_MOVE_KEY
    if not ignore_castling:
        if board.has_kingside_castling_rights(chess.WHITE):
            key ^= CASTLING_KEYS[0]
        if board.has_queenside_castling_rights(chess.WHITE):
            key ^= CASTLING_KEYS[1]
        if board.has_kingside_castling_rights(chess.BLACK):
            key ^= CASTLING_KEYS[2]
        if board.has_queenside_castling_rights(chess.BLACK):
            key ^= CASTLING_KEYS[3]
    if not ignore_ep and board.ep_square is not None:
        key ^= EN_PASSANT_FILE_KEYS[chess.square_file(board.ep_square)]
    return key


def rust_key_arrays():
    """Return strings ready to embed in a Rust source file."""
    lines = [
        "pub const PIECE_ZOBRIST_KEYS: [[[u64; 64]; 2]; 6] = [",
    ]
    for pt in range(6):
        lines.append("    [")
        for c in range(2):
            squares = ", ".join(f"{PIECE_KEYS[pt][c][sq]}u64" for sq in range(64))
            lines.append(f"        [{squares}],")
        lines.append("    ],")
    lines.append("];")
    lines.append("")
    lines.append(f"pub const BLACK_TO_MOVE_ZOBRIST_KEY: u64 = {BLACK_TO_MOVE_KEY}u64;")
    lines.append("")
    castling = ", ".join(f"{k}u64" for k in CASTLING_KEYS)
    lines.append(f"pub const CASTLING_ZOBRIST_KEYS: [u64; 4] = [{castling}];")
    lines.append("")
    ep = ", ".join(f"{k}u64" for k in EN_PASSANT_FILE_KEYS)
    lines.append(f"pub const EN_PASSANT_FILE_ZOBRIST_KEYS: [u64; 8] = [{ep}];")
    return "\n".join(lines) + "\n"
