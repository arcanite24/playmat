//! Lazav, Familiar Stranger: "Whenever you commit a crime, put a +1/+1 counter on Lazav, Familiar Stranger. Then you
//! may exile a card from a graveyard. If a creature card was exiled this way, you may have Lazav become a copy of
//! that card until end of turn. This ability triggers only once each turn."

use super::*;

pub const HANDLERS: &[Handler] = &[Handler::effect("LazavFamiliarStranger.LazavFamiliarStrangerEffect", apply)];

/// up to one card from any graveyard (`player.choose` of a not-target `TargetCardInGraveyard(0, 1)`), exiled; a
/// creature card, with Lazav still on the battlefield, asks whether Lazav becomes a copy of it until end of turn
/// (`game.copyPermanent(Duration.EndOfTurn, new PermanentCard(card), lazav)`)
fn apply(game: &mut Game, d: &mut dyn Decider, cx: &EffectCx) -> Option<()> {
    let you = cx.controller();
    let possible: Vec<ObjId> = game.players.iter().flat_map(|p| p.graveyard.iter().copied()).collect();
    let chosen = super::gilgamesh_master_at_arms::choose_any(game, d, you, &possible, 1)?;
    let Some(&card) = chosen.first() else { return Some(()) };
    let creature = game.obj(card).is(CardType::Creature);
    if !put_cards(game, d, you, &[card], crate::card::PutCards::Exiled) {
        return None;
    }
    if !creature || !cx.source_on_battlefield(game) {
        return Some(());
    }
    let message = format!(
        "Do you want {} to become a copy of {} until end of turn?",
        log::object(game, cx.source_id()),
        log::object(game, card)
    );
    if !choose_use(game, d, you, &message)? {
        return Some(());
    }
    let copy = game.obj(card).copiable();
    become_copy_until_end_of_turn(game, cx.source_id(), copy);
    Some(())
}
