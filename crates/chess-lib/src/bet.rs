use crate::{
    Account, Achievement, Chess, ChessEvent, ContractError, Quest, StorageKey, MAX_BETS_PER_GAME,
    MAX_OPEN_BETS,
};
use near_sdk::{
    borsh::{self, BorshDeserialize, BorshSerialize},
    env,
    json_types::U128,
    require,
    serde::{Deserialize, Serialize},
    store::IterableMap,
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
        env::sha256_array([self.0.as_bytes(), self.1.as_bytes()].concat())
    }
}

#[derive(BorshDeserialize, BorshSerialize)]
#[borsh(crate = "near_sdk::borsh")]
pub struct Bets {
    pub is_locked: bool,
    pub bets: IterableMap<AccountId, Vec<(AccountId, Bet)>>,
}

#[derive(Debug, Deserialize, Serialize, NearSchema)]
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
    pub fn filter_valid(
        &mut self,
        accounts: &mut IterableMap<AccountId, Account>,
    ) -> HashSet<AccountId> {
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
        let mut refunded_bettors = HashSet::new();
        for token_id in to_remove {
            let bets = self.bets.remove(&token_id).unwrap();
            for (account_id, bet) in bets {
                let account = accounts.get_mut(&account_id).unwrap();
                account.add_token(&token_id, bet.amount);
                refunded_bettors.insert(account_id);
            }
        }
        refunded_bettors
    }
}

#[derive(BorshDeserialize, BorshSerialize, Clone, Debug)]
#[borsh(crate = "near_sdk::borsh")]
pub struct Bet {
    pub amount: u128,
    pub winner: AccountId,
}

#[derive(Deserialize, Serialize, Debug, NearSchema)]
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

impl Chess {
    pub fn internal_bet(
        &mut self,
        sender_id: AccountId,
        token_id: AccountId,
        amount: u128,
        players: (AccountId, AccountId),
        winner: AccountId,
    ) -> Result<(), ContractError> {
        if sender_id == players.0 || sender_id == players.1 {
            return Err(ContractError::PlayerCannotBetOnSelf);
        }
        if winner != players.0 && winner != players.1 {
            return Err(ContractError::InvalidBetWinner);
        }
        require!(amount > 0, "Bet amount must be positive");
        if !self.accounts.contains_key(&sender_id) {
            return Err(ContractError::AccountNotRegistered(sender_id));
        }
        let bet_id = BetId::new(players.clone())?;
        if !self.bets.contains_key(&bet_id) {
            let id = bet_id.get_storage_key();
            let storage_key: Vec<u8> = [
                borsh::to_vec(&StorageKey::V9BetsInner).unwrap().as_slice(),
                &id,
            ]
            .concat();
            let bet = Bets {
                is_locked: false,
                bets: IterableMap::new(storage_key),
            };
            self.bets.insert(bet_id.clone(), bet);
        }
        let bets = self.bets.get_mut(&bet_id).unwrap();
        if bets.is_locked {
            return Err(ContractError::BetLocked);
        }

        let bettor_already_has_bet = bets.bets.iter().any(|(_, bet_list)| {
            bet_list
                .binary_search_by_key(&sender_id, |(account_id, _)| account_id.clone())
                .is_ok()
        });
        if !bettor_already_has_bet {
            let total_bets: usize = bets.bets.iter().map(|(_, bl)| bl.len()).sum();
            if total_bets >= MAX_BETS_PER_GAME as usize {
                return Err(ContractError::MaxBetsPerGameReached);
            }

            let active = self
                .bettor_active_bets
                .get(&sender_id)
                .copied()
                .unwrap_or(0);
            if active >= MAX_OPEN_BETS {
                return Err(ContractError::MaxBetsReached);
            }
            self.bettor_active_bets
                .insert(sender_id.clone(), active + 1);
        }

        if let Some(token_bets) = bets.bets.get_mut(&token_id) {
            match token_bets.binary_search_by_key(&sender_id, |(account_id, _)| account_id.clone())
            {
                Ok(index) => {
                    token_bets.get_mut(index).unwrap().1.amount += amount;
                }
                Err(index) => {
                    token_bets.insert(
                        index,
                        (
                            sender_id.clone(),
                            Bet {
                                amount,
                                winner: winner.clone(),
                            },
                        ),
                    );
                }
            }
        } else {
            bets.bets.insert(
                token_id.clone(),
                vec![(
                    sender_id.clone(),
                    Bet {
                        amount,
                        winner: winner.clone(),
                    },
                )],
            );
        }

        let event = ChessEvent::PlaceBet {
            bettor: sender_id.clone(),
            players,
            token_id,
            amount: amount.into(),
            winner,
        };
        event.emit();

        let account = self.accounts.get_mut(&sender_id).unwrap();
        account.record_bet_placed();
        let points = account.apply_quest(Quest::WeeklyBettor, true);
        self.points_total_supply += points;
        let bets_placed = account.get_bets_placed();
        if bets_placed == 1 {
            let p = account.apply_achievement(Achievement::FirstBet, true);
            self.points_total_supply += p;
        }

        Ok(())
    }
}
