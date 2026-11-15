//! Ninja's Blades: "Job select. Equipped creature gets +1/+1, is a Ninja in addition to its other types, and has
//! "Whenever this creature deals combat damage to a player, draw a card, then discard a card. That player loses life
//! equal to the discarded card's mana value." Mutsunokami — Equip {2}"

use super::*;

pub const HANDLERS: &[Handler] = &[Handler::effect("NinjasBlades.NinjasBladesEffect", apply)];

/// A card drawn, then `discardOne`; a discarded card of mana value 1 or more: that much life lost by the player
/// dealt damage (the pointer)
fn apply(game: &mut Game, d: &mut dyn Decider, cx: &EffectCx) -> Option<()> {
    let you = cx.controller();
    draw(game, you, 1);
    let discarded = discard(game, d, you, 1);
    if d.stopped() {
        return None;
    }
    let Some(&card) = discarded.first() else { return Some(()) };
    let mv = game.obj(card).def.mana_value() as i32;
    if mv < 1 {
        return Some(());
    }
    if let Some(p) = cx.player(0) {
        lose_life_by(game, p, mv, Some(cx.source_id()), false);
    }
    Some(())
}
