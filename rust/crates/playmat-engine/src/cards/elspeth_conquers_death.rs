//! Elspeth Conquers Death: "I — Exile target permanent an opponent controls with mana value 3 or greater. II —
//! Noncreature spells your opponents cast cost {2} more to cast until your next turn. III — Return target creature or
//! planeswalker card from your graveyard to the battlefield. Put a +1/+1 counter or a loyalty counter on it."

use super::*;
use crate::card::{CardType, ManaCost};

pub const HANDLERS: &[Handler] = &[
    Handler::cost("ElspethConquersDeath.ElspethConquersDeathCostEffect", CostKind::Increase, applies, increase)
        .until_your_next_turn(),
    Handler::effect("ElspethConquersDeath.ElspethConquersDeathReturnEffect", return_it),
];

/// A spell an opponent casts whose characteristics (the face cast) aren't a creature's
fn applies(_: &Game, cx: &StaticCx, of: &CostCx) -> bool {
    of.ability.is_none() && cx.opponent(of.seat) && !of.face.is(CardType::Creature)
}

/// `CardUtil.adjustCost(spellAbility, -2)`: {2} more
fn increase(_: &Game, _: &StaticCx, _: &CostCx, cost: &mut ManaCost) {
    cost.generic += 2;
}

/// The card onto the battlefield; if it's there, "Choose a type of counter to add" (+1/+1, or Loyalty) and one of it
fn return_it(game: &mut Game, d: &mut dyn Decider, cx: &EffectCx) -> Option<()> {
    let you = cx.controller();
    let Some(card) = cx.obj(0) else { return Some(()) };
    if !put_cards(game, d, you, &[card], crate::card::PutCards::Battlefield) {
        return None;
    }
    if game.obj(card).zone != Zone::Battlefield {
        return Some(());
    }
    let p1p1 = choose_use(game, d, you, "Choose a type of counter to add")?;
    add_counters_by(game, card, if p1p1 { "+1/+1" } else { "loyalty" }, 1, Some(you));
    Some(())
}
