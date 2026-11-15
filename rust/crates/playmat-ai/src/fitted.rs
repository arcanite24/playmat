//! The fitted score (`Config::eval` 2): a logistic model of the chance of winning from a game's features, its
//! weights fitted to self-play results (`fit-eval`). Filled in by M4; until then it is Java's score.

use crate::eval;
use playmat_engine::state::{Game, Seat};

/// The seat's predicted chance of winning as a logit, times 1000
pub fn evaluate(game: &Game, seat: Seat) -> i64 {
    eval::evaluate(game, seat)
}

/// A fitted score as a chance of winning
pub fn probability(score: i64) -> f64 {
    1.0 / (1.0 + (-(score as f64) / 1000.0).exp())
}
