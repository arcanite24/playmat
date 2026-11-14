//! Scourge of Skola Vale: "Trample. Scourge of Skola Vale enters with two +1/+1 counters on it. {T}, Sacrifice
//! another creature: Put a number of +1/+1 counters on Scourge of Skola Vale equal to the sacrificed creature's
//! toughness."

use super::*;

pub const HANDLERS: &[Handler] = &[Handler::effect("ScourgeOfSkolaVale.ScourgeOfSkolaValeEffect", apply)];

/// The sacrificed creature's toughness as it was sacrificed (the cost's one creature); above 0, that many counters
/// on the source (`AddCountersSourceEffect`: still the same permanent)
fn apply(game: &mut Game, _d: &mut dyn Decider, cx: &EffectCx) -> Option<()> {
    let n = cx.item.sacrificed_stats[1];
    if n > 0 && cx.source_on_battlefield(game) {
        add_counters_by(game, cx.source_id(), "+1/+1", n, Some(cx.controller()));
    }
    Some(())
}
