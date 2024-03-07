use crate::{Account, Chess, ContractError, StorageKey};
use near_sdk::{
    borsh::{self, BorshDeserialize, BorshSerialize},
    env,
    json_types::U128,
    serde::{Deserialize, Serialize},
    store::UnorderedMap,
    AccountId, NearSchema,
};
use std::{
    cmp::Ordering,
    collections::{HashMap, HashSet},
};

/// (player_0_id, player_1_id) sorted alphabetically
#[derive(BorshDeserialize, BorshSerialize, Clone, Hash, Ord, PartialOrd, PartialEq, Eq)]
#[borsh(crate = "near_sdk::borsh")]
pub struct BetId(AccountId, AccountId);

impl BetId {
    pub fn new(mut players: (AccountId, AccountId)) -> Result<Self, ContractError> {
        match players.0.cmp(&players.1) {
            Ordering::Less => {}
            Ordering::Greater => std::mem::swap(&mut players.0, &mut players.1),
            Ordering::Equal => return Err(ContractError::InvalidBetPlayers),
        }
        Ok(BetId(players.0, players.1))
    }

    pub fn get_storage_key(&self) -> [u8; 32] {
        env::sha256_array(&[self.0.as_bytes(), self.1.as_bytes()].concat())
    }
}

#[derive(BorshDeserialize, BorshSerialize)]
#[borsh(crate = "near_sdk::borsh")]
pub struct Bets {
    pub is_locked: bool,
    /// token_id -> vec<(account_id, bet)>
    pub bets: UnorderedMap<AccountId, Vec<(AccountId, Bet)>>,
}

#[derive(Debug, Deserialize, Serialize)]
#[cfg_attr(not(target = "wasm32-unknown-unknown"), derive(NearSchema))]
#[serde(crate = "near_sdk::serde")]
pub struct BetInfo {
    pub is_locked: bool,
    pub bets: HashMap<AccountId, Vec<(AccountId, BetView)>>,
}

impl From<&Bets> for BetInfo {
    fn from(bets: &Bets) -> Self {
        BetInfo {
            is_locked: bets.is_locked,
            bets: bets
                .bets
                .iter()
                .map(|(a, b)| {
                    (
                        a.clone(),
                        b.iter()
                            .map(|(id, bet)| (id.clone(), bet.clone().into()))
                            .collect(),
                    )
                })
                .collect::<HashMap<_, _>>(),
        }
    }
}

impl Bets {
    pub fn filter_valid(&mut self, accounts: &mut UnorderedMap<AccountId, Account>) {
        let mut to_remove = vec![];
        'outer: for (token_id, bets) in self.bets.iter() {
            let mut set = HashSet::new();
            for (_, bet) in bets.iter() {
                set.insert(bet.winner.clone());
                if set.len() == 2 {
                    continue 'outer;
                }
            }
            to_remove.push(token_id.clone());
        }
        for token_id in to_remove {
            let bets = self.bets.remove(&token_id).unwrap();
            for (account_id, bet) in bets {
                let account = accounts.get_mut(&account_id).unwrap();
                account.add_token(&token_id, bet.amount);
            }
        }
    }
}

#[derive(BorshDeserialize, BorshSerialize, Clone, Debug)]
#[borsh(crate = "near_sdk::borsh")]
pub struct Bet {
    pub amount: u128,
    pub winner: AccountId,
}

#[derive(Deserialize, Serialize, Debug)]
#[cfg_attr(not(target = "wasm32-unknown-unknown"), derive(NearSchema))]
#[serde(crate = "near_sdk::serde")]
pub struct BetView {
    pub amount: U128,
    pub winner: AccountId,
}

impl From<Bet> for BetView {
    fn from(bet: Bet) -> Self {
        BetView {
            amount: bet.amount.into(),
            winner: bet.winner,
        }
    }
}

// TODO `cancel_bet`
// TODO BOS notifications
// TODO message instead of poke?
impl Chess {
    pub fn internal_bet(
        &mut self,
        sender_id: AccountId,
        token_id: AccountId,
        amount: u128,
        players: (AccountId, AccountId),
        winner: AccountId,
    ) -> Result<(), ContractError> {
        let bet_id = BetId::new(players)?;
        if !self.bets.contains_key(&bet_id) {
            let id = bet_id.get_storage_key();
            let storage_key: Vec<u8> =
                [borsh::to_vec(&StorageKey::Bets).unwrap().as_slice(), &id].concat();
            let bet = Bets {
                is_locked: false,
                bets: UnorderedMap::new(storage_key),
            };
            self.bets.insert(bet_id.clone(), bet);
        }
        let bets = self.bets.get_mut(&bet_id).unwrap();
        if bets.is_locked {
            return Err(ContractError::BetLocked);
        }

        if let Some(bets) = bets.bets.get_mut(&token_id) {
            if let Ok(index) =
                bets.binary_search_by_key(&sender_id, |(account_id, _)| account_id.clone())
            {
                bets.get_mut(index).unwrap().1.amount += amount;
            } else {
                bets.push((sender_id, Bet { amount, winner }));
            }
        } else {
            bets.bets
                .insert(token_id, vec![(sender_id, Bet { amount, winner })]);
        }
        Ok(())
    }
}
