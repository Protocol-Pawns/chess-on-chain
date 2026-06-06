mod account;
mod bet;
mod challenge;
mod elo;
mod error;
mod event;
mod ft_receiver;
mod game;
mod internal;
mod points;
mod storage;
mod view;

pub use account::*;
pub use bet::*;
pub use challenge::*;
pub use elo::*;
pub use error::*;
pub use event::*;
pub use ft_receiver::*;
pub use game::*;
pub use points::*;
pub use storage::*;

use chess_engine::{Color, Move};
use near_contract_standards::fungible_token::core::ext_ft_core;
#[allow(deprecated)]
use near_sdk::{
    assert_one_yocto,
    borsh::{self, BorshDeserialize, BorshSerialize},
    env, near_bindgen, require,
    store::{IterableMap, Lazy, UnorderedMap},
    AccountId, BorshStorageKey, Gas, NearToken, PanicOnDefault, PromiseOrValue, PromiseResult,
};
use std::collections::HashSet;

pub const MAX_OPEN_GAMES: u32 = 5;
pub const MAX_OPEN_CHALLENGES: u32 = 25;
pub const MAX_OPEN_BETS: u32 = 10;

#[cfg(not(feature = "integration-test"))]
pub const MAX_BETS_PER_GAME: u32 = 1000;
#[cfg(feature = "integration-test")]
pub const MAX_BETS_PER_GAME: u32 = 5;

#[cfg(not(feature = "integration-test"))]
pub const MIN_BLOCK_DIFF_CANCEL: u64 = 60 * 60 * 24 * 3; // ~3 days
#[cfg(feature = "integration-test")]
pub const MIN_BLOCK_DIFF_CANCEL: u64 = 100;

pub const NO_DEPOSIT: NearToken = NearToken::from_yoctonear(0);
pub const ONE_YOCTO: NearToken = NearToken::from_yoctonear(1);
pub const FT_TRANSFER_GAS: Gas = Gas::from_tgas(15);
pub const WITHDRAW_CALLBACK_GAS: Gas = Gas::from_tgas(5);
pub const CANCEL_WAGER_CALLBACK_GAS: Gas = Gas::from_tgas(10);
pub const REJECT_WAGER_CALLBACK_GAS: Gas = Gas::from_tgas(10);

#[derive(BorshStorageKey, BorshSerialize)]
#[borsh(crate = "near_sdk::borsh")]
pub enum StorageKey {
    Accounts,
    VAccounts,
    AccountOrderIds,
    AccountFinishedGames,
    AccountChallenger,
    AccountChallenged,
    Games,
    Challenges,
    RecentFinishedGames,
    RecentFinishedGamesV2,
    Treasury,
    Fees,
    TokenWhitelist,
    AccountQuestCooldowns,
    AccountAchievements,
    Bets,
    BettorActiveBets,
    AccountTokens,
    V9AccountOrderIds,
    V9AccountChallenger,
    V9AccountChallenged,
    V9AccountTokens,
    V9BetsInner,
}

#[near_bindgen]
#[derive(BorshDeserialize, BorshSerialize, PanicOnDefault)]
#[borsh(crate = "near_sdk::borsh")]
pub struct Chess {
    pub owner_id: AccountId,
    pub accounts: IterableMap<AccountId, Account>,
    pub games: IterableMap<GameId, Game>,
    pub challenges: IterableMap<ChallengeId, Challenge>,
    pub treasury: IterableMap<AccountId, u128>,
    pub fees: Lazy<u16>,
    pub token_whitelist: Lazy<Vec<AccountId>>,
    pub bets: IterableMap<BetId, Bets>,
    pub bettor_active_bets: IterableMap<AccountId, u32>,
    pub is_running: bool,
    pub points_total_supply: u128,
}

impl near_sdk::state::ContractState for Chess {}

