mod account;
mod error;
mod game;
mod storage;

pub use account::*;
pub use error::*;
pub use game::*;
pub use storage::*;

use chess_engine::Move;
use near_sdk::{
    borsh::{self, BorshDeserialize, BorshSerialize},
    env, near_bindgen,
    store::UnorderedMap,
    AccountId, Balance, BorshStorageKey, PanicOnDefault,
};
use witgen::witgen;

#[derive(BorshStorageKey, BorshSerialize)]
pub enum StorageKey {
    Accounts,
    AccountOrderIds,
    Games,
}

#[near_bindgen]
#[derive(BorshDeserialize, BorshSerialize, PanicOnDefault)]
pub struct Chess {
    pub accounts: UnorderedMap<AccountId, Account>,
    pub games: UnorderedMap<GameId, Game>,
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
            accounts: UnorderedMap::new(StorageKey::Accounts),
            games: UnorderedMap::new(StorageKey::Games),
        })
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

        let block_height = env::block_height();
        let game_id = GameId(block_height, account_id.clone(), None);
        account.add_game_id(game_id.clone())?;

        let game = Game::new(Player::Human(account_id), Player::Ai(difficulty));
        self.games.insert(game_id.clone(), game);

        Ok(game_id)
    }

    /// Plays a move.
    ///
    /// Only works, if it is your turn. Panics otherwise.
    #[handle_result]
    pub fn play_move(
        &mut self,
        game_id: GameId,
        mv: MoveStr,
    ) -> Result<(Option<GameOutcome>, String), ContractError> {
        let account_id = env::signer_account_id();
        let account = self
            .accounts
            .get_mut(&account_id)
            .ok_or_else(|| ContractError::AccountNotRegistered(account_id.clone()))?;

        let game = self
            .games
            .get_mut(&game_id)
            .ok_or(ContractError::GameNotExists)?;
        let mv = Move::parse(mv).map_err(ContractError::MoveParse)?;

        if !game.is_turn(account_id) {
            return Err(ContractError::NotYourTurn);
        }

        let (outcome, board) = if let Some(outcome) = game.play_move(mv)? {
            let game = self.games.remove(&game_id);
            account.remove_game_id(&game_id);
            (Some(outcome), game.unwrap().render_board())
        } else {
            (None, self.games.get(&game_id).unwrap().render_board())
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

        let game = self
            .games
            .get_mut(&game_id)
            .ok_or(ContractError::GameNotExists)?;
        let account_id = env::signer_account_id();
        if !game.is_player(account_id) {
            return Err(ContractError::NotPlaying);
        }
        self.games.remove(&game_id);
        account.remove_game_id(&game_id);
        Ok(())
    }

    /// Returns an array of strings representing the board
    #[handle_result]
    pub fn get_board(&self, game_id: GameId) -> Result<[String; 8], ContractError> {
        let game = self
            .games
            .get(&game_id)
            .ok_or(ContractError::GameNotExists)?;
        Ok(game.get_board())
    }

    /// Renders a game as a string.
    #[handle_result]
    pub fn render_board(&self, game_id: GameId) -> Result<String, ContractError> {
        let game = self
            .games
            .get(&game_id)
            .ok_or(ContractError::GameNotExists)?;
        Ok(game.render_board())
    }

    /// Returns information about a game including players and turn color.
    #[handle_result]
    pub fn game_info(&self, game_id: GameId) -> Result<GameInfo, ContractError> {
        let game = self
            .games
            .get(&game_id)
            .ok_or(ContractError::GameNotExists)?;

        Ok(GameInfo {
            white: game.white.clone(),
            black: game.black.clone(),
            turn_color: game.board.get_turn_color(),
        })
    }

    /// Returns all open game IDs for given wallet ID.
    #[handle_result]
    pub fn get_game_ids(&self, account_id: AccountId) -> Result<Vec<GameId>, ContractError> {
        let account = self
            .accounts
            .get(&account_id)
            .ok_or_else(|| ContractError::AccountNotRegistered(account_id.clone()))?;
        Ok(account.get_game_ids())
    }
}

impl Chess {
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
