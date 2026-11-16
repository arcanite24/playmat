//! Charge of the Forever-Beast: "As an additional cost to cast this spell, reveal a creature card from your hand.
//! Charge of the Forever-Beast deals damage to target creature or planeswalker equal to the revealed card's power."

use super::*;

pub const HANDLERS: &[Handler] =
    &[Handler::effect("ChargeOfTheForeverBeast.ChargeOfTheForeverBeastEffect", apply)];

/// the revealed card's power (`getRevealedCards().get(0).getPower()`), as damage to the target
fn apply(game: &mut Game, _: &mut dyn Decider, cx: &EffectCx) -> Option<()> {
    let Some(o) = cx.obj(0) else { return Some(()) };
    let Some(card) = cx.item.cost_revealed.first() else { return Some(()) };
    let power = game.obj(*card).def.power;
    deal_damage(game, cx.source_id(), Target::Obj(o), power, false);
    Some(())
}