#[derive(BorshDeserialize)]
#[borsh(crate = "near_sdk::borsh")]
#[allow(deprecated)]
pub struct OldChess {
    pub social_db: AccountId,
    pub nada_bot_id: AccountId,
    pub accounts: UnorderedMap<AccountId, Account>,
    pub games: UnorderedMap<GameId, Game>,
    pub challenges: UnorderedMap<ChallengeId, Challenge>,
    pub treasury: UnorderedMap<AccountId, u128>,
    pub fees: Lazy<OldFees>,
    pub token_whitelist: Lazy<Vec<AccountId>>,
    pub bets: UnorderedMap<BetId, OldBets>,
    pub is_running: bool,
    pub points_total_supply: u128,
}

#[derive(BorshDeserialize, BorshSerialize)]
#[borsh(crate = "near_sdk::borsh")]
pub struct OldFees {
    pub treasury: u16,
    pub royalties: Vec<(AccountId, u16)>,
}

/// A valid move will be parsed from a string.
///
/// Possible [valid formats](https://docs.rs/chess-engine/latest/chess_engine/enum.Move.html#method.parse) include:
/// - "e2e4"
/// - "e2 e4"
/// - "e2 to e4"
/// - "castle queenside"
/// - "castle kingside"
pub type MoveStr = String;

#[near_bindgen]
impl Chess {
    #[init]
    #[handle_result]
    pub fn new(owner_id: AccountId) -> Result<Self, ContractError> {
        if env::state_exists() {
            return Err(ContractError::AlreadyInitialized);
        }
        Ok(Self {
            owner_id,
            accounts: IterableMap::new(StorageKey::VAccounts),
            games: IterableMap::new(StorageKey::Games),
            challenges: IterableMap::new(StorageKey::Challenges),
            treasury: IterableMap::new(StorageKey::Treasury),
            fees: Lazy::new(StorageKey::Fees, 0),
            token_whitelist: Lazy::new(StorageKey::TokenWhitelist, Vec::new()),
            bets: IterableMap::new(StorageKey::Bets),
            bettor_active_bets: IterableMap::new(StorageKey::BettorActiveBets),
            is_running: true,
            points_total_supply: 0,
        })
    }

    #[private]
    #[init(ignore_state)]
    pub fn migrate(owner_id: AccountId) -> Self {
        let mut old: OldChess = env::state_read().unwrap();

        let account_ids: Vec<AccountId> = old.accounts.keys().cloned().collect();
        let mut account_data = Vec::with_capacity(account_ids.len());
        for account_id in &account_ids {
            let account = old.accounts.remove(account_id).unwrap();
            account_data.push((account_id.clone(), account.migrate()));
        }
        old.accounts.flush();
        let mut accounts = IterableMap::new(StorageKey::VAccounts);
        for (id, account) in account_data {
            accounts.insert(id, account);
        }

        let game_ids: Vec<GameId> = old.games.keys().cloned().collect();
        let mut games = IterableMap::new(StorageKey::Games);
        for id in &game_ids {
            let game = old.games.remove(id).unwrap();
            games.insert(id.clone(), game);
        }
        old.games.flush();

        let challenge_ids: Vec<ChallengeId> = old.challenges.keys().cloned().collect();
        let mut challenges = IterableMap::new(StorageKey::Challenges);
        for id in &challenge_ids {
            let challenge = old.challenges.remove(id).unwrap();
            challenges.insert(id.clone(), challenge);
        }
        old.challenges.flush();

        let treasury_ids: Vec<AccountId> = old.treasury.keys().cloned().collect();
        let mut treasury = IterableMap::new(StorageKey::Treasury);
        for id in &treasury_ids {
            let amount = old.treasury.remove(id).unwrap();
            treasury.insert(id.clone(), amount);
        }
        old.treasury.flush();

        let bet_ids: Vec<BetId> = old.bets.keys().cloned().collect();
        let mut bets = IterableMap::new(StorageKey::Bets);
        for id in &bet_ids {
            let old_bet = old.bets.remove(id).unwrap();
            let storage_key: Vec<u8> = [
                borsh::to_vec(&StorageKey::V9BetsInner).unwrap().as_slice(),
                &id.get_storage_key(),
            ]
            .concat();
            let bet = Bets::migrate_from(old_bet, storage_key);
            bets.insert(id.clone(), bet);
        }
        old.bets.flush();

        Self {
            owner_id,
            accounts,
            games,
            challenges,
            treasury,
            fees: Lazy::new(StorageKey::Fees, old.fees.get().treasury),
            token_whitelist: old.token_whitelist,
            bets,
            bettor_active_bets: IterableMap::new(StorageKey::BettorActiveBets),
            is_running: old.is_running,
            points_total_supply: old.points_total_supply,
        }
    }

