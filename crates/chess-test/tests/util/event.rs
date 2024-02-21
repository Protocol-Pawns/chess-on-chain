use chess_engine::Color;
use chess_lib::{ChallengeId, GameId, GameOutcome, MoveStr, Player, Wager};
use near_sdk::AccountId;
use owo_colors::OwoColorize;
use serde::{Deserialize, Serialize};
use std::fmt::{self, Display, Formatter};

#[derive(Clone, Serialize, Deserialize, Debug)]
#[serde(tag = "standard")]
#[serde(rename_all = "kebab-case")]
pub enum ContractEvent {
    Nep141(Nep141Event),
    ChessGame(ChessEvent),
}

#[derive(Clone, Serialize, Deserialize, Debug)]
pub struct Nep141Event {
    pub version: String,
    #[serde(flatten)]
    pub event_kind: Nep141EventKind,
}

#[derive(Clone, Serialize, Deserialize, Debug)]
#[serde(tag = "event", content = "data")]
#[serde(rename_all = "snake_case")]
pub enum Nep141EventKind {
    FtTransfer(Vec<FtTransfer>),
    FtMint(Vec<FtMint>),
}

#[derive(Clone, Serialize, Deserialize, Debug)]
pub struct FtTransfer {
    pub old_owner_id: String,
    pub new_owner_id: String,
    pub amount: String,
    pub memo: Option<String>,
}

#[derive(Clone, Serialize, Deserialize, Debug)]
pub struct FtMint {
    pub owner_id: String,
    pub amount: String,
    pub memo: Option<String>,
}

#[derive(Clone, Serialize, Deserialize, Debug)]
pub struct ChessEvent {
    pub version: String,
    #[serde(flatten)]
    pub event_kind: ChessEventKind,
}

pub const KNOWN_EVENT_KINDS: [&str; 7] = [
    "challenge",
    "accept_challenge",
    "reject_challenge",
    "create_game",
    "play_move",
    "resign_game",
    "cancel_game",
];

#[derive(Clone, Serialize, Deserialize, Debug)]
#[serde(tag = "event", content = "data")]
#[serde(rename_all = "snake_case")]
pub enum ChessEventKind {
    Challenge(Challenge),
    AcceptChallenge(AcceptChallenge),
    RejectChallenge(RejectChallenge),
    CreateGame(CreateGame),
    PlayMove(PlayMove),
    ResignGame(ResignGame),
    CancelGame(CancelGame),
}

#[derive(Clone, Serialize, Deserialize, Debug)]
pub struct Challenge {
    pub id: String,
    pub challenger: AccountId,
    pub challenged: AccountId,
    pub wager: Wager,
}

#[derive(Clone, Serialize, Deserialize, Debug)]
pub struct AcceptChallenge {
    pub challenge_id: ChallengeId,
    pub game_id: GameId,
}

#[derive(Clone, Serialize, Deserialize, Debug)]
pub struct RejectChallenge {
    pub challenge_id: ChallengeId,
}

#[derive(Clone, Serialize, Deserialize, Debug)]
pub struct CreateGame {
    pub game_id: GameId,
    pub white: Player,
    pub black: Player,
    pub board: [String; 8],
}

#[derive(Clone, Serialize, Deserialize, Debug)]
pub struct PlayMove {
    pub game_id: GameId,
    pub color: Color,
    pub mv: MoveStr,
    pub board: [String; 8],
    pub outcome: Option<GameOutcome>,
}

#[derive(Clone, Serialize, Deserialize, Debug)]
pub struct ResignGame {
    pub game_id: GameId,
    pub resigner: Color,
    pub outcome: GameOutcome,
}

#[derive(Clone, Serialize, Deserialize, Debug)]
pub struct CancelGame {
    pub game_id: GameId,
    pub cancelled_by: String,
}

impl Display for ContractEvent {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> fmt::Result {
        match self {
            ContractEvent::Nep141(event) => formatter.write_fmt(format_args!("{}", event)),
            ContractEvent::ChessGame(event) => formatter.write_fmt(format_args!("{}", event)),
        }
    }
}

