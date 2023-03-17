mod error;
mod game;

use chess_engine::Move;
pub use error::*;
pub use game::*;

use near_sdk::{
    borsh::{self, BorshDeserialize, BorshSerialize},
    env, near_bindgen,
    store::UnorderedMap,
    BorshStorageKey, PanicOnDefault,
};

#[derive(BorshStorageKey, BorshSerialize)]
pub enum StorageKey {
    Games,
}

#[near_bindgen]
#[derive(BorshDeserialize, BorshSerialize, PanicOnDefault)]
pub struct Chess {
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
            games: UnorderedMap::new(StorageKey::Games),
        })
    }

    pub fn create_ai_game(&mut self, difficulty: Difficulty) {
        let account_id = env::signer_account_id();
        let block_height = env::block_height();
        let game = Game::new(Player::Human(account_id.clone()), Player::Ai(difficulty));
        self.games
            .insert(GameId(block_height, account_id, None), game);
    }

    #[handle_result]
    pub fn play_move(
        &mut self,
        game_id: GameId,
        mv: String,
    ) -> Result<Option<GameOutcome>, ContractError> {
        let game = self
            .games
            .get_mut(&game_id)
            .ok_or(ContractError::GameNotExists)?;
        let mv = Move::parse(mv).map_err(ContractError::MoveParse)?;

        let outcome = if let Some(outcome) = game.play_move(mv)? {
            self.games.remove(&game_id);
            Some(outcome)
        } else {
            None
        };

        Ok(outcome)
    }
}
