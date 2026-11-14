//! Beast, Erudite Aerialist: "As long as you've put one or more +1/+1 counters on Beast this turn, he has flying.
//! Whenever Beast deals combat damage to a player, draw a card."

use super::*;

pub const HANDLERS: &[Handler] =
    &[Handler::condition("BeastEruditeAerialist.BeastEruditeAerialistCondition", you_put_a_counter)];

/// `BeastEruditeAerialistWatcher.checkPermanent`: the source is on the battlefield and you put a +1/+1 counter on
/// it this turn, as it is now
fn you_put_a_counter(game: &Game, you: Seat, source: ObjId) -> bool {
    let o = game.obj(source);
    o.zone == Zone::Battlefield && game.p1p1_put_this_turn.contains(&(you, source, o.zcc))
}
