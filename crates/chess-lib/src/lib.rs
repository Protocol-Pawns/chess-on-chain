mod account;
mod challenge;
mod elo;
mod error;
mod event;
mod game;
mod storage;
mod view;

pub use account::*;
pub use challenge::*;
pub use elo::*;
pub use error::*;
pub use event::*;
pub use game::*;
pub use storage::*;
pub use view::*;

use chess_engine::Move;
use near_contract_standards::fungible_token::receiver::FungibleTokenReceiver;
use near_sdk::{
    borsh::{self, BorshDeserialize, BorshSerialize},
    env,
    json_types::U128,
    near_bindgen,
    serde::{Deserialize, Serialize},
    store::{Lazy, UnorderedMap},
    AccountId, Balance, BorshStorageKey, PanicOnDefault, PromiseOrValue,
};
use std::collections::VecDeque;
use witgen::witgen;

#[derive(BorshStorageKey, BorshSerialize)]
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
}

#[near_bindgen]
#[derive(BorshDeserialize, BorshSerialize, PanicOnDefault)]
pub struct Chess {
    pub accounts: UnorderedMap<AccountId, Account>,
    pub games: UnorderedMap<GameId, Game>,
    pub challenges: UnorderedMap<ChallengeId, Challenge>,
    pub recent_finished_games: Lazy<VecDeque<GameId>>,
}

#[near_bindgen]
#[derive(BorshDeserialize, BorshSerialize, PanicOnDefault)]
pub struct OldChess {
    pub accounts: UnorderedMap<AccountId, Account>,
    pub games: UnorderedMap<GameId, Game>,
    pub recent_finished_games: Lazy<VecDeque<GameId>>,
}

/// A valid move will be parsed from a string.
///
/// Possible [valid formats](https://docs.rs/chess-engine/latest/chess_engine/enum.Move.html#method.parse) include:
/// - "e2e4"
/// - "e2 e4"
/// - "e2 to e4"
/// - "castle queenside"
/// - "castle kingside"
#[witgen]
pub type MoveStr = String;

#[near_bindgen]
impl Chess {
    #[init]
    #[handle_result]
    pub fn new() -> Result<Self, ContractError> {
        if env::state_exists() {
            return Err(ContractError::AlreadyInitilized);
        }
        Ok(Self {
            accounts: UnorderedMap::new(StorageKey::VAccounts),
            games: UnorderedMap::new(StorageKey::Games),
            challenges: UnorderedMap::new(StorageKey::Challenges),
            recent_finished_games: Lazy::new(StorageKey::RecentFinishedGamesV2, VecDeque::new()),
        })
    }

    #[private]
    #[init(ignore_state)]
    pub fn migrate() -> Self {
        let mut chess: Chess = env::state_read().unwrap();

        let mut accounts = vec![];
        for (account_id, account) in chess.accounts.drain() {
            accounts.push((account_id, account.migrate()));
        }
        for (account_id, account) in accounts {
            chess.accounts.insert(account_id, account);
        }

        Self {
            accounts: chess.accounts,
            games: chess.games,
            challenges: chess.challenges,
            recent_finished_games: chess.recent_finished_games,
        }
    }

    #[private]
    pub fn clear_all_games(&mut self) {
        for (game_id, game) in self.games.drain() {
            let Player::Human(account_id) = game.get_white() else { panic!() };
            let account = self.accounts.get_mut(account_id).unwrap();
            account.remove_game_id(&game_id);
        }
    }