impl Display for Nep141Event {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> fmt::Result {
        match &self.event_kind {
            Nep141EventKind::FtTransfer(_) => {
                formatter.write_fmt(format_args!("{}: ft_transfer", "event".bright_cyan()))?;
            }
            Nep141EventKind::FtMint(_) => {
                formatter.write_fmt(format_args!("{}: ft_mint", "event".bright_cyan()))?;
            }
        }
        formatter.write_fmt(format_args!("\n{}: nep141", "standard".bright_cyan(),))?;
        formatter.write_fmt(format_args!(
            "\n{}: {}",
            "version".bright_cyan(),
            self.version
        ))?;
        match &self.event_kind {
            Nep141EventKind::FtTransfer(datas) => {
                for data in datas {
                    formatter.write_fmt(format_args!("\n{}: {}", "data".bright_cyan(), data))?;
                }
            }
            Nep141EventKind::FtMint(datas) => {
                for data in datas {
                    formatter.write_fmt(format_args!("\n{}: {}", "data".bright_cyan(), data))?;
                }
            }
        }
        Ok(())
    }
}

impl Display for FtTransfer {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> fmt::Result {
        if let Some(memo) = &self.memo {
            formatter.write_fmt(format_args!(
                "{} --> {} ({}) --> {}",
                self.old_owner_id.bright_blue(),
                self.amount.bright_blue(),
                memo,
                self.new_owner_id.bright_blue(),
            ))?;
        } else {
            formatter.write_fmt(format_args!(
                "{} --> {} --> {}",
                self.old_owner_id.bright_blue(),
                self.amount.bright_blue(),
                self.new_owner_id.bright_blue(),
            ))?;
        }
        Ok(())
    }
}

impl Display for FtMint {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> fmt::Result {
        if let Some(memo) = &self.memo {
            formatter.write_fmt(format_args!(
                "{} ({}) --> {}",
                self.amount.bright_blue(),
                memo,
                self.owner_id.bright_blue(),
            ))?;
        } else {
            formatter.write_fmt(format_args!(
                "{} --> {}",
                self.amount.bright_blue(),
                self.owner_id.bright_blue(),
            ))?;
        }
        Ok(())
    }
}

impl Display for ChessEvent {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> fmt::Result {
        match &self.event_kind {
            ChessEventKind::Challenge(_) => {
                formatter.write_fmt(format_args!("{}: challenge", "event".bright_cyan()))?;
            }
            ChessEventKind::AcceptChallenge(_) => {
                formatter.write_fmt(format_args!("{}: accept_challenge", "event".bright_cyan()))?;
            }
            ChessEventKind::RejectChallenge(_) => {
                formatter.write_fmt(format_args!("{}: reject_challenge", "event".bright_cyan()))?;
            }
            ChessEventKind::CreateGame(_) => {
                formatter.write_fmt(format_args!("{}: create_game", "event".bright_cyan()))?;
            }
            ChessEventKind::PlayMove(_) => {
                formatter.write_fmt(format_args!("{}: play_move", "event".bright_cyan()))?;
            }
            ChessEventKind::ResignGame(_) => {
                formatter.write_fmt(format_args!("{}: resign_game", "event".bright_cyan()))?;
            }
            ChessEventKind::CancelGame(_) => {
                formatter.write_fmt(format_args!("{}: cancel_game", "event".bright_cyan()))?;
            }
        }
        formatter.write_fmt(format_args!("\n{}: chess-game", "standard".bright_cyan(),))?;
        formatter.write_fmt(format_args!(
            "\n{}: {}",
            "version".bright_cyan(),
            self.version
        ))?;
        match &self.event_kind {
            ChessEventKind::Challenge(data) => {
                formatter.write_fmt(format_args!("\n{}: {:?}", "data".bright_cyan(), data))?;
            }
            ChessEventKind::AcceptChallenge(data) => {
                formatter.write_fmt(format_args!("\n{}: {:?}", "data".bright_cyan(), data))?;
            }
            ChessEventKind::RejectChallenge(data) => {
                formatter.write_fmt(format_args!("\n{}: {:?}", "data".bright_cyan(), data))?;
            }
            ChessEventKind::CreateGame(data) => {
                formatter.write_fmt(format_args!("\n{}: {:?}", "data".bright_cyan(), data))?;
            }
            ChessEventKind::PlayMove(data) => {
                formatter.write_fmt(format_args!("\n{}: {:?}", "data".bright_cyan(), data))?;
            }
            ChessEventKind::ResignGame(data) => {
                formatter.write_fmt(format_args!("\n{}: {:?}", "data".bright_cyan(), data))?;
            }
            ChessEventKind::CancelGame(data) => {
                formatter.write_fmt(format_args!("\n{}: {:?}", "data".bright_cyan(), data))?;
            }
        }
        Ok(())
    }
}
