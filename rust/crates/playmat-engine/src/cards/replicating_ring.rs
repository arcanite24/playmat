//! Replicating Ring: "At the beginning of your upkeep, put a night counter on Replicating Ring. Then if it has eight
//! or more night counters on it, remove all of them and create eight colorless snow artifact tokens named Replicated
//! Ring with "{T}: Add one mana of any color.""

use super::*;

pub const HANDLERS: &[Handler] =
    &[Handler::effect("ReplicatingRing.ReplicatingRingEffect", apply).effects(&["new CreateTokenEffect(new ReplicatedRingToken(), 8)"])];

/// The counter goes on only while the Ring is still there; the count is the permanent's, as it is or last was
fn apply(game: &mut Game, d: &mut dyn Decider, cx: &EffectCx) -> Option<()> {
    let ring = cx.source_id();
    if cx.source_on_battlefield(game) {
        add_counters_by(game, ring, "night", 1, Some(cx.controller()));
    }
    if game.obj(ring).counter("night") < 8 {
        return Some(());
    }
    game.obj_mut(ring).counters.retain(|(n, _)| n != "night");
    cx.run(game, d, &cx.effects[..1]);
    Some(())
}