    fn assert_owner(&self) {
        let signer = env::signer_account_id();
        require!(
            signer == self.owner_id || signer == env::current_account_id(),
            "Only the owner can call this method"
        );
    }

    pub fn pause(&mut self) {
        self.assert_owner();
        self.is_running = false;
    }

    #[payable]
    #[handle_result]
    pub fn set_is_agent(&mut self, is_agent: bool) -> Result<(), ContractError> {
        require!(self.is_running, "Contract is paused");
        assert_one_yocto();
        let account_id = env::signer_account_id();
        let account = self
            .accounts
            .get_mut(&account_id)
            .ok_or_else(|| ContractError::AccountNotRegistered(account_id.clone()))?;
        account.set_is_agent(is_agent);
        Ok(())
    }

    pub fn resume(&mut self) {
        self.assert_owner();
        self.is_running = true;
    }

    pub fn set_fees(&mut self, treasury: u16) {
        self.assert_owner();
        require!(treasury <= 10_000, "Treasury fee cannot exceed 100%");
        self.fees.set(treasury);
    }

    pub fn set_token_whitelist(&mut self, whitelist: Vec<AccountId>) {
        self.assert_owner();
        self.token_whitelist.set(whitelist);
    }

    #[handle_result]
    pub fn withdraw_treasury(
        &mut self,
        token_id: AccountId,
    ) -> Result<PromiseOrValue<()>, ContractError> {
        self.assert_owner();
        let amount = self.treasury.remove(&token_id).unwrap_or(0);
        if amount == 0 {
            return Ok(PromiseOrValue::Value(()));
        }
        Ok(PromiseOrValue::Promise(
            ext_ft_core::ext(token_id.clone())
                .with_attached_deposit(ONE_YOCTO)
                .with_static_gas(FT_TRANSFER_GAS)
                .ft_transfer(
                    self.owner_id.clone(),
                    amount.into(),
                    Some("treasury withdrawal".to_string()),
                )
                .then(
                    Self::ext(env::current_account_id())
                        .with_static_gas(WITHDRAW_CALLBACK_GAS)
                        .withdraw_treasury_callback(token_id, amount),
                ),
        ))
    }

    #[private]
    #[allow(deprecated)]
    pub fn withdraw_treasury_callback(&mut self, token_id: AccountId, amount: u128) {
        if !matches!(env::promise_result(0), PromiseResult::Successful(_)) {
            if let Some(treasury) = self.treasury.get_mut(&token_id) {
                *treasury += amount;
            } else {
                self.treasury.insert(token_id, amount);
            }
        }
    }

    pub fn transfer_ownership(&mut self, new_owner_id: AccountId) {
        self.assert_owner();
        self.owner_id = new_owner_id;
    }

    /// Create a new game against an AI player.
    ///
    /// Returns game ID.
    /// There can only ever be 10 open games due to storage limitations.
    #[handle_result]
    pub fn create_ai_game(&mut self, difficulty: Difficulty) -> Result<GameId, ContractError> {
        require!(self.is_running, "Contract is paused");
        let account_id = env::signer_account_id();
        let account = self
            .accounts
            .get_mut(&account_id)
            .ok_or_else(|| ContractError::AccountNotRegistered(account_id.clone()))?;

        let game = Game::new(
            Player::Human(account_id),
            Player::Ai(difficulty),
            None,
            false,
        );
        let game_id = game.get_game_id().clone();

        account.add_game_id(game_id.clone())?;

        let event = ChessEvent::CreateGame {
            game_id: game_id.clone(),
            white: game.get_white().clone(),
            black: game.get_black().clone(),
            board: game.get_board_state(),
        };
        event.emit();
        self.games.insert(game_id.clone(), game);

        Ok(game_id)
    }

