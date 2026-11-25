//! Toluz, Clever Conductor: "When Toluz, Clever Conductor enters the battlefield, it connives. Whenever you discard
//! one or more cards, exile them from your graveyard. When Toluz dies, put the cards exiled with it into their owner's
//! hand."

use super::*;

pub const HANDLERS: &[Handler] = &[
    Handler::trigger(
        "ToluzCleverConductor.ToluzCleverConductorTriggeredAbility",
        "new DiscardOneOrMoreCardsTriggeredAbility(new ToluzCleverConductorExileThem())",
        any_batch,
    ),
    Handler::effect("ToluzCleverConductor.ToluzCleverConductorExileThem", exile_them),
    Handler::effect("ToluzCleverConductor.ToluzCleverConductorEffect", return_them),
];

/// `ExileTargetForSourceEffect` on `FixedTargets` of the discarded cards that were in the graveyard as it triggered:
/// the ones still there, as they were
fn exile_them(game: &mut Game, d: &mut dyn Decider, cx: &EffectCx) -> Option<()> {
    let them: Vec<Target> = cx
        .item
        .batch
        .iter()
        .filter(|(o, zcc, _)| game.obj(*o).zone == Zone::Graveyard && game.obj(*o).zcc == *zcc)
        .map(|(o, _, _)| Target::Obj(*o))
        .collect();
    if !them.is_empty() {
        cx.run_on(game, d, &[EffectDef::ExileTargetForSourceEffect], them);
    }
    Some(())
}

/// The cards in Toluz's exile zone as the permanent it was (the dies trigger's source is the graveyard card, one zone
/// change on) move to their owners' hands, in the order they were exiled
fn return_them(game: &mut Game, _d: &mut dyn Decider, cx: &EffectCx) -> Option<()> {
    let (src, zcc) = cx.source;
    let cards: Vec<ObjId> = game
        .exiled_for_source
        .iter()
        .filter(|(s, z, c)| *s == src && (*z == zcc || *z + 1 == zcc) && game.obj(*c).zone == Zone::Exile)
        .map(|(_, _, c)| *c)
        .collect();
    for c in cards {
        game.move_to(c, Zone::Hand);
    }
    Some(())
}
