//! Sunfall: "Exile all creatures. Incubate X, where X is the number of creatures exiled this way."

use super::*;

pub const HANDLERS: &[Handler] = &[Handler::effect("Sunfall.SunfallEffect", apply)
    .filters(&["StaticFilters.FILTER_PERMANENT_CREATURE"])
    .effects(&["new IncubateEffect(0)"])];

/// every creature to exile, then incubate the number of them (Java's `cards.size()`)
fn apply(game: &mut Game, d: &mut dyn Decider, cx: &EffectCx) -> Option<()> {
    let creatures = matching(game, &cx.filters[0], cx.controller(), cx.source_id());
    for o in &creatures {
        game.move_to(*o, Zone::Exile);
    }
    let mut incubate = cx.effects[0].clone();
    if let EffectDef::IncubateEffect { amount, .. } = &mut incubate {
        *amount = creatures.len() as i32;
    }
    cx.run(game, d, &[incubate]);
    Some(())
}
