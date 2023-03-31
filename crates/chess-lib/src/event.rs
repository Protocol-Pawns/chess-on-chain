use crate::{GameId, GameOutcome, MoveStr, Player};
use chess_engine::Color;
use near_sdk::near_bindgen;

#[near_bindgen(event_json(standard = "chess001"))]
#[derive(Debug)]
pub enum ChessEvent {
    #[event_version("1.0.0")]
    CreateGame {
        game_id: GameId,
        white: Player,
        black: Player,
    },
    #[event_version("1.0.0")]
    PlayMove {
        game_id: GameId,
        color: Color,
        mv: MoveStr,
    },
    #[event_version("1.0.0")]
    FinishGame {
        game_id: GameId,
        outcome: GameOutcome,
    },
}