    /// Challenges a player to a non-money match.
    ///
    /// Returns game ID.
    /// There can only ever be 10 open games due to storage limitations.
    #[handle_result]
    pub fn challenge(&mut self, challenged_id: AccountId) -> Result<(), ContractError> {
        require!(self.is_running, "Contract is paused");
        let challenger_id = env::signer_account_id();
        if challenger_id == challenged_id {
            return Err(ContractError::SelfChallenge);
        }
        self.internal_challenge(challenger_id, challenged_id, None)
    }

    /// Accepts a challenge.
    ///
    /// Only works on non-money matches. Otherwise `ft_transfer_call` needs to be used for the
    /// respective token that is used as wager.
    #[handle_result]
    pub fn accept_challenge(&mut self, challenge_id: ChallengeId) -> Result<GameId, ContractError> {
        require!(self.is_running, "Contract is paused");
        let challenged_id = env::signer_account_id();
        Ok(self
            .internal_accept_challenge(challenged_id, challenge_id, None)?
            .0)
    }

    /// Rejects a challenge.
    #[handle_result]
    pub fn reject_challenge(
        &mut self,
        challenge_id: ChallengeId,
        is_challenger: bool,
    ) -> Result<PromiseOrValue<()>, ContractError> {
        require!(self.is_running, "Contract is paused");
        let challenge = self
            .challenges
            .remove(&challenge_id)
            .ok_or(ContractError::ChallengeNotExists(challenge_id.clone()))?;
        let wager = challenge.check_reject(is_challenger)?;

        let challenger_id = challenge.get_challenger();
        let challenger = self
            .accounts
            .get_mut(challenger_id)
            .ok_or_else(|| ContractError::AccountNotRegistered(challenger_id.clone()))?;
        challenger.reject_challenge(&challenge_id, true)?;

        let challenged_id = challenge.get_challenged();
        let challenged = self
            .accounts
            .get_mut(challenged_id)
            .ok_or_else(|| ContractError::AccountNotRegistered(challenged_id.clone()))?;
        challenged.reject_challenge(&challenge_id, false)?;

        let event = ChessEvent::RejectChallenge { challenge_id };
        event.emit();

        Ok(if let Some((token_id, amount)) = wager {
            PromiseOrValue::Promise(
                ext_ft_core::ext(token_id.clone())
                    .with_attached_deposit(ONE_YOCTO)
                    .with_static_gas(FT_TRANSFER_GAS)
                    .ft_transfer(
                        challenger_id.clone(),
                        amount,
                        Some("wager refund".to_string()),
                    )
                    .then(
                        Self::ext(env::current_account_id())
                            .with_static_gas(REJECT_WAGER_CALLBACK_GAS)
                            .reject_challenge_wager_callback(
                                token_id,
                                challenger_id.clone(),
                                amount.0,
                            ),
                    ),
            )
        } else {
            PromiseOrValue::Value(())
        })
    }

    #[private]
    #[allow(deprecated)]
    pub fn reject_challenge_wager_callback(
        &mut self,
        token_id: AccountId,
        challenger_id: AccountId,
        amount: u128,
    ) {
        if !matches!(env::promise_result(0), PromiseResult::Successful(_)) {
            if let Some(account) = self.accounts.get_mut(&challenger_id) {
                account.add_token(&token_id, amount);
            }
        }
    }

    /// Plays a move.
    ///
    /// Only works, if it is your turn. Panics otherwise.
    #[handle_result]
    pub fn play_move(
        &mut self,
        game_id: GameId,
        mv: MoveStr,
    ) -> Result<(Option<GameOutcome>, [String; 8]), ContractError> {
        require!(self.is_running, "Contract is paused");
        let account_id = env::signer_account_id();

        let mv = Move::parse(mv).map_err(ContractError::MoveParse)?;
        let game = self
            .games
            .get_mut(&game_id)
            .ok_or(ContractError::GameNotExists)?;
        let account = self
            .accounts
            .get_mut(&account_id)
            .ok_or(ContractError::AccountNotRegistered(account_id.clone()))?;
        let points = account.apply_quest(Quest::DailyPlayMove);
        self.points_total_supply += points;

        if !game.is_turn(&account_id) {
            return Err(ContractError::NotYourTurn);
        }

        let move_result = game.play_move(mv)?;

        let (outcome, board) = if let Some((outcome, board_state)) = move_result.0 {
            self.internal_handle_outcome(game_id, &outcome);
            (Some(outcome), board_state)
        } else {
            (None, self.games.get(&game_id).unwrap().get_board_state())
        };

        Ok((outcome, board))
    }

