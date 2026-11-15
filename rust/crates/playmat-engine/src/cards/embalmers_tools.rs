//! Embalmer's Tools: "Activated abilities of creature cards in your graveyard cost {1} less to activate. Tap an
//! untapped Zombie you control: Target player mills a card."

use super::*;
use crate::card::ManaCost;

pub const HANDLERS: &[Handler] =
    &[Handler::cost("EmbalmersTools.EmbalmersToolsEffect", CostKind::Reduce, creature_card_in_your_graveyard, one_less)];

/// an activated ability of a creature card you own in a graveyard
fn creature_card_in_your_graveyard(game: &Game, cx: &StaticCx, of: &CostCx) -> bool {
    let o = game.obj(of.card);
    of.ability.is_some() && o.zone == Zone::Graveyard && o.owner == cx.controller && o.is(CardType::Creature)
}

/// `CardUtil.reduceCost(ability, 1)`
fn one_less(_: &Game, _: &StaticCx, _: &CostCx, cost: &mut ManaCost) {
    cost.generic = cost.generic.saturating_sub(1);
}
