//! Breaking // Entering, Entering: "Put a creature card from a graveyard onto the battlefield under your control. It
//! gains haste until end of turn."

use super::*;

pub const HANDLERS: &[Handler] =
    &[Handler::effect("BreakingEntering.EnteringReturnFromGraveyardToBattlefieldEffect", apply)];

/// A creature card from any graveyard (`TargetCardInGraveyard(FILTER_CARD_CREATURE)`, not targeted, one), chosen if
/// there is one; it enters under you and gains haste until end of turn
fn apply(game: &mut Game, d: &mut dyn Decider, cx: &EffectCx) -> Option<()> {
    let you = cx.controller();
    let cards: Vec<ObjId> = (0..game.seats())
        .flat_map(|p| game.player(p).graveyard.clone())
        .filter(|c| game.has_type(*c, CardType::Creature))
        .collect();
    if cards.is_empty() {
        return Some(());
    }
    let rule = item_rule(game, cx.item);
    let Some(card) = choose_cards(game, d, you, &cards, &cards, 1, 1, &rule)?.first().copied() else {
        return Some(());
    };
    if !put_cards(game, d, you, &[card], crate::card::PutCards::Battlefield) {
        return None;
    }
    if game.obj(card).zone == Zone::Battlefield {
        grant(game, card, &AbilityDef::HasteAbility);
    }
    Some(())
}
