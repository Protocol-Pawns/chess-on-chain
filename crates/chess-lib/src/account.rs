#![allow(deprecated)]
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
    store::{IterableMap, IterableSet, Lazy, UnorderedMap, UnorderedSet},
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
    V8(AccountV8),
    V9(AccountV9),
}

macro_rules! access_v8_v9 {
    ($self:expr, $var:ident, $body:expr) => {
        match $self {
            Account::V8($var) => $body,
            Account::V9($var) => $body,
            _ => panic!("migration required"),
        }
    };
}

macro_rules! access_v8_v9_mut {
    ($self:expr, $var:ident, $body:expr) => {
        match $self {
            Account::V8(ref mut $var) => $body,
            Account::V9(ref mut $var) => $body,
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
#[allow(deprecated)]
pub struct AccountV8 {
    near_amount: NearToken,
    account_id: AccountId,
    is_human: bool,
    points: u128,
    elo: Option<EloRating>,
    game_ids: UnorderedSet<GameId>,
    challenger: UnorderedSet<ChallengeId>,
    challenged: UnorderedSet<ChallengeId>,
    quest_cooldowns: Lazy<VecDeque<(u64, Quest)>>,
    achievements: Lazy<Vec<(u64, Achievement)>>,
    tokens: UnorderedMap<AccountId, u128>,
}

#[derive(Deserialize, Serialize, NearSchema)]
#[serde(crate = "near_sdk::serde")]
pub struct AccountInfo {
    pub near_amount: NearToken,
    pub is_agent: bool,
    pub points: U128,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub elo: Option<EloRating>,
}

impl From<&Account> for AccountInfo {
    fn from(account: &Account) -> Self {
        AccountInfo {
            near_amount: account.get_near_amount(),
            is_agent: account.is_agent(),
            points: account.get_points().into(),
            elo: account.get_elo(),
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
        Self::V9(AccountV9 {
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
        })
    }

    #[allow(deprecated)]
    pub fn migrate(self) -> Self {
        let Account::V8(AccountV8 {
            near_amount,
            account_id,
            is_human: _,
            points,
            elo,
            mut game_ids,
            mut challenger,
            mut challenged,
            quest_cooldowns,
            achievements,
            mut tokens,
        }) = self
        else {
            panic!("migration required");
        };

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
        let tokens_key: Vec<u8> = [
            borsh::to_vec(&StorageKey::VAccounts).unwrap().as_slice(),
            &id,
            borsh::to_vec(&StorageKey::V9AccountTokens)
                .unwrap()
                .as_slice(),
        ]
        .concat();

        let game_ids_list: Vec<GameId> = game_ids.iter().cloned().collect();
        game_ids.flush();
        let mut new_game_ids = IterableSet::new(game_id_key);
        for gid in game_ids_list {
            new_game_ids.insert(gid);
        }

        let challenger_list: Vec<ChallengeId> = challenger.iter().cloned().collect();
        challenger.flush();
        let mut new_challenger = IterableSet::new(challenger_key);
        for cid in challenger_list {
            new_challenger.insert(cid);
        }

        let challenged_list: Vec<ChallengeId> = challenged.iter().cloned().collect();
        challenged.flush();
        let mut new_challenged = IterableSet::new(challenged_key);
        for cid in challenged_list {
            new_challenged.insert(cid);
        }

        let tokens_list: Vec<(AccountId, u128)> =
            tokens.iter().map(|(k, v)| (k.clone(), *v)).collect();
        tokens.flush();
        let mut new_tokens = IterableMap::new(tokens_key);
        for (k, v) in tokens_list {
            new_tokens.insert(k, v);
        }

        Account::V9(AccountV9 {
            near_amount,
            account_id,
            is_agent: false,
            points,
            elo,
            game_ids: new_game_ids,
            challenger: new_challenger,
            challenged: new_challenged,
            quest_cooldowns,
            achievements,
            tokens: new_tokens,
        })
    }

    pub fn get_near_amount(&self) -> NearToken {
        access_v8_v9!(self, account, account.near_amount)
    }

    pub fn is_agent(&self) -> bool {
        match self {
            Account::V8(_) => false,
            Account::V9(account) => account.is_agent,
            _ => panic!("migration required"),
        }
    }

    pub fn set_is_agent(&mut self, is_agent: bool) {
        let Account::V9(account) = self else {
            panic!("migration required");
        };
        account.is_agent = is_agent;
    }

    pub fn get_elo(&self) -> Option<EloRating> {
        access_v8_v9!(self, account, account.elo)
    }

    pub fn set_elo(&mut self, elo: EloRating) {
        access_v8_v9_mut!(self, account, {
            account.elo = Some(elo);
        })
    }

    pub fn get_points(&self) -> u128 {
        access_v8_v9!(self, account, account.points)
    }

    pub fn get_quest_cooldowns(&self) -> &VecDeque<(u64, Quest)> {
        access_v8_v9!(self, account, account.quest_cooldowns.get())
    }

    pub fn get_achievements(&self) -> &Vec<(u64, Achievement)> {
        access_v8_v9!(self, account, account.achievements.get())
    }

    pub fn add_game_id(&mut self, game_id: GameId) -> Result<(), ContractError> {
        access_v8_v9_mut!(self, account, {
            if account.game_ids.len() >= MAX_OPEN_GAMES {
                return Err(ContractError::MaxGamesReached);
            }
            account.game_ids.insert(game_id);
            Ok(())
        })
    }

    pub fn remove_game_id(&mut self, game_id: &GameId) -> bool {
        access_v8_v9_mut!(self, account, account.game_ids.remove(game_id))
    }

    pub fn is_playing(&self) -> bool {
        access_v8_v9!(self, account, !account.game_ids.is_empty())
    }

    pub fn get_game_ids(&self) -> Vec<GameId> {
        access_v8_v9!(self, account, {
            account.game_ids.into_iter().cloned().collect()
        })
    }

    pub fn get_tokens(&self) -> Vec<(AccountId, u128)> {
        access_v8_v9!(self, account, {
            account
                .tokens
                .iter()
                .map(|(a, b)| (a.clone(), *b))
                .collect()
        })
    }

    pub fn get_token_amount(&self, token_id: &AccountId) -> u128 {
        access_v8_v9!(self, account, *account.tokens.get(token_id).unwrap_or(&0))
    }

    pub fn add_token(&mut self, token_id: &AccountId, amount: u128) {
        access_v8_v9_mut!(self, account, {
            if !account.tokens.contains_key(token_id) {
                account.tokens.insert(token_id.clone(), 0);
            }
            *account.tokens.get_mut(token_id).unwrap() += amount;
        })
    }

    pub fn withdraw_token(&mut self, token_id: &AccountId) -> u128 {
        access_v8_v9_mut!(
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
        access_v8_v9_mut!(self, account, {
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
        access_v8_v9_mut!(self, account, {
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
        access_v8_v9_mut!(self, account, {
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
        access_v8_v9!(self, account, {
            if is_challenger {
                account.challenger.iter().cloned().collect()
            } else {
                account.challenged.iter().cloned().collect()
            }
        })
    }

    pub fn apply_quest(&mut self, quest: Quest) -> u128 {
        access_v8_v9_mut!(self, account, {
            let current_timestamp = env::block_timestamp_ms();
            let quests = account.quest_cooldowns.get_mut();
            if let Some(index) = quests
                .iter()
                .enumerate()
                .find_map(|(index, (timestamp, cd))| {
                    if timestamp + cd.get_cooldown() < current_timestamp {
                        Some(index)
                    } else {
                        None
                    }
                })
            {
                for _ in 0..index + 1 {
                    quests.pop_front();
                }
            }
            let on_cooldown = quests.iter().any(|(_, cd)| cd == &quest);
            let mint_amount = quest.get_points(on_cooldown);
            account.points += mint_amount;
            FtMint {
                owner_id: &account.account_id,
                amount: mint_amount.into(),
                memo: Some(quest.as_ref()),
            }
            .emit();
            if !on_cooldown {
                quests.push_back((current_timestamp, quest));
            }
            mint_amount
        })
    }

    pub fn apply_achievement(&mut self, achievement: Achievement) -> u128 {
        access_v8_v9_mut!(self, account, {
            let achievements = account.achievements.get_mut();
            let has_achievement = achievements.iter().any(|(_, val)| val == &achievement);
            if !has_achievement {
                let mint_amount = achievement.get_points();
                account.points += mint_amount;
                FtMint {
                    owner_id: &account.account_id,
                    amount: mint_amount.into(),
                    memo: Some(achievement.as_ref()),
                }
                .emit();
                achievements.push((env::block_timestamp_ms(), achievement));
                mint_amount
            } else {
                0
            }
        })
    }
}
