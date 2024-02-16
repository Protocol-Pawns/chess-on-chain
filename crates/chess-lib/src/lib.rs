mod account;
mod bet;
mod challenge;
mod elo;
mod error;
mod event;
mod ft_receiver;
mod game;
mod iah;
mod internal;
mod old;
mod points;
mod social;
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
pub use iah::*;
use old::*;
pub use points::*;
pub use social::*;
pub use storage::*;

use chess_engine::{Color, Move};
use maplit::hashmap;
use near_contract_standards::fungible_token::core::ext_ft_core;
use near_sdk::{
    assert_one_yocto,
    borsh::{BorshDeserialize, BorshSerialize},
    env,
    json_types::U128,
    near_bindgen,
    serde::{Deserialize, Serialize},
    store::{Lazy, UnorderedMap},
    AccountId, BorshStorageKey, Gas, GasWeight, NearToken, PanicOnDefault, PromiseOrValue,
};
use std::collections::{HashMap, VecDeque};

pub const MAX_OPEN_GAMES: u32 = 10;
pub const MAX_OPEN_CHALLENGES: u32 = 50;

#[cfg(not(feature = "integration-test"))]
pub const MIN_BLOCK_DIFF_CANCEL: u64 = 60 * 60 * 24 * 3; // ~3 days
#[cfg(feature = "integration-test")]
pub const MIN_BLOCK_DIFF_CANCEL: u64 = 100;

pub const GAS_FOR_SOCIAL_NOTIFY_CALL: Gas = Gas::from_tgas(20);
pub const GAS_FOR_IS_HUMAN_CALL: Gas = Gas::from_tgas(12);

pub const NO_DEPOSIT: NearToken = NearToken::from_yoctonear(0);
pub const ONE_YOCTO: NearToken = NearToken::from_yoctonear(1);

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
    AccountTokens,
}

#[near_bindgen]
#[derive(BorshDeserialize, BorshSerialize, PanicOnDefault)]
#[borsh(crate = "near_sdk::borsh")]
pub struct Chess {
    pub social_db: AccountId,
    pub iah_registry: AccountId,
    pub accounts: UnorderedMap<AccountId, Account>,
    pub games: UnorderedMap<GameId, Game>,
    pub challenges: UnorderedMap<ChallengeId, Challenge>,
    pub recent_finished_games: Lazy<VecDeque<GameId>>,
    pub treasury: UnorderedMap<AccountId, u128>,
    pub fees: Lazy<Fees>,
    pub token_whitelist: Lazy<Vec<AccountId>>,
    pub bets: UnorderedMap<BetId, Bets>,
}

#[derive(
    Clone,
    Debug,
    PartialEq,
    Eq,
    BorshDeserialize,
    BorshSerialize,
    PanicOnDefault,
    Deserialize,
    Serialize,
)]
#[serde(crate = "near_sdk::serde")]
#[borsh(crate = "near_sdk::borsh")]
pub struct Fees {
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
    pub fn new(social_db: AccountId, iah_registry: AccountId) -> Result<Self, ContractError> {
        if env::state_exists() {
            return Err(ContractError::AlreadyInitilized);
        }
        Ok(Self {
            social_db,
            iah_registry,
            accounts: UnorderedMap::new(StorageKey::VAccounts),
            games: UnorderedMap::new(StorageKey::Games),
            challenges: UnorderedMap::new(StorageKey::Challenges),
            recent_finished_games: Lazy::new(StorageKey::RecentFinishedGamesV2, VecDeque::new()),
            treasury: UnorderedMap::new(StorageKey::Treasury),
            fees: Lazy::new(
                StorageKey::Fees,
                Fees {
                    treasury: 0,
                    royalties: Vec::new(),
                },
            ),
            token_whitelist: Lazy::new(StorageKey::TokenWhitelist, Vec::new()),
            bets: UnorderedMap::new(StorageKey::Bets),
        })
    }

