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

    pub fn create_ai_game(&mut self, difficulty: Difficulty) -> GameId {
        let account_id = env::signer_account_id();
        let block_height = env::block_height();
        let game = Game::new(Player::Human(account_id.clone()), Player::Ai(difficulty));
        let game_id = GameId(block_height, account_id, None);
        self.games.insert(game_id.clone(), game);
        game_id
    }

    #[handle_result]
    pub fn play_move(
        &mut self,
        game_id: GameId,
        mv: String,
    ) -> Result<(Option<GameOutcome>, String), ContractError> {
        let game = self
            .games
            .get_mut(&game_id)
            .ok_or(ContractError::GameNotExists)?;
        let mv = Move::parse(mv).map_err(ContractError::MoveParse)?;

        let account_id = env::signer_account_id();
        if !game.is_turn(account_id) {
            return Err(ContractError::NotYourTurn);
        }

        let outcome = if let Some(outcome) = game.play_move(mv)? {
            self.games.remove(&game_id);
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
