//! Teyo, Diamondblade Mage: "When Teyo enters, target permanent you control gains deathtouch until end of turn. Put a
//! +1/+1 counter on it if it's a creature. Put a loyalty counter on it if it's a planeswalker."

use super::*;

pub const HANDLERS: &[Handler] = &[Handler::effect("TeyoDiamondbladeMage.TeyoDiamondbladeMageEffect", apply)];

/// both checks on the permanent as it is, the +1/+1 counter first
fn apply(game: &mut Game, _d: &mut dyn Decider, cx: &EffectCx) -> Option<()> {
    let o = cx.obj(0)?;
    if game.obj(o).zone != Zone::Battlefield {
        return Some(());
    }
    let you = Some(cx.controller());
    if game.has_type(o, CardType::Creature) {
        add_counters_by(game, o, "+1/+1", 1, you);
    }
    if game.obj(o).zone == Zone::Battlefield && game.has_type(o, CardType::Planeswalker) {
        add_counters_by(game, o, "loyalty", 1, you);
    }
    Some(())
}