    #[private]
    #[init(ignore_state)]
    pub fn migrate() -> Self {
        let mut chess: OldChess = env::state_read().unwrap();

        let mut accounts = vec![];
        for (account_id, account) in chess.accounts.drain() {
            accounts.push((account_id, account.migrate()));
        }
        for (account_id, account) in accounts {
            chess.accounts.insert(account_id, account);
        }

        let mut games = vec![];
        for (game_id, game) in chess.games.drain() {
            games.push((game_id, game.migrate()));
        }
        for (game_id, game) in games {
            chess.games.insert(game_id, game);
        }

        Self {
            social_db: chess.social_db,
            iah_registry: chess.iah_registry,
            accounts: chess.accounts,
            games: chess.games,
            challenges: chess.challenges,
            recent_finished_games: chess.recent_finished_games,
            treasury: chess.treasury,
            fees: chess.fees,
            token_whitelist: chess.wager_whitelist,
            bets: UnorderedMap::new(StorageKey::Bets),
        }
    }

    #[private]
    pub fn clear_all_games(&mut self) {
        for (game_id, game) in self.games.drain() {
            let Player::Human(account_id) = game.get_white() else {
                panic!()
            };
            let account = self.accounts.get_mut(account_id).unwrap();
            account.remove_game_id(&game_id);
        }
    }

    #[private]
    pub fn set_fees(&mut self, treasury: u16, royalties: Vec<(AccountId, u16)>) {
        self.fees.set(Fees {
            treasury,
            royalties,
        });
    }

    #[private]
    pub fn set_wager_whitelist(&mut self, whitelist: Vec<AccountId>) {
        self.token_whitelist.set(whitelist);
    }

    #[payable]
    #[handle_result]
    pub fn register_token(
        &mut self,
        token_id: AccountId,
        amount: U128,
    ) -> Result<(), ContractError> {
        let fees = self.fees.get();

        let actual_deposit = env::attached_deposit().as_yoctonear();
        let expected_deposit = (1 + fees.royalties.len() as u128) * amount.0;
        if expected_deposit < actual_deposit {
            return Err(ContractError::NotEnoughDeposit(
                actual_deposit,
                expected_deposit,
            ));
        }

        let amount = NearToken::from_yoctonear(amount.0);
        let promise_index = env::promise_batch_create(&token_id);
        env::promise_batch_action_function_call_weight(
            promise_index,
            "storage_deposit",
            serde_json::json!({
                "account_id": env::current_account_id(),
                "registration_only": true
            })
            .to_string()
            .as_bytes(),
            amount,
            Gas::from_tgas(0),
            GasWeight::default(),
        );
        for (account_id, _) in &fees.royalties {
            env::promise_batch_action_function_call_weight(
                promise_index,
                "storage_deposit",
                serde_json::json!({
                    "account_id": account_id,
                    "registration_only": true
                })
                .to_string()
                .as_bytes(),
                amount,
                Gas::from_tgas(0),
                GasWeight::default(),
            );
        }
        env::promise_return(promise_index);

        Ok(())
    }

    /// Create a new game against an AI player.
    ///
    /// Returns game ID.
    /// There can only ever be 10 open games due to storage limitations.
    #[handle_result]
    pub fn create_ai_game(&mut self, difficulty: Difficulty) -> Result<GameId, ContractError> {
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

        if !is_challenger {
            self.internal_send_notify(hashmap! {
                challenger_id.clone() => vec![ChessNotification::RejectedChallenge {
                    challenged_id: challenged_id.clone(),
                }]
            });
        }

        Ok(if let Some((token_id, amount)) = wager {
            PromiseOrValue::Promise(
                ext_ft_core::ext(token_id)
                    .with_attached_deposit(ONE_YOCTO)
                    .with_unused_gas_weight(1)
                    .ft_transfer(
                        challenger_id.clone(),
                        amount,
                        Some("wager refund".to_string()),
                    ),
            )
        } else {
            PromiseOrValue::Value(())
        })
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
        account.apply_quest(Quest::DailyPlayMove);

        if !game.is_turn(&account_id) {
            return Err(ContractError::NotYourTurn);
        }

        let move_result = game.play_move(mv)?;

        let mut notifications = HashMap::new();
        let player = match move_result.1 {
            Color::White => game.get_white(),
            Color::Black => game.get_black(),
        };

        if game.get_black().is_human() {
            if let Player::Human(account_id) = player.clone() {
                notifications.insert(
                    account_id,
                    vec![ChessNotification::YourTurn {
                        game_id: game_id.clone(),
                    }],
                );
            }
        }

        let (outcome, board) = if let Some((outcome, board_state)) = move_result.0 {
            self.internal_handle_outcome(game_id, &outcome, &mut notifications);
            (Some(outcome), board_state)
        } else {
            (None, self.games.get(&game_id).unwrap().get_board_state())
        };

        self.internal_send_notify(notifications);

        Ok((outcome, board))
    }

