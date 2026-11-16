//! `CantHaveCountersAllEffect(filter, counterType)`: "Creatures your opponents control can't have +1/+1 counters put
//! on them." (Blightbeetle). A rule-modifying effect on `CAN_ADD_COUNTERS`: the counters aren't put at all, before any
//! `ADD_COUNTERS` replacement. (A `null` counter type, any counters, isn't read: it stays unsupported.)

use super::*;

pub const HANDLERS: &[Handler] = &[Handler::rule("CantHaveCountersAllEffect", Rule::CantHaveCounters(cant))
    .args(&["filter", "counter_type"])];

fn cant(game: &Game, cx: &StaticCx, id: ObjId, counter: &str) -> bool {
    cx.args.get(1).is_some_and(|c| c == counter) && cx.matches(game, 0, id)
}
