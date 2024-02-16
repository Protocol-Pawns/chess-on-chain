use crate::{
    calculate_elo, create_challenge_id, Account, Achievement, BetId, Challenge, ChallengeId, Chess,
    ChessEvent, ChessNotification, ContractError, Difficulty, EloConfig, EloOutcome, Game, GameId,
    GameOutcome, Player, Wager, ONE_YOCTO,
};
use chess_engine::Color;
use maplit::hashmap;
use near_contract_standards::fungible_token::core::ext_ft_core;
use near_sdk::{AccountId, NearToken};
use primitive_types::U128;
use std::{cmp, collections::HashMap, ops::Div};

impl Chess {
    pub(crate) fn internal_challenge(
        &mut self,
        challenger_id: AccountId,
        challenged_id: AccountId,
        wager: Wager,
    ) -> Result<(), ContractError> {
        let challenge = Challenge::new(challenger_id.clone(), challenged_id.clone(), wager);

        if self.challenges.contains_key(challenge.id())
            || self
                .challenges
                .contains_key(&create_challenge_id(&challenged_id, &challenger_id))
        {
            return Err(ContractError::ChallengeExists);
        }

        let challenger = self
            .accounts
            .get_mut(&challenger_id)
            .ok_or_else(|| ContractError::AccountNotRegistered(challenger_id.clone()))?;
        challenger.add_challenge(challenge.id().clone(), true)?;

        let challenged = self
            .accounts
            .get_mut(&challenged_id)
            .ok_or_else(|| ContractError::AccountNotRegistered(challenged_id.clone()))?;
        challenged.add_challenge(challenge.id().clone(), false)?;

        self.challenges
            .insert(challenge.id().clone(), challenge.clone());
        let challenge_id = challenge.id().clone();

        let event = ChessEvent::Challenge(challenge);
        event.emit();

        self.internal_send_notify(hashmap! {
            challenged_id =>  vec![ChessNotification::Challenged {
                challenge_id,
                challenger_id,
            }]
        });

        Ok(())
    }

    pub(crate) fn internal_accept_challenge(
        &mut self,
        challenged_id: AccountId,
        challenge_id: ChallengeId,
        paid_wager: Wager,
    ) -> Result<(GameId, Option<u128>), ContractError> {
        let challenge = self
            .challenges
            .remove(&challenge_id)
            .ok_or(ContractError::ChallengeNotExists(challenge_id.clone()))?;
        let refund = challenge.check_accept(&challenged_id, &paid_wager)?;

        let challenger_id = challenge.get_challenger();
        let players = (challenger_id.clone(), challenged_id.clone());
        let bet_id = BetId::new(players)?;
        let has_bets = if let Some(bets) = self.bets.get_mut(&bet_id) {
            bets.filter_valid(&mut self.accounts);
            bets.is_locked = true;
            !bets.bets.is_empty()
        } else {
            false
        };

        let game = Game::new(
            Player::Human(challenger_id.clone()),
            Player::Human(challenged_id.clone()),
            paid_wager,
            has_bets,
        );
        let game_id = game.get_game_id().clone();

        self.accounts
            .get_mut(&challenged_id)
            .ok_or_else(|| ContractError::AccountNotRegistered(challenged_id.clone()))?
            .accept_challenge(&challenge_id, game_id.clone(), false)?;
        let challenger = self
            .accounts
            .get_mut(challenger_id)
            .ok_or_else(|| ContractError::AccountNotRegistered(challenger_id.clone()))?;
        challenger.accept_challenge(&challenge_id, game_id.clone(), true)?;

        let event = ChessEvent::AcceptChallenge {
            challenge_id,
            game_id: game_id.clone(),
        };
        event.emit();
        let event = ChessEvent::CreateGame {
            game_id: game_id.clone(),
            white: game.get_white().clone(),
            black: game.get_black().clone(),
            board: game.get_board_state(),
        };
        event.emit();
        self.games.insert(game_id.clone(), game);

        self.internal_send_notify(hashmap! {
            challenger_id.clone() => vec![ChessNotification::AcceptedChallenge {
                game_id: game_id.clone(),
                challenged_id,
            }]
        });

        Ok((game_id, refund))
    }

