use crate::{ContractError, GameId, StorageKey};
use near_sdk::{
    borsh::{self, BorshDeserialize, BorshSerialize},
    env,
    store::UnorderedSet,
    AccountId, Balance,
};

#[derive(BorshDeserialize, BorshSerialize)]
pub enum Account {
    V1(AccountV1),
}

#[derive(BorshDeserialize, BorshSerialize)]
pub struct AccountV1 {
    pub near_amount: Balance,
    account_id: AccountId,
    game_ids: UnorderedSet<GameId>,
    finished_games: UnorderedSet<GameId>,
}

#[derive(BorshDeserialize, BorshSerialize)]
pub struct OldAccount {
    near_amount: Balance,
    account_id: AccountId,
    game_ids: UnorderedSet<GameId>,
}

impl From<&OldAccount> for Account {
    fn from(old_account: &OldAccount) -> Self {
        let id = env::sha256_array(old_account.account_id.as_bytes());
        let game_id_prefix: Vec<u8> = [
            StorageKey::Accounts.try_to_vec().unwrap().as_slice(),
            &id,
            StorageKey::AccountOrderIds.try_to_vec().unwrap().as_slice(),
        ]
        .concat();
        let finished_games_prefix: Vec<u8> = [
            StorageKey::Accounts.try_to_vec().unwrap().as_slice(),
            &id,
            StorageKey::AccountFinishedGames
                .try_to_vec()
                .unwrap()
                .as_slice(),
        ]
        .concat();
        let mut game_ids = UnorderedSet::new(game_id_prefix);
        for game_id in old_account.game_ids.iter() {
            game_ids.insert(game_id.clone());
        }

        Self::V1(AccountV1 {
            near_amount: old_account.near_amount,
            account_id: old_account.account_id.clone(),
            game_ids,
            finished_games: UnorderedSet::new(finished_games_prefix),
        })
    }
}

impl Account {
    pub fn new(account_id: AccountId, near_amount: Balance) -> Self {
        let id = env::sha256_array(account_id.as_bytes());
        let game_id_prefix: Vec<u8> = [
            StorageKey::Accounts.try_to_vec().unwrap().as_slice(),
            &id,
            StorageKey::AccountOrderIds.try_to_vec().unwrap().as_slice(),
        ]
        .concat();
        let finished_games_prefix: Vec<u8> = [
            StorageKey::Accounts.try_to_vec().unwrap().as_slice(),
            &id,
            StorageKey::AccountFinishedGames
                .try_to_vec()
                .unwrap()
                .as_slice(),
        ]
        .concat();
        Self::V1(AccountV1 {
            account_id,
            near_amount,
            game_ids: UnorderedSet::new(game_id_prefix),
            finished_games: UnorderedSet::new(finished_games_prefix),
        })
    }

    pub fn get_near_amount(&self) -> Balance {
        let Account::V1(account) = self;
        account.near_amount
    }

    pub fn add_game_id(&mut self, game_id: GameId) -> Result<(), ContractError> {
        let Account::V1(account) = self;
        if account.game_ids.len() >= 5 {
            Err(ContractError::MaxGamesReached)
        } else {
            account.game_ids.insert(game_id);
            Ok(())
        }
    }

    pub fn remove_game_id(&mut self, game_id: &GameId) -> bool {
        let Account::V1(account) = self;
        account.game_ids.remove(game_id)
    }

    pub fn is_playing(&self) -> bool {
        let Account::V1(account) = self;
        !account.game_ids.is_empty()
    }

    pub fn get_game_ids(&self) -> Vec<GameId> {
        let Account::V1(account) = self;
        account.game_ids.into_iter().cloned().collect()
    }
}
