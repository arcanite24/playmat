//! Mm'menon, the Right Hand: "Flying. You may look at the top card of your library any time. You may cast artifact
//! spells from the top of your library. Artifacts you control have "{T}: Add {U}. Spend this mana only to cast a
//! spell from anywhere other than your hand.""

use super::*;

pub const HANDLERS: &[Handler] = &[Handler::spend("MmmenonTheRightHand.MmmenonTheRightHandManaBuilder", spend)];

/// `MmmenonTheRightHandManaCondition`: a spell its owner casts, from anywhere but their hand (a spell on the stack by
/// the zone it was cast from, a card checked as castable by the zone it's in)
fn spend(game: &Game, cx: &SpendCx) -> bool {
    if !cx.spell() {
        return false;
    }
    let o = game.obj(cx.on);
    if o.owner != cx.seat {
        return false;
    }
    let from = if o.zone == Zone::Stack { o.cast_from } else { Some(o.zone) };
    from != Some(Zone::Hand)
}
