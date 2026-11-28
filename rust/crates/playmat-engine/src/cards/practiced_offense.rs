//! Practiced Offense and Requisition Raid: "Put a +1/+1 counter on each creature target player controls."

use super::*;

pub const HANDLERS: &[Handler] = &[
    Handler::effect("PracticedOffense.PracticedOffenseEffect", apply),
    Handler::effect("RequisitionRaid.RequisitionRaidEffect", apply),
];

fn apply(game: &mut Game, _d: &mut dyn Decider, cx: &EffectCx) -> Option<()> {
    let Some(p) = cx.player(0) else { return Some(()) };
    let creatures: Vec<ObjId> = game
        .battlefield
        .iter()
        .copied()
        .filter(|b| game.obj(*b).controller == p && game.has_type(*b, CardType::Creature))
        .collect();
    for c in creatures {
        add_counters_by(game, c, "+1/+1", 1, Some(cx.controller()));
    }
    Some(())
}
