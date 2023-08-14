use crate::{Chess, ChessExt, ContractError, GameId, GameOutcome, TGAS};
use near_sdk::{
    env, ext_contract, near_bindgen,
    serde::{Deserialize, Serialize},
    AccountId, Gas, Promise, PublicKey,
};
use serde_json::Value;
use std::collections::HashMap;

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

#[near_bindgen]
impl Chess {
    pub fn update_enabled_notifications(&mut self, account_id: AccountId) -> Promise {
        social_db::ext(self.social_db.clone())
            .with_static_gas(Gas(20 * TGAS))
            .is_write_permission_granted(
                Some(env::current_account_id()),
                None,
                format!("{}/index/notify", account_id),
            )
            .then(Self::ext(env::current_account_id()).set_enabled_notifications(account_id))
    }

    #[private]
    #[handle_result]
    pub fn set_enabled_notifications(
        &mut self,
        account_id: AccountId,
        #[callback_unwrap] is_enabled: bool,
    ) -> Result<(), ContractError> {
        near_sdk::log!("is_enabled {} {}", account_id.as_str(), is_enabled);
        let account = self
            .accounts
            .get_mut(&account_id)
            .ok_or_else(|| ContractError::AccountNotRegistered(account_id.clone()))?;
        account.set_enabled_notifications(is_enabled);

        Ok(())
    }
}

impl Chess {
    pub(crate) fn internal_send_notify(&self, notifications: HashMap<AccountId, Notification>) {
        for (account_id, notification) in notifications {
            let account = self.internal_get_account(&account_id).unwrap();
            if !account.enabled_notifications() {
                continue;
            }
            social_db::ext(self.social_db.clone())
                .with_static_gas(Gas(20 * TGAS))
                .set(serde_json::json!({
                    account_id: IndexNotify {
                        index: Notify {
                            notify: serde_json::to_string(&notification).unwrap()
                        }
                    }
                }));
        }
    }
}
