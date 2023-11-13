use crate::ContractError;
use near_sdk::{
    borsh::{BorshDeserialize, BorshSerialize},
    env,
    json_types::U128,
    serde::{Deserialize, Serialize},
    AccountId, Balance,
};
use std::fmt;

pub type ChallengeId = String;

pub type Wager = Option<(AccountId, U128)>;

#[derive(
    BorshDeserialize,
    BorshSerialize,
    Clone,
    Debug,
    Eq,
    Ord,
    PartialEq,
    PartialOrd,
    Serialize,
    Deserialize,
)]
#[serde(crate = "near_sdk::serde")]
#[borsh(crate = "near_sdk::borsh")]
pub struct Challenge {
    id: String,
    challenger: AccountId,
    challenged: AccountId,
    wager: Wager,
}

pub fn create_challenge_id<T: fmt::Display>(challenger: T, challenged: T) -> String {
    format!("{}-vs-{}", challenger, challenged)
}

impl Challenge {
    pub fn new(challenger: AccountId, challenged: AccountId, wager: Wager) -> Self {
        Self {
            id: create_challenge_id(&challenger, &challenged),
            challenger,
            challenged,
            wager,
        }
    }

    pub fn check_accept(
        &self,
        challenged_id: &AccountId,
        paid_wager: &Wager,
    ) -> Result<Option<Balance>, ContractError> {
        if challenged_id != &self.challenged {
            return Err(ContractError::WrongChallengedId);
        }
        if let (Some(paid_wager), Some(wager)) = (paid_wager, &self.wager) {
            if paid_wager.0 != wager.0 || paid_wager.1 < wager.1 {
                return Err(ContractError::PaidWager);
            }
            Ok(Some(paid_wager.1 .0 - wager.1 .0))
        } else {
            Ok(None)
        }
    }

    pub fn check_reject(&self, is_challenger: bool) -> Result<Wager, ContractError> {
        let sender_id = env::signer_account_id();
        if is_challenger && sender_id != self.challenger {
            return Err(ContractError::WrongChallengerId);
        } else if !is_challenger && sender_id != self.challenged {
            return Err(ContractError::WrongChallengedId);
        }
        Ok(self.wager.clone())
    }

    pub fn id(&self) -> &String {
        &self.id
    }

    pub fn get_challenger(&self) -> &AccountId {
        &self.challenger
    }

    pub fn get_challenged(&self) -> &AccountId {
        &self.challenged
    }
}
