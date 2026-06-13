use crate::{
    Achievement, ChallengeId, ContractError, EloRating, GameId, Quest, StorageKey,
    MAX_OPEN_CHALLENGES, MAX_OPEN_GAMES,
};
use near_contract_standards::fungible_token::events::FtMint;
use near_sdk::{
    borsh::{self, BorshDeserialize, BorshSerialize},
    env,
    json_types::U128,
    serde::{Deserialize, Serialize},
    store::{IterableMap, IterableSet, Lazy},
    AccountId, NearSchema, NearToken,
};
use std::collections::VecDeque;

#[derive(BorshDeserialize, BorshSerialize)]
#[borsh(crate = "near_sdk::borsh")]
pub enum Account {
    V1(()),
    V2(()),
    V3(()),
    V4(()),
    V5(()),
    V6(()),
    V7(()),
    V8(()),
    V9(AccountV9),
    V10(AccountV10),
}

macro_rules! access_v9_v10 {
    ($self:expr, $var:ident, $body:expr) => {
        match $self {
            Account::V9($var) => $body,
            Account::V10($var) => $body,
            _ => panic!("migration required"),
        }
    };
}

macro_rules! access_v9_v10_mut {
    ($self:expr, $var:ident, $body:expr) => {
        match $self {
            Account::V9(ref mut $var) => $body,
            Account::V10(ref mut $var) => $body,
            _ => panic!("migration required"),
        }
    };
}

#[derive(BorshDeserialize, BorshSerialize)]
#[borsh(crate = "near_sdk::borsh")]
pub struct AccountV9 {
    near_amount: NearToken,
    account_id: AccountId,
    is_agent: bool,
    points: u128,
    elo: Option<EloRating>,
    game_ids: IterableSet<GameId>,
    challenger: IterableSet<ChallengeId>,
    challenged: IterableSet<ChallengeId>,
    quest_cooldowns: Lazy<VecDeque<(u64, Quest)>>,
    achievements: Lazy<Vec<(u64, Achievement)>>,
    tokens: IterableMap<AccountId, u128>,
}

#[derive(BorshDeserialize, BorshSerialize)]
#[borsh(crate = "near_sdk::borsh")]
pub struct AccountV10 {
    near_amount: NearToken,
    account_id: AccountId,
    is_agent: bool,
    points: u128,
    elo: Option<EloRating>,
    game_ids: IterableSet<GameId>,
    challenger: IterableSet<ChallengeId>,
    challenged: IterableSet<ChallengeId>,
    quest_cooldowns: Lazy<VecDeque<(u64, Quest)>>,
    achievements: Lazy<Vec<(u64, Achievement)>>,
    tokens: IterableMap<AccountId, u128>,
    wins: u32,
    win_streak: u32,
    max_win_streak: u32,
    bets_placed: u32,
    bets_won: u32,
    wagers_played: u32,
    wager_wins: u32,
    challenges_sent: u32,
    pending_points: u128,
}

#[derive(Deserialize, Serialize, NearSchema)]
#[serde(crate = "near_sdk::serde")]
pub struct AccountInfo {
    pub near_amount: NearToken,
    pub is_agent: bool,
    pub points: U128,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub elo: Option<EloRating>,
    pub wins: u32,
    pub win_streak: u32,
    pub max_win_streak: u32,
    pub bets_placed: u32,
    pub bets_won: u32,
    pub wagers_played: u32,
    pub wager_wins: u32,
    pub challenges_sent: u32,
    pub pending_points: U128,
}

impl From<&Account> for AccountInfo {
    fn from(account: &Account) -> Self {
        AccountInfo {
            near_amount: account.get_near_amount(),
            is_agent: account.is_agent(),
            points: account.get_points().into(),
            elo: account.get_elo(),
            wins: account.get_wins(),
            win_streak: account.get_win_streak(),
            max_win_streak: account.get_max_win_streak(),
            bets_placed: account.get_bets_placed(),
            bets_won: account.get_bets_won(),
            wagers_played: account.get_wagers_played(),
            wager_wins: account.get_wager_wins(),
            challenges_sent: account.get_challenges_sent(),
            pending_points: account.get_pending_points().into(),
        }
    }
}