    /// Resigns a game.
    ///
    /// Can be called even if it is not your turn.
    /// You might need to call this if a game is stuck and the AI refuses to work.
    /// You can also only have 10 open games due to storage limitations.
    #[handle_result]
    pub fn resign(&mut self, game_id: GameId) -> Result<GameOutcome, ContractError> {
        require!(self.is_running, "Contract is paused");
        let account_id = env::signer_account_id();
        let game = self
            .games
            .get_mut(&game_id)
            .ok_or(ContractError::GameNotExists)?;

        if !game.is_player(&account_id) {
            return Err(ContractError::NotPlaying);
        }

        let (outcome, resigner) = if let Player::Human(black_id) = game.get_black() {
            if black_id == &account_id {
                (GameOutcome::Victory(Color::White), Color::Black)
            } else {
                (GameOutcome::Victory(Color::Black), Color::White)
            }
        } else {
            (GameOutcome::Victory(Color::Black), Color::White)
        };

        self.internal_handle_outcome(game_id.clone(), &outcome);

        let event = ChessEvent::ResignGame {
            game_id,
            resigner,
            outcome: outcome.clone(),
        };
        event.emit();

        Ok(outcome)
    }

    /// Cancel a game, resulting in no player winning or loosing.
    ///
    /// Players can only cancel a game, if the opponent is human
    /// and hasn't been doing a move for the last approx. 3days (measured in block height)
    #[handle_result]
    pub fn cancel(&mut self, game_id: GameId) -> Result<PromiseOrValue<()>, ContractError> {
        require!(self.is_running, "Contract is paused");
        let account_id = env::signer_account_id();
        let game = self
            .games
            .get_mut(&game_id)
            .ok_or(ContractError::GameNotExists)?;

        if !game.is_player(&account_id) {
            return Err(ContractError::NotPlaying);
        }
        if game.is_turn(&account_id) {
            return Err(ContractError::CancelOnOpponentsTurn);
        }
        if env::block_height() - game.get_last_block_height() < MIN_BLOCK_DIFF_CANCEL {
            return Err(ContractError::GameNotCancellable(
                MIN_BLOCK_DIFF_CANCEL + game.get_last_block_height() - env::block_height(),
            ));
        }

        let game = self.games.remove(&game_id).unwrap();
        if let Some(account) = game.get_white().as_account_mut(self) {
            account.remove_game_id(&game_id);
        }
        if let Some(account) = game.get_black().as_account_mut(self) {
            account.remove_game_id(&game_id);
        }

        if game.has_bets() {
            let players = (
                game.get_white().get_account_id().unwrap(),
                game.get_black().get_account_id().unwrap(),
            );
            let bet_id = BetId::new(players.clone()).unwrap();
            if let Some(all_bets) = self.bets.remove(&bet_id) {
                let resolved_bettors: HashSet<_> = all_bets
                    .bets
                    .iter()
                    .flat_map(|(_, bet_list)| bet_list.iter().map(|(id, _)| id.clone()))
                    .collect();
                for account_id in &resolved_bettors {
                    if let Some(active) = self.bettor_active_bets.get_mut(account_id) {
                        *active = active.saturating_sub(1);
                    }
                }
                for (token_id, bets) in all_bets.bets.iter() {
                    for (account_id, bet) in bets {
                        self.accounts
                            .get_mut(account_id)
                            .unwrap()
                            .add_token(token_id, bet.amount);
                    }
                }
            }
        }

        let event = ChessEvent::CancelGame {
            game_id,
            cancelled_by: account_id,
        };
        event.emit();

        Ok(if let Some((token_id, amount)) = game.get_wager().clone() {
            let white_id = game.get_white().get_account_id().unwrap();
            let black_id = game.get_black().get_account_id().unwrap();

            PromiseOrValue::Promise(
                ext_ft_core::ext(token_id.clone())
                    .with_attached_deposit(ONE_YOCTO)
                    .with_static_gas(FT_TRANSFER_GAS)
                    .ft_transfer(white_id.clone(), amount, Some("wager refund".to_string()))
                    .and(
                        ext_ft_core::ext(token_id.clone())
                            .with_attached_deposit(ONE_YOCTO)
                            .with_static_gas(FT_TRANSFER_GAS)
                            .ft_transfer(
                                black_id.clone(),
                                amount,
                                Some("wager refund".to_string()),
                            ),
                    )
                    .then(
                        Self::ext(env::current_account_id())
                            .with_static_gas(CANCEL_WAGER_CALLBACK_GAS)
                            .cancel_wager_refund_callback(token_id, white_id, black_id, amount.0),
                    ),
            )
        } else {
            PromiseOrValue::Value(())
        })
    }

