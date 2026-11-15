//! Memory Theft: "Target opponent reveals their hand. You choose a nonland card from it. That player discards that
//! card. You may put a card that has an Adventure that player owns from exile into that player's graveyard."

use super::*;

pub const HANDLERS: &[Handler] = &[Handler::effect("MemoryTheft.MemoryTheftEffect", apply)
    .effects(&["new DiscardCardYouChooseTargetEffect(StaticFilters.FILTER_CARD_NON_LAND)"])];

/// the discard (`DiscardCardYouChooseTargetEffect`), then up to one exiled card with an Adventure the opponent owns,
/// asked even with none there (a target of zero to one can always be chosen), into their graveyard
fn apply(game: &mut Game, d: &mut dyn Decider, cx: &EffectCx) -> Option<()> {
    cx.run(game, d, &cx.effects[..1]);
    if d.stopped() {
        return None;
    }
    let you = cx.controller();
    let Some(p) = cx.player(0) else { return Some(()) };
    let options: Vec<ObjId> = game
        .exile
        .iter()
        .copied()
        .filter(|c| {
            let o = game.obj(*c);
            o.owner == p && o.def.adventure.is_some()
        })
        .collect();
    // `TargetCardInExile(0, 1, …)` can always be chosen: asked even with nothing to pick
    let rule = item_rule(game, cx.item);
    if let Some(card) = choose_cards(game, d, you, &options, &options, 0, 1, &rule)?.first().copied() {
        game.move_to(card, Zone::Graveyard);
    }
    Some(())
}
