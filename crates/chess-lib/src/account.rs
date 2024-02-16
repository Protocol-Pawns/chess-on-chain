use crate::{
    Achievement, ChallengeId, ContractError, EloRating, GameId, Quest, StorageKey,
    MAX_OPEN_CHALLENGES, MAX_OPEN_GAMES,
};
use near_contract_standards::fungible_token::events::FtMint;
use near_sdk::{
    borsh::{self, BorshDeserialize, BorshSerialize},
    env,
    store::{Lazy, UnorderedMap, UnorderedSet},
    AccountId, NearToken,
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
    V6(AccountV6),
    V7(AccountV7),
}

#[derive(BorshDeserialize, BorshSerialize)]
#[borsh(crate = "near_sdk::borsh")]
pub struct AccountV7 {
    near_amount: NearToken,
    account_id: AccountId,
    is_human: bool,
    points: u128,
    elo: Option<EloRating>,
    game_ids: UnorderedSet<GameId>,
    finished_games: UnorderedSet<GameId>,
    challenger: UnorderedSet<ChallengeId>,
    challenged: UnorderedSet<ChallengeId>,
    quest_cooldowns: Lazy<VecDeque<(u64, Quest)>>,
    achievements: Lazy<Vec<(u64, Achievement)>>,
    tokens: UnorderedMap<AccountId, u128>,
}

#[derive(BorshDeserialize, BorshSerialize)]
#[borsh(crate = "near_sdk::borsh")]
pub struct AccountV6 {
    near_amount: u128,
    account_id: AccountId,
    is_human: bool,
    points: u128,
    elo: Option<EloRating>,
    game_ids: UnorderedSet<GameId>,
    finished_games: UnorderedSet<GameId>,
    challenger: UnorderedSet<ChallengeId>,
    challenged: UnorderedSet<ChallengeId>,
    quest_cooldowns: Lazy<VecDeque<(u64, Quest)>>,
    achievements: Lazy<Vec<(u64, Achievement)>>,
}

impl Account {
    pub fn new(account_id: AccountId, near_amount: NearToken, is_human: bool) -> Self {
        let id = env::sha256_array(account_id.as_bytes());
        let game_id_key: Vec<u8> = [
            borsh::to_vec(&StorageKey::VAccounts).unwrap().as_slice(),
            &id,
            borsh::to_vec(&StorageKey::AccountOrderIds)
                .unwrap()
                .as_slice(),
        ]
        .concat();
        let finished_games_key: Vec<u8> = [
            borsh::to_vec(&StorageKey::VAccounts).unwrap().as_slice(),
            &id,
            borsh::to_vec(&StorageKey::AccountFinishedGames)
                .unwrap()
                .as_slice(),
        ]
        .concat();
        let challenger_key: Vec<u8> = [
            borsh::to_vec(&StorageKey::VAccounts).unwrap().as_slice(),
            &id,
            borsh::to_vec(&StorageKey::AccountChallenger)
                .unwrap()
                .as_slice(),
        ]
        .concat();
        let challenged_key: Vec<u8> = [
            borsh::to_vec(&StorageKey::VAccounts).unwrap().as_slice(),
            &id,
            borsh::to_vec(&StorageKey::AccountChallenged)
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
            borsh::to_vec(&StorageKey::AccountTokens)
                .unwrap()
                .as_slice(),
        ]
        .concat();
        Self::V7(AccountV7 {
            account_id,
            near_amount,
            is_human,
            points: 0,
            elo: if is_human { Some(1_000.) } else { None },
            game_ids: UnorderedSet::new(game_id_key),
            finished_games: UnorderedSet::new(finished_games_key),
            challenger: UnorderedSet::new(challenger_key),
            challenged: UnorderedSet::new(challenged_key),
            quest_cooldowns: Lazy::new(quest_cooldown_key, VecDeque::new()),
            achievements: Lazy::new(achievement_key, Vec::new()),
            tokens: UnorderedMap::new(tokens_key),
        })
    }

    pub fn migrate(self) -> Self {
        if let Account::V6(AccountV6 {
            near_amount,
            account_id,
            is_human,
            elo,
            game_ids,
            finished_games,
            challenger,
            challenged,
            points,
            quest_cooldowns,
            achievements,
        }) = self
        {
            let id = env::sha256_array(account_id.as_bytes());
            let tokens_key: Vec<u8> = [
                borsh::to_vec(&StorageKey::VAccounts).unwrap().as_slice(),
                &id,
                borsh::to_vec(&StorageKey::AccountTokens)
                    .unwrap()
                    .as_slice(),
            ]
            .concat();
            Account::V7(AccountV7 {
                near_amount: NearToken::from_yoctonear(near_amount),
                account_id,
                is_human,
                points,
                elo,
                game_ids,
                finished_games,
                challenger,
                challenged,
                quest_cooldowns,
                achievements,
                tokens: UnorderedMap::new(tokens_key),
            })
        } else {
            self
        }
    }

    pub fn get_near_amount(&self) -> NearToken {
        let Account::V7(account) = self else {
            panic!("migration required");
        };
        account.near_amount
    }

    pub fn is_human(&self) -> bool {
        let Account::V7(account) = self else {
            panic!("migration required");
        };
        account.is_human
    }

    pub fn set_is_human(&mut self, is_human: bool) {
        let Account::V7(account) = self else {
            panic!("migration required");
        };
        account.is_human = is_human;
        if is_human {
            if account.elo.is_none() {
                account.elo = Some(1_000.);
            }
        } else {
            account.elo = None;
        }
    }

    pub fn get_elo(&self) -> Option<EloRating> {
        let Account::V7(account) = self else {
            panic!("migration required");
        };
        account.elo
    }

