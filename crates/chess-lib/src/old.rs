use crate::{Account, Challenge, ChallengeId, Fees, Game, GameId};
use near_sdk::{
    borsh::{BorshDeserialize, BorshSerialize},
    store::{Lazy, UnorderedMap},
    AccountId, PanicOnDefault,
};
use std::collections::VecDeque;

#[derive(BorshDeserialize, BorshSerialize, PanicOnDefault)]
#[borsh(crate = "near_sdk::borsh")]
pub struct OldChess {
    pub social_db: AccountId,
    pub iah_registry: AccountId,
    pub accounts: UnorderedMap<AccountId, Account>,
    pub games: UnorderedMap<GameId, Game>,
    pub challenges: UnorderedMap<ChallengeId, Challenge>,
    pub recent_finished_games: Lazy<VecDeque<GameId>>,
    pub treasury: UnorderedMap<AccountId, u128>,
    pub fees: Lazy<Fees>,
    pub wager_whitelist: Lazy<Vec<AccountId>>,
}
