use near_sdk::{
    ext_contract,
    serde::{Deserialize, Serialize},
    AccountId,
};
use serde_json::Value;

use crate::{GameId, GameOutcome};

#[ext_contract(social_db)]
trait SocialDb {
    fn set(&mut self, data: Value);
}

#[derive(Debug, Deserialize, Serialize)]
#[serde(crate = "near_sdk::serde")]
pub struct IndexNotify {
    pub index: Notify,
}

#[derive(Debug, Deserialize, Serialize)]
#[serde(crate = "near_sdk::serde")]
pub struct Notify {
    pub notify: String,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(crate = "near_sdk::serde")]
pub struct Notification {
    pub key: AccountId,
    pub value: ChessNotification,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(crate = "near_sdk::serde")]
pub struct ChessNotification {
    #[serde(rename = "type")]
    pub _type: String,
    pub item: ChessNotificationItem,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(crate = "near_sdk::serde")]
pub enum ChessNotificationItem {
    YourTurn {
        game_id: GameId,
    },
    Outcome {
        game_id: GameId,
        outcome: GameOutcome,
    },
}