    #[private]
    #[allow(deprecated)]
    pub fn cancel_wager_refund_callback(
        &mut self,
        token_id: AccountId,
        white_id: AccountId,
        black_id: AccountId,
        amount: u128,
    ) {
        for i in 0..2 {
            if !matches!(env::promise_result(i), PromiseResult::Successful(_)) {
                let account_id = if i == 0 { &white_id } else { &black_id };
                if let Some(account) = self.accounts.get_mut(account_id) {
                    account.add_token(&token_id, amount);
                }
            }
        }
    }

    #[handle_result]
    pub fn cancel_bet(
        &mut self,
        players: (AccountId, AccountId),
        token_id: AccountId,
    ) -> Result<(), ContractError> {
        require!(self.is_running, "Contract is paused");
        let sender_id = env::signer_account_id();
        let bet_id = BetId::new(players.clone())?;

        let (should_remove, bet_amount) = {
            let bets = self
                .bets
                .get_mut(&bet_id)
                .ok_or(ContractError::BetNotExists)?;
            if bets.is_locked {
                return Err(ContractError::BetLocked);
            }

            let token_bets = bets
                .bets
                .get_mut(&token_id)
                .ok_or(ContractError::BetNotFound)?;
            let index = token_bets
                .binary_search_by_key(&sender_id, |(id, _)| id.clone())
                .map_err(|_| ContractError::BetNotFound)?;
            let (_, removed_bet) = token_bets.remove(index);

            self.accounts
                .get_mut(&sender_id)
                .unwrap()
                .add_token(&token_id, removed_bet.amount);

            if token_bets.is_empty() {
                bets.bets.remove(&token_id);
            }
            (bets.bets.is_empty(), removed_bet.amount)
        };
        if should_remove {
            self.bets.remove(&bet_id);
        }

        if let Some(active) = self.bettor_active_bets.get_mut(&sender_id) {
            *active = active.saturating_sub(1);
        }

        let event = ChessEvent::CancelBet {
            bettor: sender_id,
            players,
            token_id,
            amount: bet_amount.into(),
        };
        event.emit();

        Ok(())
    }

    #[handle_result]
    #[payable]
    pub fn withdraw_token(
        &mut self,
        token_id: AccountId,
    ) -> Result<PromiseOrValue<()>, ContractError> {
        require!(self.is_running, "Contract is paused");
        assert_one_yocto();

        let signer_id = env::signer_account_id();
        let account = self.internal_get_account_mut(&signer_id)?;
        let amount = account.withdraw_token(&token_id);

        Ok(if amount == 0 {
            PromiseOrValue::Value(())
        } else {
            PromiseOrValue::Promise(
                ext_ft_core::ext(token_id)
                    .with_static_gas(FT_TRANSFER_GAS)
                    .with_attached_deposit(ONE_YOCTO)
                    .ft_transfer(signer_id, amount.into(), Some("withdraw".to_string())),
            )
        })
    }
}
