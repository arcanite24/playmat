//! Recommission: "Return target artifact or creature card with mana value 3 or less from your graveyard to the
//! battlefield. If a creature enters this way, it enters with an additional +1/+1 counter on it."

use super::*;

pub const HANDLERS: &[Handler] = &[Handler::effect("Recommission.RecommissionEffect", apply)];

/// The target card, if still legal, onto the battlefield under you; a creature it became gets the +1/+1 counter
/// (`RecommissionCounterEffect`, put by you)
fn apply(game: &mut Game, d: &mut dyn Decider, cx: &EffectCx) -> Option<()> {
    let Some(card) = cx.obj(0) else { return Some(()) };
    let you = cx.controller();
    if !put_cards(game, d, you, &[card], crate::card::PutCards::Battlefield) {
        return None;
    }
    if game.obj(card).zone == Zone::Battlefield && game.has_type(card, CardType::Creature) {
        put_counters(game, card, "+1/+1", 1, you);
    }
    Some(())
}
