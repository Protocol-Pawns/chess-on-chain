use crate::{
    Challenge, ChallengeId, Chess, ChessExt, ContractError, EloRating, Fees, GameId, GameInfo,
};
use near_sdk::{json_types::U128, near_bindgen, AccountId};

#[near_bindgen]
impl Chess {
    /// Returns an array of strings representing the board
    #[handle_result]
    pub fn get_board(&self, game_id: GameId) -> Result<[String; 8], ContractError> {
        let game = self
            .games
            .get(&game_id)
            .ok_or(ContractError::GameNotExists)?;
        Ok(game.get_board_state())
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
            white: game.get_white().clone(),
            black: game.get_black().clone(),
            turn_color: game.get_board().get_turn_color(),
            last_block_height: game.get_last_block_height(),
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

    /// Returns whether account has been I-Am-Human verified.
    #[handle_result]
    pub fn is_human(&self, account_id: AccountId) -> Result<bool, ContractError> {
        let account = self
            .accounts
            .get(&account_id)
            .ok_or_else(|| ContractError::AccountNotRegistered(account_id.clone()))?;
        Ok(account.is_human())
    }

    /// Returns ELO rating for given wallet ID.
    /// Only I-Am-Human verified accounts have an ELO.
    #[handle_result]
    pub fn get_elo(&self, account_id: AccountId) -> Result<Option<EloRating>, ContractError> {
        let account = self
            .accounts
            .get(&account_id)
            .ok_or_else(|| ContractError::AccountNotRegistered(account_id.clone()))?;
        Ok(account.get_elo())
    }

    pub fn get_elo_ratings(
        &self,
        skip: Option<usize>,
        limit: Option<usize>,
    ) -> Vec<(AccountId, EloRating)> {
        self.accounts
            .iter()
            .skip(skip.unwrap_or_default())
            .take(limit.unwrap_or(100))
            .filter_map(|(account_id, account)| {
                account.get_elo().map(|elo| (account_id.clone(), elo))
            })
            .collect()
    }

    pub fn get_accounts(&self, skip: Option<usize>, limit: Option<usize>) -> Vec<AccountId> {
        self.accounts
            .keys()
            .skip(skip.unwrap_or_default())
            .take(limit.unwrap_or(100))
            .cloned()
            .collect()
    }

    /// Returns info about open challenge.
    #[handle_result]
    pub fn get_challenge(&self, challenge_id: ChallengeId) -> Result<Challenge, ContractError> {
        let challenge = self
            .challenges
            .get(&challenge_id)
            .ok_or_else(|| ContractError::ChallengeNotExists(challenge_id.clone()))?;
        Ok(challenge.clone())
    }

    /// Returns all open challenges.
    #[handle_result]
    pub fn get_challenges(
        &self,
        account_id: AccountId,
        is_challenger: bool,
    ) -> Result<Vec<ChallengeId>, ContractError> {
        let account = self
            .accounts
            .get(&account_id)
            .ok_or_else(|| ContractError::AccountNotRegistered(account_id.clone()))?;
        Ok(account.get_challenges(is_challenger))
    }

    /// Returns game IDs of recently finished games (max 100).
    ///
    /// Output is ordered with newest game ID as first elemtn.
    pub fn recent_finished_games(&self) -> Vec<GameId> {
        self.recent_finished_games.iter().cloned().collect()
    }

    /// Returns game IDs of finished games for given account ID.
    ///
    /// Output is NOT ordered, but client side can do so by looking at block height of game ID (first array entry).
    #[handle_result]
    pub fn finished_games(&self, account_id: AccountId) -> Result<Vec<GameId>, ContractError> {
        let account = self
            .accounts
            .get(&account_id)
            .ok_or_else(|| ContractError::AccountNotRegistered(account_id.clone()))?;
        Ok(account.get_finished_games())
    }

    pub fn get_treasury_tokens(&self) -> Vec<(AccountId, U128)> {
        self.treasury
            .iter()
            .map(|(token_id, amount)| (token_id.clone(), (*amount).into()))
            .collect()
    }

    pub fn get_fees(&self) -> Fees {
        self.fees.get().clone()
    }

    pub fn get_wager_whitelist(&self) -> Vec<AccountId> {
        self.wager_whitelist.get().clone()
    }
}
