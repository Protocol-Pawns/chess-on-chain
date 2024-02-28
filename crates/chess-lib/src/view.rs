use crate::{
    AccountInfo, Achievement, AchievementInfo, BetId, BetInfo, Challenge, ChallengeId, Chess,
    ChessExt, ContractError, EloRating, Fees, GameId, GameInfo, Quest, QuestInfo,
};
use near_sdk::{json_types::U128, near_bindgen, AccountId};
use std::collections::VecDeque;
use strum::IntoEnumIterator;

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
            has_bets: game.has_bets(),
        })
    }

    #[handle_result]
    pub fn bet_info(&self, players: (AccountId, AccountId)) -> Result<BetInfo, ContractError> {
        let bet_id = BetId::new(players)?;
        Ok(self
            .bets
            .get(&bet_id)
            .ok_or(ContractError::BetNotExists)?
            .into())
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

    #[handle_result]
    pub fn get_account(&self, account_id: AccountId) -> Result<AccountInfo, ContractError> {
        let account = self
            .accounts
            .get(&account_id)
            .ok_or_else(|| ContractError::AccountNotRegistered(account_id.clone()))?;
        Ok(account.into())
    }

    pub fn get_quest_list(&self) -> Vec<QuestInfo> {
        Quest::iter().map(QuestInfo::from).collect()
    }

    #[handle_result]
    pub fn get_quest_cooldowns(
        &self,
        account_id: AccountId,
    ) -> Result<&VecDeque<(u64, Quest)>, ContractError> {
        let account = self
            .accounts
            .get(&account_id)
            .ok_or_else(|| ContractError::AccountNotRegistered(account_id.clone()))?;
        Ok(account.get_quest_cooldowns())
    }

    pub fn get_achievement_list(&self) -> Vec<AchievementInfo> {
        Achievement::iter().map(AchievementInfo::from).collect()
    }

    #[handle_result]
    pub fn get_achievements(
        &self,
        account_id: AccountId,
    ) -> Result<&Vec<(u64, Achievement)>, ContractError> {
        let account = self
            .accounts
            .get(&account_id)
            .ok_or_else(|| ContractError::AccountNotRegistered(account_id.clone()))?;
        Ok(account.get_achievements())
    }

    #[handle_result]
    pub fn get_tokens(
        &mut self,
        account_id: AccountId,
    ) -> Result<Vec<(AccountId, U128)>, ContractError> {
        let account = self.internal_get_account(&account_id)?;
        Ok(account
            .get_tokens()
            .into_iter()
            .map(|(token_id, balance)| (token_id, balance.into()))
            .collect())
    }

    #[handle_result]
    pub fn get_token_amount(
        &mut self,
        account_id: AccountId,
        token_id: AccountId,
    ) -> Result<U128, ContractError> {
        let account = self.internal_get_account(&account_id)?;
        Ok(account.get_token_amount(&token_id).into())
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

    pub fn get_elo_ratings_by_ids(
        &self,
        account_ids: Vec<AccountId>,
    ) -> Vec<(AccountId, EloRating)> {
        account_ids
            .iter()
            .filter_map(|account_id| {
                self.accounts
                    .get(account_id)
                    .and_then(|account| account.get_elo().map(|elo| (account_id.clone(), elo)))
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

    pub fn get_treasury_tokens(&self) -> Vec<(AccountId, U128)> {
        self.treasury
            .iter()
            .map(|(token_id, amount)| (token_id.clone(), (*amount).into()))
            .collect()
    }

    pub fn get_fees(&self) -> Fees {
        self.fees.get().clone()
    }

    pub fn get_token_whitelist(&self) -> Vec<AccountId> {
        self.token_whitelist.get().clone()
    }
}