    pub(crate) fn internal_handle_outcome(
        &mut self,
        game_id: GameId,
        outcome: &GameOutcome,
        notifications: &mut HashMap<AccountId, Vec<ChessNotification>>,
    ) {
        let game = self.games.remove(&game_id).unwrap();
        if let Some(account) = game.get_white().as_account_mut(self) {
            account.remove_game_id(&game_id);
            account.save_finished_game(game_id.clone());
        }
        if let Some(account) = game.get_black().as_account_mut(self) {
            account.remove_game_id(&game_id);
            account.save_finished_game(game_id.clone());
        }
        let recent_finished_games = self.recent_finished_games.get_mut();
        recent_finished_games.push_front(game_id.clone());
        if recent_finished_games.len() > 100 {
            recent_finished_games.pop_back();
        }

        if game.get_black().is_human() {
            self.internal_calculate_elo(&game, outcome);
        }

        if let GameOutcome::Victory(color) = outcome {
            let (winner, looser) = match color {
                Color::White => (game.get_white(), game.get_black()),
                Color::Black => (game.get_black(), game.get_white()),
            };
            if winner.is_human() {
                if let Some(achievement) = match looser {
                    Player::Human(account_id) => {
                        let looser_is_human = self.accounts.get(account_id).unwrap().is_human();
                        if looser_is_human {
                            Some(Achievement::FirstWinHuman)
                        } else {
                            None
                        }
                    }
                    Player::Ai(Difficulty::Easy) => Some(Achievement::FirstWinAiEasy),
                    Player::Ai(Difficulty::Medium) => Some(Achievement::FirstWinAiMedium),
                    Player::Ai(Difficulty::Hard) => Some(Achievement::FirstWinAiHard),
                } {
                    winner
                        .as_account_mut(self)
                        .unwrap()
                        .apply_achievement(achievement);
                }
            }
        }

        if let Some((token_id, amount)) = game.get_wager().clone() {
            match outcome {
                GameOutcome::Victory(color) => {
                    let wager_amount = 2 * amount.0 - self.deduct_fees(&token_id, amount.0);
                    ext_ft_core::ext(token_id)
                        .with_attached_deposit(ONE_YOCTO)
                        .with_unused_gas_weight(1)
                        .ft_transfer(
                            match color {
                                Color::White => game.get_white().get_account_id().unwrap(),
                                Color::Black => game.get_black().get_account_id().unwrap(),
                            },
                            wager_amount.into(),
                            Some("wager win".to_string()),
                        );
                }
                GameOutcome::Stalemate => {
                    ext_ft_core::ext(token_id.clone())
                        .with_attached_deposit(ONE_YOCTO)
                        .with_unused_gas_weight(1)
                        .ft_transfer(
                            game.get_white().get_account_id().unwrap(),
                            amount,
                            Some("wager refund".to_string()),
                        )
                        .then(
                            ext_ft_core::ext(token_id)
                                .with_attached_deposit(ONE_YOCTO)
                                .with_unused_gas_weight(1)
                                .ft_transfer(
                                    game.get_black().get_account_id().unwrap(),
                                    amount,
                                    Some("wager refund".to_string()),
                                ),
                        );
                }
            }
        }

        if game.has_bets() {
            let players = (
                game.get_white().get_account_id().unwrap(),
                game.get_black().get_account_id().unwrap(),
            );
            let bet_id = BetId::new(players).unwrap();
            let all_bets = self.bets.remove(&bet_id).unwrap();
            match outcome {
                GameOutcome::Victory(color) => {
                    let winner_id = match color {
                        Color::White => game.get_white().get_account_id().unwrap(),

                        Color::Black => game.get_black().get_account_id().unwrap(),
                    };

                    // TODO Limit max amount of bets
                    for (token_id, bets) in all_bets.bets.iter() {
                        let mut total_winner = 0;
                        let mut total_looser = 0;
                        for (_, bet) in bets.iter() {
                            if winner_id == bet.winner {
                                total_winner += bet.amount;
                            } else {
                                total_looser += bet.amount;
                            }
                        }

                        total_looser -= self.deduct_fees(token_id, total_looser);

                        let mut total_win_amount = 0;

                        // pay out winners
                        for (account_id, bet) in bets.iter() {
                            if winner_id == bet.winner {
                                let mut win_amount = U128::from(total_looser)
                                    .full_mul(bet.amount.into())
                                    .div(total_winner)
                                    .as_u128();
                                win_amount = cmp::min(win_amount, bet.amount);
                                total_win_amount += win_amount;

                                self.accounts
                                    .get_mut(account_id)
                                    .unwrap()
                                    .add_token(token_id, win_amount + bet.amount);
                            }
                        }

                        // refund loosers, if `total_win_amount` is less than what loosers lost
                        let total_refund_amount = total_looser - total_win_amount;
                        if total_refund_amount > 0 {
                            for (account_id, bet) in bets.iter() {
                                if winner_id != bet.winner {
                                    let refund_amount = U128::from(total_refund_amount)
                                        .full_mul(bet.amount.into())
                                        .div(total_looser)
                                        .as_u128();

                                    self.accounts
                                        .get_mut(account_id)
                                        .unwrap()
                                        .add_token(token_id, refund_amount);
                                }
                            }
                        }
                    }
                }
                GameOutcome::Stalemate => {
                    for (token_id, bets) in all_bets.bets.iter() {
                        for (account_id, bet) in bets {
                            self.accounts
                                .get_mut(account_id)
                                .unwrap()
                                .add_token(token_id, bet.amount);
                        }
                    }
                }
            }
        }

        if let Player::Human(account_id) = game.get_white() {
            notifications.insert(
                account_id.clone(),
                vec![ChessNotification::Outcome {
                    game_id: game_id.clone(),
                    outcome: outcome.clone(),
                }],
            );
        }
        if let Player::Human(account_id) = game.get_black() {
            notifications.insert(
                account_id.clone(),
                vec![ChessNotification::Outcome {
                    game_id,
                    outcome: outcome.clone(),
                }],
            );
        }
    }

