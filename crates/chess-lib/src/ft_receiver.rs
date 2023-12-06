use crate::{ChallengeId, Chess, ChessExt, ContractError};
use near_contract_standards::fungible_token::receiver::FungibleTokenReceiver;
use near_sdk::{
    env,
    json_types::U128,
    near_bindgen,
    serde::{Deserialize, Serialize},
    AccountId, PromiseOrValue,
};

#[derive(Debug, Deserialize, Serialize)]
#[serde(crate = "near_sdk::serde")]
pub enum FtReceiverMsg {
    Challenge(ChallengeMsg),
    AcceptChallenge(AcceptChallengeMsg),
    Bet(BetMsg),
}

#[derive(Debug, Deserialize, Serialize)]
#[serde(crate = "near_sdk::serde")]
pub struct ChallengeMsg {
    pub challenged_id: AccountId,
}

#[derive(Debug, Deserialize, Serialize)]
#[serde(crate = "near_sdk::serde")]
pub struct AcceptChallengeMsg {
    pub challenge_id: ChallengeId,
}

#[derive(Debug, Deserialize, Serialize)]
#[serde(crate = "near_sdk::serde")]
pub struct BetMsg {
    pub players: (AccountId, AccountId),
    pub winner: AccountId,
}

#[near_bindgen]
impl FungibleTokenReceiver for Chess {
    fn ft_on_transfer(
        &mut self,
        sender_id: AccountId,
        amount: U128,
        msg: String,
    ) -> PromiseOrValue<U128> {
        match self.internal_ft_on_transfer(sender_id, amount, msg) {
            Ok(res) => res,
            Err(err) => {
                panic!("{}", err);
            }
        }
    }
}
impl Chess {
    fn internal_ft_on_transfer(
        &mut self,
        sender_id: AccountId,
        amount: U128,
        msg: String,
    ) -> Result<PromiseOrValue<U128>, ContractError> {
        let msg = serde_json::from_str(&msg).map_err(|_| ContractError::Deserialize)?;
        let token_id = env::predecessor_account_id();
        if !self.wager_whitelist.contains(&token_id) {
            return Err(ContractError::WagerNoWhitelist);
        }

        let refund = match msg {
            FtReceiverMsg::Challenge(ChallengeMsg { challenged_id }) => {
                let challenger_id = sender_id;
                self.internal_challenge(challenger_id, challenged_id, Some((token_id, amount)))?;
                None
            }
            FtReceiverMsg::AcceptChallenge(AcceptChallengeMsg { challenge_id }) => {
                let challenged_id = sender_id;
                self.internal_accept_challenge(
                    challenged_id,
                    challenge_id,
                    Some((token_id, amount)),
                )?
                .1
            }
            FtReceiverMsg::Bet(BetMsg { players, winner }) => {
                self.internal_bet(sender_id, token_id, amount.0, players, winner)?;
                None
            }
        };

        Ok(PromiseOrValue::Value(refund.unwrap_or_default().into()))
    }
}
