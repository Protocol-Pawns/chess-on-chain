use crate::{GameId, StorageKey};
use near_sdk::{
    borsh::{self, BorshDeserialize, BorshSerialize},
    env,
    store::UnorderedSet,
    AccountId, Balance,
};

#[derive(BorshDeserialize, BorshSerialize)]
pub struct Account {
    pub near_amount: Balance,
    account_id: AccountId,
    game_ids: UnorderedSet<GameId>,
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
        Self {
            account_id,
            near_amount,
            game_ids: UnorderedSet::new(game_id_prefix),
        }
    }

    pub fn add_game_id(&mut self, game_id: GameId) {
        self.game_ids.insert(game_id);
    }

    pub fn remove_game_id(&mut self, game_id: &GameId) -> bool {
        self.game_ids.remove(game_id)
    }

    pub fn is_playing(&self) -> bool {
        !self.game_ids.is_empty()
    }
}
