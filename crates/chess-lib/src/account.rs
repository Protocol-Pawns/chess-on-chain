use crate::{ChallengeId, ContractError, GameId, StorageKey};
use near_sdk::{
    borsh::{self, BorshDeserialize, BorshSerialize},
    env,
    store::UnorderedSet,
    AccountId, Balance,
};

#[derive(BorshDeserialize, BorshSerialize)]
pub enum Account {
    V1(AccountV1),
    V2(AccountV2),
}

#[derive(BorshDeserialize, BorshSerialize)]
pub struct AccountV1 {
    near_amount: Balance,
    account_id: AccountId,
    game_ids: UnorderedSet<GameId>,
    finished_games: UnorderedSet<GameId>,
}

#[derive(BorshDeserialize, BorshSerialize)]
pub struct AccountV2 {
    near_amount: Balance,
    account_id: AccountId,
    game_ids: UnorderedSet<GameId>,
    finished_games: UnorderedSet<GameId>,
    challenger: UnorderedSet<ChallengeId>,
    challenged: UnorderedSet<ChallengeId>,
}

impl Account {
    pub fn new(account_id: AccountId, near_amount: Balance) -> Self {
        let id = env::sha256_array(account_id.as_bytes());
        let game_id_prefix: Vec<u8> = [
            StorageKey::VAccounts.try_to_vec().unwrap().as_slice(),
            &id,
            StorageKey::AccountOrderIds.try_to_vec().unwrap().as_slice(),
        ]
        .concat();
        let finished_games_prefix: Vec<u8> = [
            StorageKey::VAccounts.try_to_vec().unwrap().as_slice(),
            &id,
            StorageKey::AccountFinishedGames
                .try_to_vec()
                .unwrap()
                .as_slice(),
        ]
        .concat();
        let challenger_prefix: Vec<u8> = [
            StorageKey::VAccounts.try_to_vec().unwrap().as_slice(),
            &id,
            StorageKey::AccountChallenger
                .try_to_vec()
                .unwrap()
                .as_slice(),
        ]
        .concat();
        let challenged_prefix: Vec<u8> = [
            StorageKey::VAccounts.try_to_vec().unwrap().as_slice(),
            &id,
            StorageKey::AccountChallenged
                .try_to_vec()
                .unwrap()
                .as_slice(),
        ]
        .concat();
        Self::V2(AccountV2 {
            account_id,
            near_amount,
            game_ids: UnorderedSet::new(game_id_prefix),
            finished_games: UnorderedSet::new(finished_games_prefix),
            challenger: UnorderedSet::new(challenger_prefix),
            challenged: UnorderedSet::new(challenged_prefix),
        })
    }

    pub fn migrate(self) -> Self {
        if let Account::V1(AccountV1 {
            near_amount,
            account_id,
            game_ids,
            finished_games,
        }) = self
        {
            let id = env::sha256_array(account_id.as_bytes());
            let challenger_prefix: Vec<u8> = [
                StorageKey::VAccounts.try_to_vec().unwrap().as_slice(),
                &id,
                StorageKey::AccountChallenger
                    .try_to_vec()
                    .unwrap()
                    .as_slice(),
            ]
            .concat();
            let challenged_prefix: Vec<u8> = [
                StorageKey::VAccounts.try_to_vec().unwrap().as_slice(),
                &id,
                StorageKey::AccountChallenged
                    .try_to_vec()
                    .unwrap()
                    .as_slice(),
            ]
            .concat();
            Account::V2(AccountV2 {
                near_amount,
                account_id,
                game_ids,
                finished_games,
                challenger: UnorderedSet::new(challenger_prefix),
                challenged: UnorderedSet::new(challenged_prefix),
            })
        } else {
            self
        }
    }

    pub fn get_near_amount(&self) -> Balance {
        let Account::V2(account) = self else {
            panic!("migration required");
        };
        account.near_amount
    }

    pub fn add_game_id(&mut self, game_id: GameId) -> Result<(), ContractError> {
        let Account::V2(account) = self else {
            panic!("migration required");
        };
        if account.game_ids.len() >= 5 {
            return Err(ContractError::MaxGamesReached);
        }
        account.game_ids.insert(game_id);
        Ok(())
    }

    pub fn remove_game_id(&mut self, game_id: &GameId) -> bool {
        let Account::V2(account) = self else {
            panic!("migration required");
        };
        account.game_ids.remove(game_id)
    }

    pub fn save_finished_game(&mut self, game_id: GameId) {
        let Account::V2(account) = self else {
            panic!("migration required");
        };
        account.finished_games.insert(game_id);
    }

    pub fn is_playing(&self) -> bool {
        let Account::V2(account) = self else {
            panic!("migration required");
        };
        !account.game_ids.is_empty()
    }

    pub fn get_game_ids(&self) -> Vec<GameId> {
        let Account::V2(account) = self else {
            panic!("migration required");
        };
        account.game_ids.into_iter().cloned().collect()
    }

    pub fn get_finished_games(&self) -> Vec<GameId> {
        let Account::V2(account) = self else {
            panic!("migration required");
        };
        account.finished_games.into_iter().cloned().collect()
    }

    pub fn accept_challenge(
        &mut self,
        challenge_id: &ChallengeId,
        game_id: GameId,
        is_challenger: bool,
    ) -> Result<(), ContractError> {
        self.add_game_id(game_id)?;
        let Account::V2(account) = self else {
            panic!("migration required");
        };
        if is_challenger {
            account.challenger.remove(challenge_id);
        } else {
            account.challenged.remove(challenge_id);
        }
        Ok(())
    }

    pub fn reject_challenge(
        &mut self,
        challenge_id: &ChallengeId,
        is_challenger: bool,
    ) -> Result<(), ContractError> {
        let Account::V2(account) = self else {
            panic!("migration required");
        };
        if is_challenger {
            account.challenger.remove(challenge_id);
        } else {
            account.challenged.remove(challenge_id);
        }
        Ok(())
    }

    pub fn add_challenge(&mut self, challenge_id: ChallengeId, is_challenger: bool) {
        let Account::V2(account) = self else {
            panic!("migration required");
        };
        if is_challenger {
            account.challenger.insert(challenge_id);
        } else {
            account.challenged.insert(challenge_id);
        }
    }

    pub fn get_challenges(&self, is_challenger: bool) -> Vec<ChallengeId> {
        let Account::V2(account) = self else {
            panic!("migration required");
        };
        if is_challenger {
            account.challenger.iter().cloned().collect()
        } else {
            account.challenged.iter().cloned().collect()
        }
    }
}
