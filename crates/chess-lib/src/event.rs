use crate::{Challenge, ChallengeId, GameId, GameOutcome, MoveStr, Player};
use chess_engine::Color;
use near_sdk::near_bindgen;

#[near_bindgen(event_json(standard = "chess-game"))]
#[derive(Debug)]
pub enum ChessEvent {
    #[event_version("1.0.0")]
    Challenge(Challenge),
    #[event_version("1.0.0")]
    AcceptChallenge {
        challenge_id: ChallengeId,
        game_id: GameId,
    },
    #[event_version("1.0.0")]
    RejectChallenge { challenge_id: ChallengeId },
    #[event_version("1.0.0")]
    CreateGame {
        game_id: GameId,
        white: Player,
        black: Player,
        board: [String; 8],
    },
    #[event_version("1.0.0")]
    PlayMove {
        game_id: GameId,
        color: Color,
        mv: MoveStr,
    },
    #[event_version("1.0.0")]
    ChangeBoard { game_id: GameId, board: [String; 8] },
    #[event_version("1.0.0")]
    FinishGame {
        game_id: GameId,
        outcome: GameOutcome,
        board: [String; 8],
    },
}