    /// Resigns a game.
    ///
    /// Can be called even if it is not your turn.
    /// You might need to call this if a game is stuck and the AI refuses to work.
    /// You can also only have 10 open games due to storage limitations.
    #[handle_result]
    pub fn resign(&mut self, game_id: GameId) -> Result<GameOutcome, ContractError> {
        let account_id = env::signer_account_id();
        let game = self
            .games
            .get_mut(&game_id)
            .ok_or(ContractError::GameNotExists)?;

        if !game.is_player(&account_id) {
            return Err(ContractError::NotPlaying);
        }

        let outcome = if let Player::Human(black_id) = game.get_black() {
            if black_id == &account_id {
                GameOutcome::Victory(Color::White)
            } else {
                GameOutcome::Victory(Color::Black)
            }
        } else {
            GameOutcome::Victory(Color::Black)
        };
        let board_state = Game::_get_board_state(game.get_board());

        let mut notifications = HashMap::new();
        self.internal_handle_outcome(game_id.clone(), &outcome, &mut notifications);
        self.internal_send_notify(notifications);

        let event = ChessEvent::ResignGame {
            game_id: game_id.clone(),
            resigner: account_id,
        };
        event.emit();

        let event = ChessEvent::FinishGame {
            game_id,
            outcome: outcome.clone(),
            board: board_state.clone(),
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

        let event = ChessEvent::CancelGame {
            game_id,
            cancelled_by: account_id,
        };
        event.emit();

        Ok(if let Some((token_id, amount)) = game.get_wager().clone() {
            PromiseOrValue::Promise(
                ext_ft_core::ext(token_id.clone())
                    .with_attached_deposit(ONE_YOCTO)
                    .with_unused_gas_weight(1)
                    .ft_transfer(
                        game.get_white().get_account_id().unwrap(),
                        amount,
                        Some("wager refund".to_string()),
                    )
                    .then(
                        ext_ft_core::ext(token_id)
                            .with_attached_deposit(ONE_YOCTO)
                            .with_unused_gas_weight(1)
                            .ft_transfer(
                                game.get_black().get_account_id().unwrap(),
                                amount,
                                Some("wager refund".to_string()),
                            ),
                    ),
            )
        } else {
            PromiseOrValue::Value(())
        })
    }

    #[handle_result]
    #[payable]
    pub fn withdraw_token(
        &mut self,
        token_id: AccountId,
    ) -> Result<PromiseOrValue<()>, ContractError> {
        assert_one_yocto();

        let signer_id = env::signer_account_id();
        let account = self.internal_get_account_mut(&signer_id)?;
        let amount = account.withdraw_token(&token_id);

        Ok(if amount == 0 {
            PromiseOrValue::Value(())
        } else {
            PromiseOrValue::Promise(
                ext_ft_core::ext(token_id)
                    .with_unused_gas_weight(1)
                    .with_attached_deposit(NearToken::from_yoctonear(1))
                    .ft_transfer(signer_id, amount.into(), Some("withdraw".to_string())),
            )
        })
    }
}
