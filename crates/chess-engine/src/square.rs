use super::Piece;
use near_sdk::borsh::{BorshDeserialize, BorshSerialize};

/// Essentially a container for a single piece on a board.
#[derive(
    Clone, Copy, Debug, Default, PartialEq, Eq, PartialOrd, Ord, BorshDeserialize, BorshSerialize,
)]
#[borsh(crate = "near_sdk::borsh")]
pub struct Square {
    piece: Option<Piece>,
}

/// A square containing no piece
pub const EMPTY_SQUARE: Square = Square { piece: None };

impl From<Piece> for Square {
    fn from(piece: Piece) -> Self {
        Self { piece: Some(piece) }
    }
}

impl Square {
    /// Does this square contain a piece?
    #[inline]
    pub fn is_empty(&self) -> bool {
        self.piece.is_none()
    }

    /// Get the piece contained in this square.
    #[inline]
    pub fn get_piece(&self) -> Option<Piece> {
        self.piece
    }
}