impl Account {
    pub fn new(account_id: AccountId, near_amount: NearToken) -> Self {
        let id = env::sha256_array(account_id.as_bytes());
        let game_id_key: Vec<u8> = [
            borsh::to_vec(&StorageKey::VAccounts).unwrap().as_slice(),
            &id,
            borsh::to_vec(&StorageKey::V9AccountOrderIds)
                .unwrap()
                .as_slice(),
        ]
        .concat();
        let challenger_key: Vec<u8> = [
            borsh::to_vec(&StorageKey::VAccounts).unwrap().as_slice(),
            &id,
            borsh::to_vec(&StorageKey::V9AccountChallenger)
                .unwrap()
                .as_slice(),
        ]
        .concat();
        let challenged_key: Vec<u8> = [
            borsh::to_vec(&StorageKey::VAccounts).unwrap().as_slice(),
            &id,
            borsh::to_vec(&StorageKey::V9AccountChallenged)
                .unwrap()
                .as_slice(),
        ]
        .concat();
        let quest_cooldown_key: Vec<u8> = [
            borsh::to_vec(&StorageKey::VAccounts).unwrap().as_slice(),
            &id,
            borsh::to_vec(&StorageKey::AccountQuestCooldowns)
                .unwrap()
                .as_slice(),
        ]
        .concat();
        let achievement_key: Vec<u8> = [
            borsh::to_vec(&StorageKey::VAccounts).unwrap().as_slice(),
            &id,
            borsh::to_vec(&StorageKey::AccountAchievements)
                .unwrap()
                .as_slice(),
        ]
        .concat();
        let tokens_key: Vec<u8> = [
            borsh::to_vec(&StorageKey::VAccounts).unwrap().as_slice(),
            &id,
            borsh::to_vec(&StorageKey::V9AccountTokens)
                .unwrap()
                .as_slice(),
        ]
        .concat();
        Self::V10(AccountV10 {
            account_id,
            near_amount,
            is_agent: false,
            points: 0,
            elo: Some(1_000.),
            game_ids: IterableSet::new(game_id_key),
            challenger: IterableSet::new(challenger_key),
            challenged: IterableSet::new(challenged_key),
            quest_cooldowns: Lazy::new(quest_cooldown_key, VecDeque::new()),
            achievements: Lazy::new(achievement_key, Vec::new()),
            tokens: IterableMap::new(tokens_key),
            wins: 0,
            win_streak: 0,
            max_win_streak: 0,
            bets_placed: 0,
            bets_won: 0,
            wagers_played: 0,
            wager_wins: 0,
            challenges_sent: 0,
            pending_points: 0,
        })
    }

    pub fn migrate(self) -> Self {
        let Account::V9(AccountV9 {
            near_amount,
            account_id,
            is_agent,
            points,
            elo,
            game_ids,
            challenger,
            challenged,
            quest_cooldowns,
            achievements,
            tokens,
        }) = self
        else {
            panic!("migration required");
        };

        Account::V10(AccountV10 {
            near_amount,
            account_id,
            is_agent,
            points,
            elo,
            game_ids,
            challenger,
            challenged,
            quest_cooldowns,
            achievements,
            tokens,
            wins: 0,
            win_streak: 0,
            max_win_streak: 0,
            bets_placed: 0,
            bets_won: 0,
            wagers_played: 0,
            wager_wins: 0,
            challenges_sent: 0,
            pending_points: 0,
        })
    }

    pub fn get_near_amount(&self) -> NearToken {
        access_v9_v10!(self, account, account.near_amount)
    }

    pub fn is_agent(&self) -> bool {
        match self {
            Account::V9(account) => account.is_agent,
            Account::V10(account) => account.is_agent,
            _ => panic!("migration required"),
        }
    }

    pub fn set_is_agent(&mut self, is_agent: bool) {
        access_v9_v10_mut!(self, account, {
            account.is_agent = is_agent;
        });
    }

    pub fn get_elo(&self) -> Option<EloRating> {
        access_v9_v10!(self, account, account.elo)
    }

    pub fn set_elo(&mut self, elo: EloRating) {
        access_v9_v10_mut!(self, account, {
            account.elo = Some(elo);
        })
    }

    pub fn get_points(&self) -> u128 {
        access_v9_v10!(self, account, account.points)
    }

