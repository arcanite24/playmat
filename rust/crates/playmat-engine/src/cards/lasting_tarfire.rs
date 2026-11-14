//! Lasting Tarfire: "At the beginning of each end step, if you put a counter on a creature this turn, Lasting
//! Tarfire deals 2 damage to each opponent."

use super::*;

pub const HANDLERS: &[Handler] = &[Handler::condition("LastingTarfire.LastingTarfireCondition", you_put_a_counter)];

/// `LastingTarfireWatcher`: a `COUNTER_ADDED` of yours on a creature this turn
fn you_put_a_counter(game: &Game, you: Seat, _: ObjId) -> bool {
    game.watch.counters_put.iter().any(|(by, _, _, _, n, creature)| *by == Some(you) && *n > 0 && *creature)
}
