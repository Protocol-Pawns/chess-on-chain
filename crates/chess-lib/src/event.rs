use crate::{Challenge, ChallengeId, GameId, GameOutcome, MoveStr, Player};
use chess_engine::Color;
use near_sdk::{json_types::U128, near_bindgen, AccountId};

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
        outcome: GameOutcome,
    },
    #[event_version("1.0.0")]
    CancelGame {
        game_id: GameId,
        cancelled_by: AccountId,
    },
    #[event_version("1.0.0")]
    PlaceBet {
        bettor: AccountId,
        players: (AccountId, AccountId),
        token_id: AccountId,
        amount: U128,
        winner: AccountId,
    },
    #[event_version("1.0.0")]
    CancelBet {
        bettor: AccountId,
        players: (AccountId, AccountId),
        token_id: AccountId,
        amount: U128,
    },
    #[event_version("1.0.0")]
    LockBets {
        players: (AccountId, AccountId),
        game_id: GameId,
    },
    #[event_version("1.0.0")]
    ResolveBets {
        players: (AccountId, AccountId),
        game_id: GameId,
        outcome: GameOutcome,
        fee_bps: u16,
    },
}
