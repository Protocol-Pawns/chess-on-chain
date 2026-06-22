use crate::{
    Chess, ChessEvent, ContractError, Game, GameId, Player, Wager, MATCHMAKING_EXPIRY_NS,
    MAX_MATCHMAKING_QUEUE,
};
use near_sdk::{
    borsh::{BorshDeserialize, BorshSerialize},
    env,
    serde::{Deserialize, Serialize},
    AccountId, NearSchema,
};

/// A pending matchmaking queue entry.
///
/// `min_elo` / `max_elo` describe the range of opponent ratings the player is
/// willing to accept. `wager` is `None` for a non-money match, or a
/// `(token_id, amount)` pair that a matched opponent must match exactly.
#[derive(BorshDeserialize, BorshSerialize, Clone, Debug, Deserialize, Serialize, NearSchema)]
#[serde(crate = "near_sdk::serde")]
#[borsh(crate = "near_sdk::borsh")]
pub struct MatchmakingEntry {
    pub min_elo: f64,
    pub max_elo: f64,
    pub wager: Wager,
    pub joined_timestamp: u64,
}

/// Two wagers are compatible for matchmaking when both are `None` (non-money)
/// or both specify the exact same token and amount.
pub fn wager_compatible(a: &Wager, b: &Wager) -> bool {
    match (a, b) {
        (None, None) => true,
        (Some((token_a, amount_a)), Some((token_b, amount_b))) => {
            token_a == token_b && amount_a == amount_b
        }
        _ => false,
    }
}

impl Chess {
    /// Either matches `joiner_id` with a compatible queued opponent (creating a
    /// game immediately) or adds them to the queue.
    ///
    /// Returns `Some(game_id)` when matched, `None` when queued.
    pub(crate) fn internal_join_matchmaking(
        &mut self,
        joiner_id: AccountId,
        min_elo: f64,
        max_elo: f64,
        wager: Wager,
    ) -> Result<Option<GameId>, ContractError> {
        if min_elo > max_elo {
            return Err(ContractError::InvalidEloRange);
        }
        if self.matchmaking_queue.contains_key(&joiner_id) {
            return Err(ContractError::AlreadyInMatchmaking);
        }

        let (can_add_game, joiner_elo) = {
            let joiner = self
                .accounts
                .get(&joiner_id)
                .ok_or_else(|| ContractError::AccountNotRegistered(joiner_id.clone()))?;
            (joiner.can_add_game(), joiner.get_elo().unwrap_or(1_000.))
        };
        if !can_add_game {
            return Err(ContractError::MaxGamesReached);
        }

        // Scan the queue, lazily purging expired entries and looking for the
        // first fully-compatible opponent.
        let now = env::block_timestamp();
        let mut expired: Vec<AccountId> = Vec::new();
        let mut matched: Option<(AccountId, MatchmakingEntry)> = None;
        for (queued_id, entry) in self.matchmaking_queue.iter() {
            if now.saturating_sub(entry.joined_timestamp) >= MATCHMAKING_EXPIRY_NS {
                expired.push(queued_id.clone());
                continue;
            }
            if queued_id == &joiner_id {
                continue;
            }
            // Skip if there's already an active game between these two players
            let already_playing = self
                .accounts
                .get(&joiner_id)
                .map(|a| a.has_game_with(queued_id))
                .unwrap_or(false);
            if already_playing {
                continue;
            }
            // elo ranges must be mutually acceptable
            let queued_elo = self
                .accounts
                .get(queued_id)
                .and_then(|a| a.get_elo())
                .unwrap_or(1_000.);
            let elo_ok = joiner_elo >= entry.min_elo
                && joiner_elo <= entry.max_elo
                && queued_elo >= min_elo
                && queued_elo <= max_elo;
            if !elo_ok {
                continue;
            }
            if !wager_compatible(&entry.wager, &wager) {
                continue;
            }
            // queued player must still have a free game slot
            let has_room = self
                .accounts
                .get(queued_id)
                .map(|a| a.can_add_game())
                .unwrap_or(false);
            if !has_room {
                continue;
            }
            matched = Some((queued_id.clone(), entry.clone()));
            break;
        }

        // Credit tokens for purged expired entries so players can withdraw.
        for account_id in &expired {
            if let Some(entry) = self.matchmaking_queue.remove(account_id) {
                if let Some((token_id, amount)) = entry.wager {
                    if let Some(account) = self.accounts.get_mut(account_id) {
                        account.add_token(&token_id, amount.0);
                    }
                }
            }
        }

        if let Some((queued_id, entry)) = matched {
            self.matchmaking_queue.remove(&queued_id);
            let game = Game::new(
                Player::Human(queued_id.clone()),
                Player::Human(joiner_id.clone()),
                entry.wager,
                false,
            );
            let game_id = game.get_game_id().clone();
            self.accounts
                .get_mut(&joiner_id)
                .unwrap()
                .add_game_id(game_id.clone())?;
            self.accounts
                .get_mut(&queued_id)
                .unwrap()
                .add_game_id(game_id.clone())?;
            let event = ChessEvent::CreateGame {
                game_id: game_id.clone(),
                white: game.get_white().clone(),
                black: game.get_black().clone(),
                board: game.get_board_state(),
            };
            event.emit();
            self.games.insert(game_id.clone(), game);
            return Ok(Some(game_id));
        }

        // No match found — queue the joiner.
        if (self.matchmaking_queue.len() as u32) >= MAX_MATCHMAKING_QUEUE {
            return Err(ContractError::MatchmakingQueueFull);
        }
        self.matchmaking_queue.insert(
            joiner_id,
            MatchmakingEntry {
                min_elo,
                max_elo,
                wager,
                joined_timestamp: now,
            },
        );
        Ok(None)
    }
}
