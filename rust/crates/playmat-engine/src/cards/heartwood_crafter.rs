//! Heartwood Crafter: "{T}: Add {C}. This mana can't be spent to cast spells from your hand."

use super::*;
use crate::state::Zone;

pub const HANDLERS: &[Handler] = &[Handler::spend("HeartwoodCrafter.HeartwoodCrafterManaBuilder", spend)];

fn spend(game: &Game, cx: &SpendCx) -> bool {
    if !cx.spell() {
        return true;
    }
    let o = game.obj(cx.on);
    // a card you don't own can't be cast from your hand
    if o.owner != cx.seat {
        return true;
    }
    if o.zone == Zone::Stack { o.cast_from != Some(Zone::Hand) } else { o.zone != Zone::Hand }
}
