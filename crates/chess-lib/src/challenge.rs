use near_sdk::{
    borsh::{self, BorshDeserialize, BorshSerialize},
    env,
    serde::Serialize,
    AccountId, Balance,
};
use std::fmt;
use witgen::witgen;

use crate::ContractError;

#[witgen]
pub type ChallengeId = String;

#[witgen]
pub type Wager = Option<(AccountId, Balance)>;

#[derive(
    BorshDeserialize, BorshSerialize, Clone, Debug, Eq, Ord, PartialEq, PartialOrd, Serialize,
)]
#[serde(crate = "near_sdk::serde")]
#[witgen]
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
    ) -> Result<(), ContractError> {
        if challenged_id != &self.challenged {
            return Err(ContractError::WrongChallengedId);
        }
        if paid_wager != &self.wager {
            return Err(ContractError::PaidWager);
        }
        Ok(())
    }

    pub fn check_reject(&self, is_challenger: bool) -> Result<(), ContractError> {
        let sender_id = env::signer_account_id();
        if is_challenger && sender_id != self.challenger {
            return Err(ContractError::WrongChallengerId);
        } else if !is_challenger && sender_id != self.challenged {
            return Err(ContractError::WrongChallengedId);
        }
        Ok(())
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
