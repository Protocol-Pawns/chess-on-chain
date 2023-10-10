use std::collections::HashMap;

use crate::{ChallengeId, Chess, GameId, GameOutcome, TGAS};
use chess_engine::Color;
use near_sdk::{
    env, ext_contract,
    serde::{Deserialize, Serialize},
    AccountId, Gas, PublicKey,
};
use serde_json::{json, Value};
use urlencoding::encode;

#[ext_contract(social_db)]
trait SocialDb {
    fn set(&mut self, data: Value);

    fn is_write_permission_granted(
        &self,
        predecessor_id: Option<AccountId>,
        public_key: Option<PublicKey>,
        key: String,
    ) -> bool;
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(crate = "near_sdk::serde")]
pub struct CustomNotification {
    #[serde(rename = "type")]
    pub _type: String,
    pub message: String,
    pub widget: String,
    pub params: Value,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(crate = "near_sdk::serde")]
#[serde(tag = "type", content = "data")]
#[serde(rename_all = "kebab-case")]
pub enum ChessNotification {
    Challenged {
        challenge_id: ChallengeId,
        challenger_id: AccountId,
    },
    AcceptedChallenge {
        game_id: GameId,
        challenged_id: AccountId,
    },
    RejectedChallenge {
        challenged_id: AccountId,
    },
    YourTurn {
        game_id: GameId,
    },
    Outcome {
        game_id: GameId,
        outcome: GameOutcome,
    },
}

impl Chess {
    pub(crate) fn internal_send_notify(
        &self,
        notifications: HashMap<AccountId, Vec<ChessNotification>>,
    ) {
        let notifications: Vec<_> = notifications
            .iter()
            .flat_map(|(account_id, notifications)| {
                let mut notifications: Vec<_> = notifications
                    .iter()
                    .map(|notification| chess_notification_to_value(account_id, notification))
                    .collect();
                notifications.push(json!({
                    "key": account_id,
                    "value": {
                        "type": "poke"
                    }
                }));
                notifications
            })
            .collect();
        if notifications.is_empty() {
            return;
        }
        social_db::ext(self.social_db.clone())
            .with_static_gas(Gas(20 * TGAS))
            .set(json!({
                env::current_account_id(): {
                    "index": {
                        "notify": json!(notifications).to_string()
                    }
                }
            }));
    }
}

pub fn chess_notification_to_value(
    notifier: &AccountId,
    notification: &ChessNotification,
) -> Value {
    json!({
        "key": notifier.to_string(),
        "value": match notification {
            ChessNotification::Challenged {
                challenge_id,
                challenger_id,
            } => CustomNotification {
                _type: "custom".to_string(),
                message: format!("{challenger_id} challenged you for a chess game"),
                widget: "chess-game.near/widget/ChessGameLobby".to_string(),
                params: json!({
                    "tab": "challenge",
                    "challenge_id": challenge_id
                }),
            },
            ChessNotification::AcceptedChallenge {
                game_id,
                challenged_id,
            } => CustomNotification {
                _type: "custom".to_string(),
                message: format!("{challenged_id} accepted your chess game challenge"),
                widget: "chess-game.near/widget/ChessGameLobby".to_string(),
                params: json!({
                    "tab": "game",
                    "game_id": encode(&json!(game_id).to_string())
                }),
            },
            ChessNotification::RejectedChallenge { challenged_id } => CustomNotification {
                _type: "custom".to_string(),
                message: format!("{challenged_id} rejected your chess game challenge"),
                widget: "chess-game.near/widget/ChessGameLobby".to_string(),
                params: json!({
                    "tab": "challenge"
                }),
            },
            ChessNotification::YourTurn { game_id } => {
                let against = if &game_id.1 == notifier {
                    game_id.2.clone().unwrap().to_string()
                } else {
                    game_id.1.to_string()
                };
                CustomNotification {
                _type: "custom".to_string(),
                message: format!("It is your chess game turn against {against}"),
                widget: "chess-game.near/widget/ChessGameLobby".to_string(),
                params: json!({
                    "tab": "game",
                    "game_id": encode(&json!(game_id).to_string())
                }),
            }},
            ChessNotification::Outcome { game_id, outcome } => {
                let against = if &game_id.1 == notifier {
                    game_id.2.clone().map(|id| id.to_string()).unwrap_or_else(|| "AI".to_string())
                } else {
                    game_id.1.to_string()
                };
                let message = match outcome {
                    GameOutcome::Victory(Color::White) => if &game_id.1 == notifier {
                        format!("You won your chess game against {against}")
                    } else {
                        format!("You lost your chess game against {against}")
                    },
                    GameOutcome::Victory(Color::Black) => if &game_id.1 == notifier {
                        format!("You lost your chess game against {against}")
                    } else {
                        format!("You won your chess game against {against}")
                    },
                    GameOutcome::Stalemate => format!("Your chess game against {against} ended with stalemate")
                };
                CustomNotification {
                _type: "custom".to_string(),
                message,
                widget: "chess-game.near/widget/ChessGameLobby".to_string(),
                params: json!({
                    "tab": "replay",
                    "game_id": encode(&json!(game_id).to_string())
                }),
            }},
        }
    })
}
