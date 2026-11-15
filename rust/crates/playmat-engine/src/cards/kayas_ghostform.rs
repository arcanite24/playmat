//! Kaya's Ghostform: "When enchanted permanent dies or is put into exile, return that card to the battlefield under
//! your control."

use super::*;

pub const HANDLERS: &[Handler] = &[
    Handler::trigger(
        "KayasGhostform.KayasGhostformTriggeredAbility",
        "new LeavesBattlefieldAllTriggeredAbility(new KayasGhostformReturnEffect(), StaticFilters.FILTER_PERMANENT)",
        enchanted_dies_or_exiled,
    ),
    // the trigger's `ReturnToBattlefieldUnderYourControlTargetEffect` at the card that left (`FixedTargets`)
    Handler::effect("KayasGhostform.KayasGhostformReturnEffect", apply),
];

/// the permanent that left had this Aura attached as it left, and went to a graveyard or exile
fn enchanted_dies_or_exiled(game: &Game, item: &StackItem) -> bool {
    let Some(left) = item.left else { return false };
    matches!(game.obj(left).zone, Zone::Graveyard | Zone::Exile)
        && game.obj(left).lki_attachments.iter().any(|(a, _)| *a == item.card)
}

/// the card, still in that graveyard or exile, onto the battlefield under the Aura's controller's control
fn apply(game: &mut Game, d: &mut dyn Decider, cx: &EffectCx) -> Option<()> {
    let Some(card) = cx.item.left else { return Some(()) };
    if !matches!(game.obj(card).zone, Zone::Graveyard | Zone::Exile) || game.obj(card).token {
        return Some(());
    }
    // a mutated permanent's merged cards come back with it, together (`CardUtil.getAllCardsFromPermanentLeftBattlefield`)
    let merged: Vec<ObjId> = game
        .obj(card)
        .merged
        .iter()
        .copied()
        .filter(|c| game.obj(*c).zone == game.obj(card).zone && !game.obj(*c).token)
        .collect();
    if merged.is_empty() {
        put_onto_battlefield(game, d, card, cx.controller());
    } else {
        let mut cards = vec![card];
        cards.extend(merged);
        put_cards(game, d, cx.controller(), &cards, crate::card::PutCards::Battlefield);
    }
    if d.stopped() {
        return None;
    }
    Some(())
}
