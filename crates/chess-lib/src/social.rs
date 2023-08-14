use crate::{Chess, GameId, GameOutcome, TGAS};
use near_sdk::{
    env, ext_contract,
    serde::{Deserialize, Serialize},
    AccountId, Gas, PublicKey,
};
use serde_json::Value;

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
    Challenged {
        challenger_id: AccountId,
    },
    AcceptedChallenge {
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
    pub(crate) fn internal_send_notify(&self, notifications: Vec<Notification>) {
        if notifications.len() == 1 {
            social_db::ext(self.social_db.clone())
                .with_static_gas(Gas(20 * TGAS))
                .set(serde_json::json!({
                    env::current_account_id(): IndexNotify {
                        index: Notify {
                            notify: serde_json::to_string(notifications.get(0).unwrap()).unwrap()
                        }
                    }
                }));
        } else {
            social_db::ext(self.social_db.clone())
                .with_static_gas(Gas(20 * TGAS))
                .set(serde_json::json!({
                    env::current_account_id(): IndexNotify {
                        index: Notify {
                            notify: serde_json::to_string(&notifications).unwrap()
                        }
                    }
                }));
        }
    }
}
