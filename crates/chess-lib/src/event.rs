use crate::{Challenge, ChallengeId, GameId, GameOutcome, MoveStr, Player};
use chess_engine::Color;
use near_sdk::{near_bindgen, AccountId};

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
        board: [String; 8],
        outcome: Option<GameOutcome>,
    },
    #[event_version("1.0.0")]
    ResignGame {
        game_id: GameId,
        resigner: Color,
        board: [String; 8],
        outcome: GameOutcome,
    },
    #[event_version("1.0.0")]
    CancelGame {
        game_id: GameId,
        cancelled_by: AccountId,
    },
}
