//! Littjara Kinseekers: "When this creature enters, if you control three or more creatures that share a creature
//! type, put a +1/+1 counter on it, then scry 1."

use super::*;

pub const HANDLERS: &[Handler] =
    &[Handler::condition("LittjaraKinseekers.LittjaraKinseekersCondition", three_share)];

/// `GreatestSharedCreatureTypeCount` is 3 or more
fn three_share(game: &Game, you: Seat, source: ObjId) -> bool {
    super::greatest_shared_creature_type_count::value(game, you, source, 0) >= 3
}
