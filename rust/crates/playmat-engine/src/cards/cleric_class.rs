//! Cleric Class, level 3: "When this Class becomes level 3, return target creature card from your graveyard to the
//! battlefield. You gain life equal to that creature's toughness."

use super::*;

pub const HANDLERS: &[Handler] = &[Handler::effect("ClericClass.ClericClassReturnEffect", apply)];

/// the toughness of the permanent it became, or else the card's
fn apply(game: &mut Game, d: &mut dyn Decider, cx: &EffectCx) -> Option<()> {
    let you = cx.controller();
    let Some(card) = cx.obj(0) else { return Some(()) };
    if !put_cards(game, d, you, &[card], crate::card::PutCards::Battlefield) {
        return None;
    }
    let toughness =
        if game.obj(card).zone == Zone::Battlefield { game.power_toughness(card).1 } else { game.obj(card).def.toughness };
    gain_life(game, you, toughness);
    Some(())
}
