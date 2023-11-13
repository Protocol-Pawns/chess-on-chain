use crate::{
    calculate_elo, create_challenge_id, Account, Challenge, ChallengeId, Chess, ChessEvent,
    ChessNotification, ContractError, EloConfig, EloOutcome, Game, GameId, GameOutcome, Player,
    Wager,
};
use chess_engine::Color;
use maplit::hashmap;
use near_contract_standards::fungible_token::core::ext_ft_core;
use near_sdk::{AccountId, Balance};
use std::collections::HashMap;

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
    ) -> Result<(GameId, Option<Balance>), ContractError> {
        let challenged = self
            .accounts
            .get_mut(&challenged_id)
            .ok_or_else(|| ContractError::AccountNotRegistered(challenged_id.clone()))?;

        let challenge = self
            .challenges
            .remove(&challenge_id)
            .ok_or(ContractError::ChallengeNotExists(challenge_id.clone()))?;
        let refund = challenge.check_accept(&challenged_id, &paid_wager)?;

        let challenger_id = challenge.get_challenger();
        let game = Game::new(
            Player::Human(challenger_id.clone()),
            Player::Human(challenged_id.clone()),
            paid_wager,
        );
        let game_id = game.get_game_id().clone();

        challenged.accept_challenge(&challenge_id, game_id.clone(), false)?;
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

        if let Some((token_id, amount)) = game.get_wager().clone() {
            match outcome {
                GameOutcome::Victory(color) => {
                    ext_ft_core::ext(token_id)
                        .with_attached_deposit(1)
                        .with_unused_gas_weight(1)
                        .ft_transfer(
                            match color {
                                Color::White => game.get_white().get_account_id().unwrap(),
                                Color::Black => game.get_black().get_account_id().unwrap(),
                            },
                            (2 * amount.0).into(),
                            Some("wager win".to_string()),
                        );
                }
                GameOutcome::Stalemate => {
                    ext_ft_core::ext(token_id.clone())
                        .with_attached_deposit(1)
                        .with_unused_gas_weight(1)
                        .ft_transfer(
                            game.get_white().get_account_id().unwrap(),
                            amount,
                            Some("wager refund".to_string()),
                        )
                        .then(
                            ext_ft_core::ext(token_id)
                                .with_attached_deposit(1)
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

    pub(crate) fn internal_register_account(
        &mut self,
        account_id: AccountId,
        amount: Balance,
        is_human: bool,
    ) {
        let account = Account::new(account_id.clone(), amount, is_human);
        self.accounts.insert(account_id, account);
    }
}