    /// Create a new game against an AI player.
    ///
    /// Returns game ID, which is necessary to play the game.
    /// You can only have 5 open games due to storage limitations.
    /// If you reach the limit you can call `resign` method.
    ///
    /// Before you can play a game you need to pay `storage_deposit`.
    #[handle_result]
    pub fn create_ai_game(&mut self, difficulty: Difficulty) -> Result<GameId, ContractError> {
        let account_id = env::signer_account_id();
        let account = self
            .accounts
            .get_mut(&account_id)
            .ok_or_else(|| ContractError::AccountNotRegistered(account_id.clone()))?;

        let game = Game::new(Player::Human(account_id), Player::Ai(difficulty));
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
    #[handle_result]
    pub fn challenge(&mut self, challenged_id: AccountId) -> Result<(), ContractError> {
        let challenger_id = env::signer_account_id();
        self.internal_challenge(challenger_id, challenged_id, None)
    }

    /// Accepts a challenge.
    ///
    /// Only works on non-money matches. Otherwise `ft_transfer_call` needs to be used for the
    /// respective token that is used as wager.
    #[handle_result]
    pub fn accept_challenge(&mut self, challenge_id: ChallengeId) -> Result<GameId, ContractError> {
        let challenged_id = env::signer_account_id();
        self.internal_accept_challenge(challenged_id, challenge_id, &None)
    }

    /// Rejects a challenge.
    #[handle_result]
    pub fn reject_challenge(
        &mut self,
        challenge_id: ChallengeId,
        is_challenger: bool,
    ) -> Result<(), ContractError> {
        let challenge = self
            .challenges
            .remove(&challenge_id)
            .ok_or(ContractError::ChallengeNotExists(challenge_id.clone()))?;
        challenge.check_reject(is_challenger)?;

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

        Ok(())
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

        let game = self
            .games
            .get_mut(&game_id)
            .ok_or(ContractError::GameNotExists)?;
        let mv = Move::parse(mv).map_err(ContractError::MoveParse)?;

        if !game.is_turn(&account_id) {
            return Err(ContractError::NotYourTurn);
        }

        let (outcome, board) = if let Some((outcome, board_state)) = game.play_move(mv)? {
            let game = self.games.remove(&game_id).unwrap();
            if let Some(account) = game.get_white().as_account_mut(self) {
                account.remove_game_id(&game_id);
                account.save_finished_game(game_id.clone());
            }
            if let Some(account) = game.get_black().as_account_mut(self) {
                account.remove_game_id(&game_id);
                account.save_finished_game(game_id.clone());
            }
            let recent_finished_games = self.recent_finished_games.get_mut();
            recent_finished_games.push_front(game_id);
            if recent_finished_games.len() > 100 {
                recent_finished_games.pop_back();
            }
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
    /// You can also only have 5 open games due to storage limitations.
    #[handle_result]
    pub fn resign(&mut self, game_id: GameId) -> Result<(), ContractError> {
        let account_id = env::signer_account_id();
        let account = self
            .accounts
            .get_mut(&account_id)
            .ok_or_else(|| ContractError::AccountNotRegistered(account_id.clone()))?;

        if let Some(game) = self.games.get_mut(&game_id) {
            if !game.is_player(&account_id) {
                return Err(ContractError::NotPlaying);
            }
            self.games.remove(&game_id);
            account.remove_game_id(&game_id);

            let event = ChessEvent::ResignGame {
                game_id,
                resigner: account_id,
            };
            event.emit();
        } else {
            account.remove_game_id(&game_id);
        }

        Ok(())
    }
}

#[derive(Debug, Deserialize, Serialize)]
#[serde(crate = "near_sdk::serde")]
#[witgen]
pub enum FtReceiverMsg {
    Challenge(ChallengeMsg),
    AcceptChallenge(AcceptChallengeMsg),
}

#[derive(Debug, Deserialize, Serialize)]
#[serde(crate = "near_sdk::serde")]
#[witgen]
pub struct ChallengeMsg {
    pub challenged_id: AccountId,
}

#[derive(Debug, Deserialize, Serialize)]
#[serde(crate = "near_sdk::serde")]
#[witgen]
pub struct AcceptChallengeMsg {
    pub challenge_id: ChallengeId,
}

#[near_bindgen]
impl FungibleTokenReceiver for Chess {
    fn ft_on_transfer(
        &mut self,
        sender_id: AccountId,
        amount: U128,
        msg: String,
    ) -> PromiseOrValue<U128> {
        match self.internal_ft_on_transfer(sender_id, amount, msg) {
            Ok(res) => res,
            Err(err) => {
                panic!("{}", err);
            }
        }
    }
}

impl Chess {
    fn internal_ft_on_transfer(
        &mut self,
        sender_id: AccountId,
        amount: U128,
        msg: String,
    ) -> Result<PromiseOrValue<U128>, ContractError> {
        let msg = serde_json::from_str(&msg).map_err(|_| ContractError::Deserialize)?;

        match msg {
            FtReceiverMsg::Challenge(ChallengeMsg { challenged_id }) => {
                let challenger_id = sender_id;
                let token_id = env::predecessor_account_id();
                self.internal_challenge(challenger_id, challenged_id, Some((token_id, amount)))?;
            }
            FtReceiverMsg::AcceptChallenge(AcceptChallengeMsg { challenge_id }) => {
                let challenged_id = sender_id;
                let token_id = env::predecessor_account_id();
                self.internal_accept_challenge(
                    challenged_id,
                    challenge_id,
                    &Some((token_id, amount)),
                )?;
            }
        }

        Ok(PromiseOrValue::Value(0.into()))
    }

    fn internal_challenge(
        &mut self,
        challenger_id: AccountId,
        challenged_id: AccountId,
        wager: Wager,
    ) -> Result<(), ContractError> {
        let challenge: Challenge =
            Challenge::new(challenger_id.clone(), challenged_id.clone(), wager);

        let challenger = self
            .accounts
            .get_mut(&challenger_id)
            .ok_or_else(|| ContractError::AccountNotRegistered(challenger_id.clone()))?;
        challenger.add_challenge(challenge.id().clone(), true);

        let challenged = self
            .accounts
            .get_mut(&challenged_id)
            .ok_or_else(|| ContractError::AccountNotRegistered(challenged_id.clone()))?;
        challenged.add_challenge(challenge.id().clone(), false);

        self.challenges
            .insert(challenge.id().clone(), challenge.clone());

        let event = ChessEvent::Challenge(challenge);
        event.emit();

        Ok(())
    }

    fn internal_accept_challenge(
        &mut self,
        challenged_id: AccountId,
        challenge_id: ChallengeId,
        paid_wager: &Wager,
    ) -> Result<GameId, ContractError> {
        let challenged = self
            .accounts
            .get_mut(&challenged_id)
            .ok_or_else(|| ContractError::AccountNotRegistered(challenged_id.clone()))?;

        let challenge = self
            .challenges
            .remove(&challenge_id)
            .ok_or(ContractError::ChallengeNotExists(challenge_id.clone()))?;
        challenge.check_accept(&challenged_id, paid_wager)?;

        let challenger_id = challenge.get_challenger();
        let game = Game::new(
            Player::Human(challenger_id.clone()),
            Player::Human(challenged_id),
        );
        let game_id = game.get_game_id().clone();

        challenged.accept_challenge(&challenge_id, game_id.clone(), false)?;
        let challenger = self
            .accounts
            .get_mut(challenger_id)
            .ok_or_else(|| ContractError::AccountNotRegistered(challenger_id.clone()))?;
        challenger.accept_challenge(&challenge_id, game_id.clone(), true)?;

        let event = ChessEvent::AcceptChallenge {
            challenge_id,
            game_id: game_id.clone(),
        };
        event.emit();
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

    pub(crate) fn internal_get_account(
        &self,
        account_id: &AccountId,
    ) -> Result<&Account, ContractError> {
        self.accounts
            .get(account_id)
            .ok_or_else(|| ContractError::AccountNotRegistered(account_id.clone()))
    }

    pub(crate) fn internal_register_account(&mut self, account_id: AccountId, amount: Balance) {
        let account = Account::new(account_id.clone(), amount);
        self.accounts.insert(account_id, account);
    }
}