    pub fn get_pending_points(&self) -> u128 {
        match self {
            Account::V10(account) => account.pending_points,
            _ => 0,
        }
    }

    pub fn claim_points(&mut self) -> u128 {
        let Account::V10(account) = self else {
            return 0;
        };
        let amount = account.pending_points;
        if amount > 0 {
            account.points += amount;
            FtMint {
                owner_id: &account.account_id,
                amount: amount.into(),
                memo: Some("claimed_points"),
            }
            .emit();
            account.pending_points = 0;
        }
        amount
    }

    pub fn get_quest_cooldowns(&self) -> &VecDeque<(u64, Quest)> {
        access_v9_v10!(self, account, account.quest_cooldowns.get())
    }

    pub fn get_achievements(&self) -> &Vec<(u64, Achievement)> {
        access_v9_v10!(self, account, account.achievements.get())
    }

    pub fn add_game_id(&mut self, game_id: GameId) -> Result<(), ContractError> {
        access_v9_v10_mut!(self, account, {
            if account.game_ids.len() >= MAX_OPEN_GAMES {
                return Err(ContractError::MaxGamesReached);
            }
            account.game_ids.insert(game_id);
            Ok(())
        })
    }

    pub fn remove_game_id(&mut self, game_id: &GameId) -> bool {
        access_v9_v10_mut!(self, account, account.game_ids.remove(game_id))
    }

    pub fn is_playing(&self) -> bool {
        access_v9_v10!(self, account, !account.game_ids.is_empty())
    }

    pub fn get_game_ids(&self) -> Vec<GameId> {
        access_v9_v10!(self, account, {
            account.game_ids.into_iter().cloned().collect()
        })
    }

    pub fn get_tokens(&self) -> Vec<(AccountId, u128)> {
        access_v9_v10!(self, account, {
            account
                .tokens
                .iter()
                .map(|(a, b)| (a.clone(), *b))
                .collect()
        })
    }

    pub fn get_token_amount(&self, token_id: &AccountId) -> u128 {
        access_v9_v10!(self, account, *account.tokens.get(token_id).unwrap_or(&0))
    }

    pub fn add_token(&mut self, token_id: &AccountId, amount: u128) {
        access_v9_v10_mut!(self, account, {
            if !account.tokens.contains_key(token_id) {
                account.tokens.insert(token_id.clone(), 0);
            }
            *account.tokens.get_mut(token_id).unwrap() += amount;
        })
    }

    pub fn withdraw_token(&mut self, token_id: &AccountId) -> u128 {
        access_v9_v10_mut!(
            self,
            account,
            account.tokens.remove(token_id).unwrap_or_default()
        )
    }

    pub fn accept_challenge(
        &mut self,
        challenge_id: &ChallengeId,
        game_id: GameId,
        is_challenger: bool,
    ) -> Result<(), ContractError> {
        self.add_game_id(game_id)?;
        access_v9_v10_mut!(self, account, {
            if is_challenger {
                account.challenger.remove(challenge_id);
            } else {
                account.challenged.remove(challenge_id);
            }
            Ok(())
        })
    }

    pub fn reject_challenge(
        &mut self,
        challenge_id: &ChallengeId,
        is_challenger: bool,
    ) -> Result<(), ContractError> {
        access_v9_v10_mut!(self, account, {
            if is_challenger {
                account.challenger.remove(challenge_id);
            } else {
                account.challenged.remove(challenge_id);
            }
            Ok(())
        })
    }

    pub fn add_challenge(
        &mut self,
        challenge_id: ChallengeId,
        is_challenger: bool,
    ) -> Result<(), ContractError> {
        access_v9_v10_mut!(self, account, {
            if account.challenger.len() + account.challenged.len() >= MAX_OPEN_CHALLENGES {
                return Err(ContractError::MaxChallengesReached);
            }
            if is_challenger {
                account.challenger.insert(challenge_id);
            } else {
                account.challenged.insert(challenge_id);
            }
            Ok(())
        })
    }

    pub fn get_challenges(&self, is_challenger: bool) -> Vec<ChallengeId> {
        access_v9_v10!(self, account, {
            if is_challenger {
                account.challenger.iter().cloned().collect()
            } else {
                account.challenged.iter().cloned().collect()
            }
        })
    }

