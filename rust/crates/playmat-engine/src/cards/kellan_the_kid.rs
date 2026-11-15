//! Kellan, the Kid: "Flying, lifelink. Whenever you cast a spell from anywhere other than your hand, you may cast a
//! permanent spell with equal or lesser mana value from your hand without paying its mana cost. If you don't, you
//! may put a land card from your hand onto the battlefield."

use super::*;
use crate::card::{Cmp, Pred};

pub const HANDLERS: &[Handler] = &[Handler::effect("KellanTheKid.KellanTheKidEffect", apply)
    .filters(&["new FilterPermanentCard()"])
    .effects(&["new PutCardFromHandOntoBattlefieldEffect(StaticFilters.FILTER_CARD_LAND)"])];

/// `castSpellWithAttributesForFree` over the hand with a permanent card of mana value below the spell's plus one
/// (the spell's mana value is the trigger's X); with nothing cast, `PutCardFromHandOntoBattlefieldEffect` for a land
fn apply(game: &mut Game, d: &mut dyn Decider, cx: &EffectCx) -> Option<()> {
    let you = cx.controller();
    cx.item.cast_spell?;
    let mut filter = cx.filters.first()?.clone();
    filter.0.push(Pred::Mv { cmp: Cmp::FewerThan, n: cx.item.x + 1 });
    let hand = game.player(you).hand.clone();
    if !cast_one_for_free(game, d, you, cx.source_id(), &filter) {
        return None;
    }
    if hand.iter().any(|c| game.obj(*c).zone == Zone::Stack) {
        return Some(());
    }
    cx.run(game, d, &cx.effects[..1]);
    Some(())
}
