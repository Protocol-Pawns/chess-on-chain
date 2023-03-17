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

    #[handle_result]
    pub fn create_ai_game(&mut self, difficulty: Difficulty) -> Result<GameId, ContractError> {
        let account_id = env::signer_account_id();
        let account = self
            .accounts
            .get_mut(&account_id)
            .ok_or_else(|| ContractError::AccountNotRegistered(account_id.clone()))?;

        let block_height = env::block_height();
        let game = Game::new(Player::Human(account_id.clone()), Player::Ai(difficulty));
        let game_id = GameId(block_height, account_id, None);
        self.games.insert(game_id.clone(), game);
        account.add_game_id(game_id.clone());

        Ok(game_id)
    }

    #[handle_result]
    pub fn play_move(
        &mut self,
        game_id: GameId,
        mv: String,
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

        let outcome = if let Some(outcome) = game.play_move(mv)? {
            self.games.remove(&game_id);
            account.remove_game_id(&game_id);
            Some(outcome)
        } else {
            None
        };
        let board = self.games.get(&game_id).unwrap().render_board();

        Ok((outcome, board))
    }

    #[handle_result]
    pub fn resign(&mut self, game_id: GameId) -> Result<(), ContractError> {
        let game = self
            .games
            .get_mut(&game_id)
            .ok_or(ContractError::GameNotExists)?;
        let account_id = env::signer_account_id();
        if !game.is_player(account_id) {
            return Err(ContractError::NotPlaying);
        }
        self.games.remove(&game_id);
        Ok(())
    }

    #[handle_result]
    pub fn render_board(&self, game_id: GameId) -> Result<String, ContractError> {
        let game = self
            .games
            .get(&game_id)
            .ok_or(ContractError::GameNotExists)?;
        Ok(game.render_board())
    }

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
