//! Underworld Sentinel: "Whenever Underworld Sentinel attacks, exile target creature card from your graveyard. When
//! Underworld Sentinel dies, put all cards exiled with it onto the battlefield."

use super::*;

pub const HANDLERS: &[Handler] = &[Handler::effect("UnderworldSentinel.UnderworldSentinelEffect", apply)];

/// The cards still in its exile zone (`ExileTargetForSourceEffect`'s, from the permanent this card was: a dies
/// trigger has the graveyard card's zone change counter, one more), in the order they were exiled, put onto the
/// battlefield under your control together
fn apply(game: &mut Game, d: &mut dyn Decider, cx: &EffectCx) -> Option<()> {
    let (src, zcc) = cx.source;
    let mut cards: Vec<ObjId> = Vec::new();
    for (s, z, c) in &game.exiled_for_source {
        if *s == src && (*z == zcc || *z + 1 == zcc) && game.obj(*c).zone == Zone::Exile && !cards.contains(c) {
            cards.push(*c);
        }
    }
    if cards.is_empty() {
        return Some(());
    }
    put_cards(game, d, cx.controller(), &cards, crate::card::PutCards::Battlefield).then_some(())
}