    pub fn set_elo(&mut self, elo: EloRating) {
        let Account::V7(account) = self else {
            panic!("migration required");
        };
        if account.is_human {
            account.elo = Some(elo);
        }
    }

    pub fn get_points(&self) -> u128 {
        let Account::V7(account) = self else {
            panic!("migration required");
        };
        account.points
    }

    pub fn get_quest_cooldowns(&self) -> &VecDeque<(u64, Quest)> {
        let Account::V7(account) = self else {
            panic!("migration required");
        };
        account.quest_cooldowns.get()
    }

    pub fn get_achievements(&self) -> &Vec<(u64, Achievement)> {
        let Account::V7(account) = self else {
            panic!("migration required");
        };
        account.achievements.get()
    }

    pub fn add_game_id(&mut self, game_id: GameId) -> Result<(), ContractError> {
        let Account::V7(account) = self else {
            panic!("migration required");
        };
        if account.game_ids.len() >= MAX_OPEN_GAMES {
            return Err(ContractError::MaxGamesReached);
        }
        account.game_ids.insert(game_id);
        Ok(())
    }

    pub fn remove_game_id(&mut self, game_id: &GameId) -> bool {
        let Account::V7(account) = self else {
            panic!("migration required");
        };
        account.game_ids.remove(game_id)
    }

    pub fn save_finished_game(&mut self, game_id: GameId) {
        let Account::V7(account) = self else {
            panic!("migration required");
        };
        account.finished_games.insert(game_id);
    }

    pub fn is_playing(&self) -> bool {
        let Account::V7(account) = self else {
            panic!("migration required");
        };
        !account.game_ids.is_empty()
    }

    pub fn get_game_ids(&self) -> Vec<GameId> {
        let Account::V7(account) = self else {
            panic!("migration required");
        };
        account.game_ids.into_iter().cloned().collect()
    }

    pub fn get_finished_games(&self) -> Vec<GameId> {
        let Account::V7(account) = self else {
            panic!("migration required");
        };
        account.finished_games.into_iter().cloned().collect()
    }

    pub fn get_tokens(&self) -> Vec<(AccountId, u128)> {
        let Account::V7(account) = self else {
            panic!("migration required");
        };
        account
            .tokens
            .iter()
            .map(|(a, b)| (a.clone(), *b))
            .collect()
    }

    pub fn get_token_amount(&self, token_id: &AccountId) -> u128 {
        let Account::V7(account) = self else {
            panic!("migration required");
        };
        *account.tokens.get(token_id).unwrap_or(&0)
    }

    pub fn add_token(&mut self, token_id: &AccountId, amount: u128) {
        let Account::V7(account) = self else {
            panic!("migration required");
        };
        if !account.tokens.contains_key(token_id) {
            account.tokens.insert(token_id.clone(), 0);
        }
        *account.tokens.get_mut(token_id).unwrap() += amount;
    }

    pub fn withdraw_token(&mut self, token_id: &AccountId) -> u128 {
        let Account::V7(account) = self else {
            panic!("migration required");
        };
        account.tokens.remove(token_id).unwrap_or_default()
    }

    pub fn accept_challenge(
        &mut self,
        challenge_id: &ChallengeId,
        game_id: GameId,
        is_challenger: bool,
    ) -> Result<(), ContractError> {
        self.add_game_id(game_id)?;
        let Account::V7(account) = self else {
            panic!("migration required");
        };
        if is_challenger {
            account.challenger.remove(challenge_id);
        } else {
            account.challenged.remove(challenge_id);
        }
        Ok(())
    }

    pub fn reject_challenge(
        &mut self,
        challenge_id: &ChallengeId,
        is_challenger: bool,
    ) -> Result<(), ContractError> {
        let Account::V7(account) = self else {
            panic!("migration required");
        };
        if is_challenger {
            account.challenger.remove(challenge_id);
        } else {
            account.challenged.remove(challenge_id);
        }
        Ok(())
    }

    pub fn add_challenge(
        &mut self,
        challenge_id: ChallengeId,
        is_challenger: bool,
    ) -> Result<(), ContractError> {
        let Account::V7(account) = self else {
            panic!("migration required");
        };
        if account.challenger.len() + account.challenged.len() >= MAX_OPEN_CHALLENGES {
            return Err(ContractError::MaxChallengesReached);
        }
        if is_challenger {
            account.challenger.insert(challenge_id);
        } else {
            account.challenged.insert(challenge_id);
        }
        Ok(())
    }

    pub fn get_challenges(&self, is_challenger: bool) -> Vec<ChallengeId> {
        let Account::V7(account) = self else {
            panic!("migration required");
        };
        if is_challenger {
            account.challenger.iter().cloned().collect()
        } else {
            account.challenged.iter().cloned().collect()
        }
    }

    pub fn apply_quest(&mut self, quest: Quest) {
        let Account::V7(account) = self else {
            panic!("migration required");
        };
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
            memo: Some(quest.get_name()),
        }
        .emit();
        if !on_cooldown {
            quests.push_back((current_timestamp, quest));
        }
    }

    pub fn apply_achievement(&mut self, achievement: Achievement) {
        let Account::V7(account) = self else {
            panic!("migration required");
        };
        let achievements = account.achievements.get_mut();
        let has_achievement = achievements.iter().any(|(_, val)| val == &achievement);
        if !has_achievement {
            let mint_amount = achievement.get_points();
            account.points += mint_amount;
            FtMint {
                owner_id: &account.account_id,
                amount: mint_amount.into(),
                memo: Some(achievement.get_name()),
            }
            .emit();
            achievements.push((env::block_timestamp_ms(), achievement));
        }
    }
}
