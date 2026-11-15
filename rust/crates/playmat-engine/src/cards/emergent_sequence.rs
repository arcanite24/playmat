//! Emergent Sequence: "Search your library for a basic land card, put it onto the battlefield tapped, then shuffle.
//! That land becomes a 0/0 green and blue Fractal creature that's still a land. Put a +1/+1 counter on it for each
//! land you had enter the battlefield under your control this turn."

use super::*;

pub const HANDLERS: &[Handler] = &[Handler::effect("EmergentSequence.EmergentSequenceEffect", apply)
    .filters(&["StaticFilters.FILTER_CARD_BASIC_LAND_A"])
    .effects(&["new BecomesCreatureTargetEffect(new FractalToken(), false, true, Duration.Custom)"])];

/// The search, the land onto the battlefield tapped, the shuffle; then, if it's a permanent, it becomes the Fractal
/// for good (`FixedTarget`) and gets a +1/+1 counter for each land that entered under your control this turn (the
/// `EmergentSequenceWatcher`, this land counted)
fn apply(game: &mut Game, d: &mut dyn Decider, cx: &EffectCx) -> Option<()> {
    let you = cx.controller();
    let found = search_library(game, d, you, &cx.filters[0], 1, 1, cx.source_id())?;
    let card = found.first().copied();
    if let Some(c) = card {
        if game.obj(c).zone == Zone::Library
            && !put_cards(game, d, you, &[c], crate::card::PutCards::BattlefieldTapped)
        {
            return None;
        }
    }
    cx.run(game, d, &[EffectDef::ShuffleLibrarySourceEffect]);
    let Some(land) = card.filter(|c| game.obj(*c).zone == Zone::Battlefield) else { return Some(()) };
    cx.run_on(game, d, &cx.effects[..1], vec![Target::Obj(land)]);
    if d.stopped() {
        return None;
    }
    let amount = game.player(you).lands_entered as i32;
    if amount > 0 && game.obj(land).zone == Zone::Battlefield {
        put_counters(game, land, "+1/+1", amount, you);
    }
    Some(())
}
