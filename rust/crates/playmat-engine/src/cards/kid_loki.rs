//! Kid Loki: "Each creature you control that you've put one or more +1/+1 counters on this turn has hexproof."

use super::*;

pub const HANDLERS: &[Handler] = &[Handler::predicate("KidLoki.KidLokiPredicate", you_put_a_counter)];

/// `KidLokiWatcher.checkPermanent`: you put a +1/+1 counter on it this turn, as it is now
fn you_put_a_counter(game: &Game, you: Seat, _: Option<ObjId>, id: ObjId) -> bool {
    game.p1p1_put_this_turn.contains(&(you, id, game.obj(id).zcc))
}
