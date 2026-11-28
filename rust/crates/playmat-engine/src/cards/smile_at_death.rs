//! Smile at Death: "At the beginning of your upkeep, return up to two target creature cards with power 2 or less
//! from your graveyard to the battlefield. Put a +1/+1 counter on each of those creatures."

use super::*;

pub const HANDLERS: &[Handler] = &[Handler::effect("SmileAtDeath.SmileAtDeathEffect", apply)];

/// The targets still in the graveyard onto the battlefield together, then a +1/+1 counter on each that's there
fn apply(game: &mut Game, d: &mut dyn Decider, cx: &EffectCx) -> Option<()> {
    let you = cx.controller();
    let cards: Vec<ObjId> = cx.objs(0).into_iter().filter(|c| game.obj(*c).zone == Zone::Graveyard).collect();
    if cards.is_empty() {
        return Some(());
    }
    if !put_cards(game, d, you, &cards, crate::card::PutCards::Battlefield) {
        return None;
    }
    for c in cards {
        if game.obj(c).zone == Zone::Battlefield {
            put_counters(game, c, "+1/+1", 1, you);
        }
    }
    Some(())
}
