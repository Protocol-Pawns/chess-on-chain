use crate::{
    calculate_elo, create_challenge_id, Account, Achievement, BetId, Challenge, ChallengeId, Chess,
    ChessEvent, ContractError, Difficulty, EloConfig, EloOutcome, Game, GameId, GameOutcome,
    Player, Quest, Wager, FT_TRANSFER_GAS, ONE_YOCTO, WAGER_PAYOUT_CALLBACK_GAS,
};
use chess_engine::Color;
use near_contract_standards::fungible_token::core::ext_ft_core;
use near_sdk::{env, AccountId, NearToken};
use primitive_types::U128;
use std::{cmp, collections::HashSet, ops::Div};

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

        let event = ChessEvent::Challenge(challenge);
        event.emit();

        let mut minted: u128 = 0;
        {
            let challenger = self.accounts.get_mut(&challenger_id).unwrap();
            challenger.record_challenge_sent();
            minted += challenger.apply_quest(Quest::WeeklyChallenger, false);
            let challenges_sent = challenger.get_challenges_sent();
            if challenges_sent == 1 {
                minted += challenger.apply_achievement(Achievement::FirstChallenge, false);
            }
            for (threshold, achievement) in [
                (10, Achievement::Challenges10),
                (100, Achievement::Challenges100),
            ] {
                if challenges_sent == threshold {
                    minted += challenger.apply_achievement(achievement, false);
                }
            }
        }
        self.points_total_supply += minted;

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
            let refunded = bets.filter_valid(&mut self.accounts);
            for account_id in refunded {
                if let Some(active) = self.bettor_active_bets.get_mut(&account_id) {
                    *active = active.saturating_sub(1);
                }
            }
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

        if has_bets {
            let event = ChessEvent::LockBets {
                players: (challenger_id.clone(), challenged_id.clone()),
                game_id: game_id.clone(),
            };
            event.emit();
        }

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

        Ok((game_id, refund))
    }

    pub(crate) fn internal_handle_outcome(
        &mut self,
        game_id: GameId,
        outcome: &GameOutcome,
        resigned: bool,
    ) {
        let game = self.games.remove(&game_id).unwrap();
        let move_count = game.get_move_count();
        if let Some(account) = game.get_white().as_account_mut(self) {
            account.remove_game_id(&game_id);
        }
        if let Some(account) = game.get_black().as_account_mut(self) {
            account.remove_game_id(&game_id);
        }

        let is_human_game = game.get_black().is_human();

        if is_human_game {
            self.internal_calculate_elo(&game, outcome);
        }

        let mut minted: u128 = 0;

        if let GameOutcome::Victory(color) = outcome {
            let (winner, looser) = match color {
                Color::White => (game.get_white(), game.get_black()),
                Color::Black => (game.get_black(), game.get_white()),
            };
            if winner.is_human() {
                if let Some(achievement) = match looser {
                    Player::Human(_) => Some(Achievement::FirstWin),
                    Player::Ai(Difficulty::Easy) => Some(Achievement::FirstWinAiEasy),
                    Player::Ai(Difficulty::Medium) => Some(Achievement::FirstWinAiMedium),
                    Player::Ai(Difficulty::Hard) => Some(Achievement::FirstWinAiHard),
                } {
                    minted += winner
                        .as_account_mut(self)
                        .unwrap()
                        .apply_achievement(achievement, false);
                }
                if looser.is_human() {
                    minted += winner
                        .as_account_mut(self)
                        .unwrap()
                        .apply_quest(Quest::WeeklyWin, false);

                    let winner_id = winner.get_account_id().unwrap().clone();
                    let winner_account = self.accounts.get_mut(&winner_id).unwrap();
                    winner_account.record_win();
                    let wins = winner_account.get_wins();
                    let streak = winner_account.get_win_streak();
                    for (threshold, achievement) in [
                        (10, Achievement::Wins10),
                        (50, Achievement::Wins50),
                        (100, Achievement::Wins100),
                        (500, Achievement::Wins500),
                    ] {
                        if wins == threshold {
                            minted += winner_account.apply_achievement(achievement, false);
                        }
                    }
                    for (threshold, achievement) in [
                        (3, Achievement::WinStreak3),
                        (5, Achievement::WinStreak5),
                        (10, Achievement::WinStreak10),
                        (25, Achievement::WinStreak25),
                    ] {
                        if streak == threshold {
                            minted += winner_account.apply_achievement(achievement, false);
                        }
                    }

                    if let Some(looser_id) = looser.get_account_id() {
                        if let Some(looser_account) = self.accounts.get_mut(&looser_id) {
                            looser_account.record_loss();
                        }
                    }
                }
            }
        }

        let daily_game_eligible = !resigned || move_count >= 5;
        if daily_game_eligible {
            if let Some(white_id) = game.get_white().get_account_id() {
                minted += self
                    .accounts
                    .get_mut(&white_id)
                    .unwrap()
                    .apply_quest(Quest::DailyGame, false);
            }
            if let Some(black_id) = game.get_black().get_account_id() {
                minted += self
                    .accounts
                    .get_mut(&black_id)
                    .unwrap()
                    .apply_quest(Quest::DailyGame, false);
            }
        }

        if let Some((token_id, amount)) = game.get_wager().clone() {
            if let Some(white_id) = game.get_white().get_account_id() {
                let account = self.accounts.get_mut(&white_id).unwrap();
                account.record_wager_played();
                if account.get_wagers_played() == 1 {
                    minted += account.apply_achievement(Achievement::FirstWager, false);
                }
            }
            if let Some(black_id) = game.get_black().get_account_id() {
                let account = self.accounts.get_mut(&black_id).unwrap();
                account.record_wager_played();
                if account.get_wagers_played() == 1 {
                    minted += account.apply_achievement(Achievement::FirstWager, false);
                }
            }

            match outcome {
                GameOutcome::Victory(color) => {
                    let winner_id = match color {
                        Color::White => game.get_white().get_account_id().unwrap().clone(),
                        Color::Black => game.get_black().get_account_id().unwrap().clone(),
                    };
                    let winner_account = self.accounts.get_mut(&winner_id).unwrap();
                    winner_account.record_wager_win();
                    let wager_wins = winner_account.get_wager_wins();
                    if wager_wins == 1 {
                        minted +=
                            winner_account.apply_achievement(Achievement::FirstWagerWin, false);
                    }
                    for (threshold, achievement) in [
                        (10, Achievement::WagerWins10),
                        (100, Achievement::WagerWins100),
                    ] {
                        if wager_wins == threshold {
                            minted += winner_account.apply_achievement(achievement, false);
                        }
                    }

                    let total_pool = 2 * amount.0;
                    let fees = self.deduct_fees(&token_id, total_pool);
                    let wager_amount = total_pool - fees;
                    ext_ft_core::ext(token_id.clone())
                        .with_attached_deposit(ONE_YOCTO)
                        .with_static_gas(FT_TRANSFER_GAS)
                        .ft_transfer(
                            winner_id.clone(),
                            wager_amount.into(),
                            Some("wager win".to_string()),
                        )
                        .then(
                            Chess::ext(env::current_account_id())
                                .with_static_gas(WAGER_PAYOUT_CALLBACK_GAS)
                                .wager_victory_callback(token_id, winner_id, wager_amount),
                        )
                        .detach();
                }
                GameOutcome::Stalemate => {
                    let white_id = game.get_white().get_account_id().unwrap().clone();
                    let black_id = game.get_black().get_account_id().unwrap().clone();
                    ext_ft_core::ext(token_id.clone())
                        .with_attached_deposit(ONE_YOCTO)
                        .with_static_gas(FT_TRANSFER_GAS)
                        .ft_transfer(white_id.clone(), amount, Some("wager refund".to_string()))
                        .and(
                            ext_ft_core::ext(token_id.clone())
                                .with_attached_deposit(ONE_YOCTO)
                                .with_static_gas(FT_TRANSFER_GAS)
                                .ft_transfer(
                                    black_id.clone(),
                                    amount,
                                    Some("wager refund".to_string()),
                                ),
                        )
                        .then(
                            Chess::ext(env::current_account_id())
                                .with_static_gas(WAGER_PAYOUT_CALLBACK_GAS)
                                .wager_stalemate_callback(token_id, white_id, black_id, amount.0),
                        )
                        .detach();
                }
            }
        }

        if game.has_bets() {
            let players = (
                game.get_white().get_account_id().unwrap(),
                game.get_black().get_account_id().unwrap(),
            );
            let bet_id = BetId::new(players.clone()).unwrap();
            let all_bets = self.bets.remove(&bet_id).unwrap();
            let resolved_bettors: HashSet<_> = all_bets
                .bets
                .iter()
                .flat_map(|(_, bet_list)| bet_list.iter().map(|(id, _)| id.clone()))
                .collect();
            for account_id in &resolved_bettors {
                if let Some(active) = self.bettor_active_bets.get_mut(account_id) {
                    *active = active.saturating_sub(1);
                }
            }
            let fee_bps = *self.fees.get();
            match outcome {
                GameOutcome::Victory(color) => {
                    let winner_id = match color {
                        Color::White => game.get_white().get_account_id().unwrap(),
                        Color::Black => game.get_black().get_account_id().unwrap(),
                    };

                    let mut bet_winners: HashSet<AccountId> = HashSet::new();

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

                        for (account_id, bet) in bets.iter() {
                            if winner_id == bet.winner {
                                let mut win_amount = U128::from(total_looser)
                                    .full_mul(bet.amount.into())
                                    .div(total_winner)
                                    .as_u128();
                                win_amount = cmp::min(win_amount, bet.amount);
                                total_win_amount += win_amount;

                                let payout = win_amount + bet.amount;
                                self.accounts
                                    .get_mut(account_id)
                                    .unwrap()
                                    .add_token(token_id, payout);

                                if win_amount > 0 {
                                    bet_winners.insert(account_id.clone());
                                }
                            }
                        }

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

                    for account_id in &bet_winners {
                        let account = self.accounts.get_mut(account_id).unwrap();
                        account.record_bet_won();
                        let bets_won = account.get_bets_won();
                        if bets_won == 1 {
                            minted += account.apply_achievement(Achievement::FirstBetWin, true);
                        }
                        for (threshold, achievement) in
                            [(10, Achievement::BetsWon10), (100, Achievement::BetsWon100)]
                        {
                            if bets_won == threshold {
                                minted += account.apply_achievement(achievement, true);
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

            let event = ChessEvent::ResolveBets {
                players,
                game_id: game_id.clone(),
                outcome: outcome.clone(),
                fee_bps,
            };
            event.emit();
        }

        self.points_total_supply += minted;
    }

    pub(crate) fn internal_calculate_elo(&mut self, game: &Game, outcome: &GameOutcome) {
        let white_id = game.get_white().get_account_id();
        let black_id = game.get_black().get_account_id();

        let elo_white = white_id
            .as_ref()
            .and_then(|id| self.accounts.get(id).and_then(|a| a.get_elo()));
        let elo_black = black_id
            .as_ref()
            .and_then(|id| self.accounts.get(id).and_then(|a| a.get_elo()));

        if let (Some(elo_white), Some(elo_black), GameOutcome::Victory(color)) =
            (elo_white, elo_black, outcome)
        {
            let (new_elo_white, new_elo_black) = calculate_elo(
                elo_white,
                elo_black,
                match color {
                    Color::White => &EloOutcome::WIN,
                    Color::Black => &EloOutcome::LOSS,
                },
                &EloConfig::new(),
            );

            let mut minted: u128 = 0;
            let elo_thresholds: &[(f64, Achievement)] = &[
                (1100.0, Achievement::Elo1100),
                (1200.0, Achievement::Elo1200),
                (1300.0, Achievement::Elo1300),
                (1400.0, Achievement::Elo1400),
                (1500.0, Achievement::Elo1500),
            ];

            if let Some(ref wid) = white_id {
                let account = self.accounts.get_mut(wid).unwrap();
                account.set_elo(new_elo_white);
                for (threshold, achievement) in elo_thresholds {
                    if new_elo_white >= *threshold {
                        minted += account.apply_achievement(achievement.clone(), false);
                    }
                }
            }

            if let Some(ref bid) = black_id {
                let account = self.accounts.get_mut(bid).unwrap();
                account.set_elo(new_elo_black);
                for (threshold, achievement) in elo_thresholds {
                    if new_elo_black >= *threshold {
                        minted += account.apply_achievement(achievement.clone(), false);
                    }
                }
            }

            self.points_total_supply += minted;
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

    pub(crate) fn internal_register_account(&mut self, account_id: AccountId, amount: NearToken) {
        let account = Account::new(account_id.clone(), amount);
        self.accounts.insert(account_id, account);
    }

    fn deduct_fees(&mut self, token_id: &AccountId, amount: u128) -> u128 {
        let treasury_bps = *self.fees.get();
        let treasury_amount = U128::from(amount)
            .full_mul(treasury_bps.into())
            .div(10_000)
            .as_u128();
        if let Some(treasury) = self.treasury.get_mut(token_id) {
            *treasury += treasury_amount;
        } else {
            self.treasury.insert(token_id.clone(), treasury_amount);
        }
        treasury_amount
    }
}