    pub fn apply_quest(&mut self, quest: Quest, defer: bool) -> u128 {
        let Account::V10(account) = self else {
            panic!("migration required");
        };
        let current_timestamp = env::block_timestamp_ms();
        let quests = account.quest_cooldowns.get_mut();
        while let Some((timestamp, cd)) = quests.front() {
            if *timestamp + cd.get_cooldown() < current_timestamp {
                quests.pop_front();
            } else {
                break;
            }
        }
        let on_cooldown = quests.iter().any(|(_, cd)| cd == &quest);
        let mint_amount = quest.get_points(on_cooldown);
        if defer {
            account.pending_points += mint_amount;
        } else {
            account.points += mint_amount;
            FtMint {
                owner_id: &account.account_id,
                amount: mint_amount.into(),
                memo: Some(quest.as_ref()),
            }
            .emit();
        }
        if !on_cooldown {
            quests.push_back((current_timestamp, quest));
        }
        mint_amount
    }

    pub fn apply_achievement(&mut self, achievement: Achievement, defer: bool) -> u128 {
        let Account::V10(account) = self else {
            panic!("migration required");
        };
        let achievements = account.achievements.get_mut();
        let has_achievement = achievements.iter().any(|(_, val)| val == &achievement);
        if !has_achievement {
            let mint_amount = achievement.get_points();
            if defer {
                account.pending_points += mint_amount;
            } else {
                account.points += mint_amount;
                FtMint {
                    owner_id: &account.account_id,
                    amount: mint_amount.into(),
                    memo: Some(achievement.as_ref()),
                }
                .emit();
            }
            achievements.push((env::block_timestamp_ms(), achievement));
            mint_amount
        } else {
            0
        }
    }

    pub fn record_win(&mut self) {
        let Account::V10(account) = self else {
            panic!("migration required");
        };
        account.wins += 1;
        account.win_streak += 1;
        if account.win_streak > account.max_win_streak {
            account.max_win_streak = account.win_streak;
        }
    }

    pub fn record_loss(&mut self) {
        let Account::V10(account) = self else {
            panic!("migration required");
        };
        account.win_streak = 0;
    }

    pub fn record_bet_placed(&mut self) {
        let Account::V10(account) = self else {
            panic!("migration required");
        };
        account.bets_placed += 1;
    }

    pub fn record_bet_won(&mut self) {
        let Account::V10(account) = self else {
            panic!("migration required");
        };
        account.bets_won += 1;
    }

    pub fn record_wager_played(&mut self) {
        let Account::V10(account) = self else {
            panic!("migration required");
        };
        account.wagers_played += 1;
    }

    pub fn record_wager_win(&mut self) {
        let Account::V10(account) = self else {
            panic!("migration required");
        };
        account.wager_wins += 1;
    }

    pub fn record_challenge_sent(&mut self) {
        let Account::V10(account) = self else {
            panic!("migration required");
        };
        account.challenges_sent += 1;
    }

    pub fn get_wins(&self) -> u32 {
        match self {
            Account::V10(account) => account.wins,
            _ => 0,
        }
    }

    pub fn get_win_streak(&self) -> u32 {
        match self {
            Account::V10(account) => account.win_streak,
            _ => 0,
        }
    }

    pub fn get_max_win_streak(&self) -> u32 {
        match self {
            Account::V10(account) => account.max_win_streak,
            _ => 0,
        }
    }

    pub fn get_bets_placed(&self) -> u32 {
        match self {
            Account::V10(account) => account.bets_placed,
            _ => 0,
        }
    }

    pub fn get_bets_won(&self) -> u32 {
        match self {
            Account::V10(account) => account.bets_won,
            _ => 0,
        }
    }

    pub fn get_wagers_played(&self) -> u32 {
        match self {
            Account::V10(account) => account.wagers_played,
            _ => 0,
        }
    }

    pub fn get_wager_wins(&self) -> u32 {
        match self {
            Account::V10(account) => account.wager_wins,
            _ => 0,
        }
    }

    pub fn get_challenges_sent(&self) -> u32 {
        match self {
            Account::V10(account) => account.challenges_sent,
            _ => 0,
        }
    }
}