    pub(crate) fn internal_calculate_elo(&mut self, game: &Game, outcome: &GameOutcome) {
        if let (Some(Some(elo_white)), Some(Some(elo_black)), GameOutcome::Victory(color)) = (
            game.get_white()
                .as_account_mut(self)
                .map(|account| account.get_elo()),
            game.get_black()
                .as_account_mut(self)
                .map(|account| account.get_elo()),
            outcome,
        ) {
            let (new_elo_white, new_elo_black) = calculate_elo(
                elo_white,
                elo_black,
                match color {
                    Color::White => &EloOutcome::WIN,
                    Color::Black => &EloOutcome::LOSS,
                },
                &EloConfig::new(),
            );
            game.get_white()
                .as_account_mut(self)
                .unwrap()
                .set_elo(new_elo_white);
            game.get_black()
                .as_account_mut(self)
                .unwrap()
                .set_elo(new_elo_black);
        }
    }

    pub(crate) fn internal_get_account(
        &self,
        account_id: &AccountId,
    ) -> Result<&Account, ContractError> {
        self.accounts
            .get(account_id)
            .ok_or_else(|| ContractError::AccountNotRegistered(account_id.clone()))
    }

    pub(crate) fn internal_get_account_mut(
        &mut self,
        account_id: &AccountId,
    ) -> Result<&mut Account, ContractError> {
        self.accounts
            .get_mut(account_id)
            .ok_or_else(|| ContractError::AccountNotRegistered(account_id.clone()))
    }

    pub(crate) fn internal_register_account(
        &mut self,
        account_id: AccountId,
        amount: NearToken,
        is_human: bool,
    ) {
        let account = Account::new(account_id.clone(), amount, is_human);
        self.accounts.insert(account_id, account);
    }

    fn deduct_fees(&mut self, token_id: &AccountId, amount: u128) -> u128 {
        let fees = self.fees.get();
        let treasury_amount = U128::from(amount)
            .full_mul(fees.treasury.into())
            .div(10_000)
            .as_u128();
        if let Some(treasury) = self.treasury.get_mut(token_id) {
            *treasury += treasury_amount;
        } else {
            self.treasury.insert(token_id.clone(), treasury_amount);
        }

        let mut total_royalty = 0;
        fees.royalties
            .iter()
            .for_each(|(royalty_account, royalty_fee)| {
                let royalty_amount = U128::from(amount)
                    .full_mul((*royalty_fee).into())
                    .div(10_000)
                    .as_u128();
                total_royalty += royalty_amount;

                ext_ft_core::ext(token_id.clone())
                    .with_attached_deposit(ONE_YOCTO)
                    .with_unused_gas_weight(1)
                    .ft_transfer(royalty_account.clone(), royalty_amount.into(), None);
            });

        treasury_amount + total_royalty
    }
}
