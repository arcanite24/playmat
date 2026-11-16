//! Titan's Presence: "As an additional cost to cast this spell, reveal a colorless creature card from your hand.
//! Exile target creature if its power is less than or equal to the revealed card's power."

use super::*;

pub const HANDLERS: &[Handler] = &[Handler::effect("TitansPresence.TitansPresenceEffect", apply)];

/// the revealed card's power against the target's: exiled if it's no greater
fn apply(game: &mut Game, _: &mut dyn Decider, cx: &EffectCx) -> Option<()> {
    let creature = cx.obj(0)?;
    let card = *cx.item.cost_revealed.first()?;
    if game.obj(card).def.power >= game.power_toughness(creature).0 {
        game.move_to(creature, Zone::Exile);
    }
    Some(())
}
