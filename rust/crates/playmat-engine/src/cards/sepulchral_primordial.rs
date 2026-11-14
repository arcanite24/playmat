//! Sepulchral Primordial: "When Sepulchral Primordial enters, for each opponent, you may put up to one target
//! creature card from that player's graveyard onto the battlefield under your control."

use super::*;

pub const HANDLERS: &[Handler] = &[Handler::effect("SepulchralPrimordial.SepulchralPrimordialEffect", apply)];

/// each target group's first card still there (one per opponent, `ForEachPlayerTargetsAdjuster`) onto the
/// battlefield under your control, all at once
fn apply(game: &mut Game, d: &mut dyn Decider, cx: &EffectCx) -> Option<()> {
    let you = cx.controller();
    let cards: Vec<ObjId> = (0..cx.legal.len())
        .filter_map(|i| cx.obj(i))
        .filter(|c| game.obj(*c).zone == Zone::Graveyard)
        .collect();
    if cards.is_empty() {
        return Some(());
    }
    if !put_cards(game, d, you, &cards, crate::card::PutCards::Battlefield) {
        return None;
    }
    Some(())
}
