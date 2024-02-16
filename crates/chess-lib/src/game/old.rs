use crate::{GameId, Player, Wager};
use chess_engine::Board;
use near_sdk::borsh::{BorshDeserialize, BorshSerialize};

#[derive(BorshDeserialize, BorshSerialize)]
#[borsh(crate = "near_sdk::borsh")]
pub struct GameV3 {
    pub game_id: GameId,
    pub white: Player,
    pub black: Player,
    pub board: Board,
    pub wager: Wager,
    pub last_move_block_height: u64,
}
