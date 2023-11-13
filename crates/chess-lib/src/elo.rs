//! Elo algorithm implementation taken from <https://github.com/atomflunder/skillratings/>
//!
//! MIT License
//!
//! Copyright (c) 2022 atomflunder
//!
//! Permission is hereby granted, free of charge, to any person obtaining a copy
//! of this software and associated documentation files (the "Software"), to deal
//! in the Software without restriction, including without limitation the rights
//! to use, copy, modify, merge, publish, distribute, sublicense, and/or sell
//! copies of the Software, and to permit persons to whom the Software is
//! furnished to do so, subject to the following conditions:
//!
//! The above copyright notice and this permission notice shall be included in all
//! copies or substantial portions of the Software.
//!
//! THE SOFTWARE IS PROVIDED "AS IS", WITHOUT WARRANTY OF ANY KIND, EXPRESS OR
//! IMPLIED, INCLUDING BUT NOT LIMITED TO THE WARRANTIES OF MERCHANTABILITY,
//! FITNESS FOR A PARTICULAR PURPOSE AND NONINFRINGEMENT. IN NO EVENT SHALL THE
//! AUTHORS OR COPYRIGHT HOLDERS BE LIABLE FOR ANY CLAIM, DAMAGES OR OTHER
//! LIABILITY, WHETHER IN AN ACTION OF CONTRACT, TORT OR OTHERWISE, ARISING FROM,
//! OUT OF OR IN CONNECTION WITH THE SOFTWARE OR THE USE OR OTHER DEALINGS IN THE
//! SOFTWARE.

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum EloOutcome {
    /// A win, from player_one's perspective.
    WIN,
    /// A loss, from player_one's perspective.
    LOSS,
    /// A draw.
    DRAW,
}

impl EloOutcome {
    #[must_use]
    /// Converts the outcome of the match into the points used in chess (1 = Win, 0.5 = Draw, 0 = Loss).
    ///
    /// Used internally in several rating algorithms, but some, like TrueSkill, have their own conversion.
    pub const fn to_chess_points(self) -> f64 {
        // Could set the visibility to crate level, but maybe someone has a use for it, who knows.
        match self {
            Self::WIN => 1.0,
            Self::DRAW => 0.5,
            Self::LOSS => 0.0,
        }
    }
}

/// The Elo rating of a player.
///
/// The default rating is 1000.0.
pub type EloRating = f64;

#[derive(Clone, Copy, Debug)]
/// Constants used in the Elo calculations.
pub struct EloConfig {
    /// The k-value is the maximum amount of rating change from a single match.
    /// In chess, k-values from 40 to 10 are used, with the most common being 32, 24, 16 or 10.
    /// The higher the number, the more volatile the ranking.  
    /// Here the default is 32.
    pub k: f64,
}

impl EloConfig {
    #[must_use]
    /// Initialise a new `EloConfig` with a k value of `32.0`.
    pub const fn new() -> Self {
        Self { k: 32.0 }
    }
}

#[must_use]
pub fn calculate_elo(
    player_one: EloRating,
    player_two: EloRating,
    outcome: &EloOutcome,
    config: &EloConfig,
) -> (EloRating, EloRating) {
    let (one_expected, two_expected) = expected_score(player_one, player_two);

    let outcome1 = outcome.to_chess_points();
    let outcome2 = 1.0 - outcome1;

    let one_new_elo = config.k.mul_add(outcome1 - one_expected, player_one);
    let two_new_elo = config.k.mul_add(outcome2 - two_expected, player_two);

    (one_new_elo, two_new_elo)
}

#[must_use]
pub fn expected_score(player_one: EloRating, player_two: EloRating) -> (f64, f64) {
    let exp_one = (1.0 + 10_f64.powf((player_two - player_one) / 400.0)).recip();
    let exp_two = 1.0 - exp_one;

    (exp_one, exp_two)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_elo() {
        let (winner_new_elo, loser_new_elo) =
            calculate_elo(1_000., 1_000., &EloOutcome::WIN, &EloConfig::new());
        assert!((winner_new_elo - 1016.0).abs() < f64::EPSILON);
        assert!((loser_new_elo - 984.0).abs() < f64::EPSILON);

        let (winner_new_elo, loser_new_elo) =
            calculate_elo(1_000., 1_000., &EloOutcome::LOSS, &EloConfig::new());
        assert!((winner_new_elo - 984.0).abs() < f64::EPSILON);
        assert!((loser_new_elo - 1016.0).abs() < f64::EPSILON);

        let (winner_new_elo, loser_new_elo) =
            calculate_elo(1_000., 1_000., &EloOutcome::DRAW, &EloConfig::new());
        assert!((winner_new_elo - 1000.0).abs() < f64::EPSILON);
        assert!((loser_new_elo - 1000.0).abs() < f64::EPSILON);

        let (winner_new_elo, loser_new_elo) =
            calculate_elo(500., 1_500., &EloOutcome::WIN, &EloConfig::new());
        assert!((winner_new_elo.round() - 532.0).abs() < f64::EPSILON);
        assert!((loser_new_elo.round() - 1468.0).abs() < f64::EPSILON);
    }

    #[test]
    fn test_expected_score() {
        let player_one = 1_000.;
        let player_two = 1_000.;

        let (winner_expected, loser_expected) = expected_score(player_one, player_two);

        assert!((winner_expected - 0.5).abs() < f64::EPSILON);
        assert!((loser_expected - 0.5).abs() < f64::EPSILON);

        let player_one = 2251.;
        let player_two = 1934.;

        let (winner_expected, loser_expected) = expected_score(player_one, player_two);

        assert!(((winner_expected * 100.0).round() - 86.0).abs() < f64::EPSILON);
        assert!(((loser_expected * 100.0).round() - 14.0).abs() < f64::EPSILON);

        assert!((winner_expected + loser_expected - 1.0).abs() < f64::EPSILON);
    }

    #[test]
    fn test_misc_stuff() {
        let player_one = 1_000.;
        let config = EloConfig::new();

        assert_eq!(player_one, player_one.clone());
        assert!((config.k - config.k).abs() < f64::EPSILON);

        assert!(!format!("{player_one:?}").is_empty());
        assert!(!format!("{config:?}").is_empty());

        assert_eq!(player_one, 1000.);
    }
}
